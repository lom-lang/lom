# Lom Eval Runner (PowerShell)
#
# Usage:
#   ./run.ps1 -Verify                        # Verify reference solutions (smoke test eval set)
#   ./run.ps1 -CandidatesDir eval/candidates # Evaluate LLM-generated candidates
#   ./run.ps1 -Verify -Verbose               # Show per-task detail
#   ./run.ps1 -Help
#
# Requirements:
#   - lom.exe on PATH (run `cargo build` first)
#   - PowerShell 5.1+ (Windows native, no extra deps)

param(
    [switch]$Verify,
    [string]$CandidatesDir,
    [switch]$Verbose,
    [switch]$Help,
    [string]$LomBin = "lom",
    [string]$EvalDir = (Split-Path -Parent $PSScriptRoot),
    # Phase 7.9：后端选择——interp（默认，树遍历解释器）/ wasm（编译后经 node run_wasm.mjs 运行）
    [string]$Backend = "interp",
    [string]$NodeBin = "node"
)

if ($Help -or (-not $Verify -and -not $CandidatesDir)) {
    Write-Host "Lom Eval Runner (PowerShell)"
    Write-Host ""
    Write-Host "Usage:"
    Write-Host "  ./run.ps1 -Verify                        Verify reference solutions"
    Write-Host "  ./run.ps1 -CandidatesDir <dir>           Evaluate LLM candidates in <dir>"
    Write-Host "  ./run.ps1 -Verify -Verbose               Show per-task detail"
    Write-Host "  ./run.ps1 -Help                          This help"
    Write-Host ""
    Write-Host "Requirements:"
    Write-Host "  - Build lom first:  cargo build"
    Write-Host "  - lom.exe on PATH (or use -LomBin <path>)"
    exit 0
}

# Use Continue (not Stop): external commands like lom write to stderr, which
# PowerShell would otherwise elevate to a terminating error under Stop mode.
$ErrorActionPreference = "Continue"
$tasksDir = Join-Path $EvalDir "tasks"

if (-not (Test-Path $tasksDir)) {
    Write-Error "tasks dir not found: $tasksDir"
    exit 1
}

# Verify lom is callable. lom --help writes to stderr (PowerShell treats stderr
# as error stream); check exit code instead of relying on try/catch.
& $LomBin --help 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Error "Cannot run lom binary at: $LomBin. Build it first: cargo build"
    exit 1
}

# Collect all task files
$taskFiles = Get-ChildItem -Path $tasksDir -Filter "*.json" | Sort-Object Name
if ($taskFiles.Count -eq 0) {
    Write-Error "No task files in $tasksDir"
    exit 1
}

# Stats
$stats = [ordered]@{
    total     = 0
    passed    = 0
    failed    = 0
    byCategory = @{}
}

function Test-OneTask($task, $src) {
    # Write source to temp file
    $tmp = New-TemporaryFile
    $tmpPath = $tmp.FullName + ".lom"
    Rename-Item $tmp $tmpPath
    try {
        [System.IO.File]::WriteAllText($tmpPath, $src)
        # 只比对 stdout：stderr 是诊断通道（2026-08-22 起运行时类型检查 warning 走 stderr）
        # 同时要求进程退出码 0——"先打印正确输出再崩溃"不能算通过
        if ($Backend -eq "wasm") {
            # Phase 7.9：wasm 后端——先编译再经 node harness 运行
            $wasmPath = $tmpPath -replace '\.lom$', '.wasm'
            & $LomBin build $tmpPath --target wasm -o $wasmPath 2>$null | Out-Null
            if ($LASTEXITCODE -ne 0) {
                return [pscustomobject]@{ Pass = $false; Expected = $task.expected; Actual = "<wasm 编译失败>" }
            }
            $harness = Join-Path $PSScriptRoot "run_wasm.mjs"
            $actual = & $NodeBin $harness $wasmPath 2>$null | Out-String
            $exitCode = $LASTEXITCODE
            Remove-Item $wasmPath -Force -ErrorAction SilentlyContinue
        } else {
            $actual = & $LomBin $tmpPath 2>$null | Out-String
            $exitCode = $LASTEXITCODE
        }
        # Normalize line endings; use -ceq for case-sensitive comparison
        $expected = $task.expected -replace "`r`n", "`n"
        $actual = $actual -replace "`r`n", "`n"
        return [pscustomobject]@{
            Pass = (($actual -ceq $expected) -and ($exitCode -eq 0))
            Expected = $expected
            Actual = $actual
        }
    } finally {
        Remove-Item $tmpPath -Force -ErrorAction SilentlyContinue
    }
}

$mode = if ($Verify) { "verify" } else { "candidates" }
Write-Host "Lom Eval Runner — mode: $mode"
Write-Host ""

foreach ($file in $taskFiles) {
    $tasks = Get-Content $file.FullName -Raw -Encoding UTF8 | ConvertFrom-Json
    $category = $file.BaseName -replace '^\d+_', ''
    if (-not $stats.byCategory.ContainsKey($category)) {
        $stats.byCategory[$category] = [ordered]@{ total = 0; passed = 0; failed = 0 }
    }

    foreach ($task in $tasks) {
        $stats.total++
        $stats.byCategory[$category].total++

        if ($Verify) {
            $src = $task.solution
        } else {
            $candidatePath = Join-Path $CandidatesDir "$($task.id).lom"
            if (-not (Test-Path $candidatePath)) {
                $stats.failed++
                $stats.byCategory[$category].failed++
                if ($Verbose) { Write-Host "  [$($task.id)] MISSING candidate: $candidatePath" }
                continue
            }
            $src = Get-Content $candidatePath -Raw
        }

        $result = Test-OneTask $task $src
        if ($result.Pass) {
            $stats.passed++
            $stats.byCategory[$category].passed++
            if ($Verbose) { Write-Host "  [$($task.id)] PASS ($category)" }
        } else {
            $stats.failed++
            $stats.byCategory[$category].failed++
            if ($Verbose) {
                Write-Host "  [$($task.id)] FAIL ($category)" -ForegroundColor Red
                Write-Host "    expected: $($result.Expected -replace "`n", "\n")"
                Write-Host "    actual:   $($result.Actual -replace "`n", "\n")"
            }
        }
    }
}

# Summary
Write-Host ""
Write-Host "===== Summary ====="
Write-Host ("Total:  {0}" -f $stats.total)
Write-Host ("Passed: {0}" -f $stats.passed)
Write-Host ("Failed: {0}" -f $stats.failed)
if ($stats.total -gt 0) {
    $rate = [math]::Round($stats.passed / $stats.total * 100, 1)
    Write-Host ("Rate:   {0}%" -f $rate)
}
Write-Host ""
Write-Host "===== By category ====="
foreach ($cat in $stats.byCategory.Keys) {
    $c = $stats.byCategory[$cat]
    $r = if ($c.total -gt 0) { [math]::Round($c.passed / $c.total * 100, 1) } else { 0 }
    Write-Host ("  {0,-20} {1}/{2} ({3}%)" -f $cat, $c.passed, $c.total, $r)
}

if ($stats.failed -gt 0) { exit 1 } else { exit 0 }

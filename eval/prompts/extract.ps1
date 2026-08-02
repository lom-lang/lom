# extract.ps1 - Extract candidates/<id>.lom files from LLM output
#
# LLM output format (as required by prompts):
#   === 001.lom ===
#   <code>
#   === 002.lom ===
#   <code>
#   ...
#
# This script splits such output into eval/candidates/001.lom, 002.lom, ...
#
# Usage:
#   1. Save LLM reply to a text file (e.g. eval/candidates/output_01.txt)
#   2. Run:
#      powershell -ExecutionPolicy Bypass -File eval/prompts/extract.ps1 -InputFile eval/candidates/output_01.txt
#      (run multiple times for different files; merges into same candidates/ dir)
#   3. Or pipe directly:
#      Get-Content eval/candidates/output_01.txt -Raw | powershell -ExecutionPolicy Bypass -File eval/prompts/extract.ps1 -Stdin
#
# Options:
#   -InputFile <path>   Input file (LLM reply saved as text)
#   -OutputDir <path>   Output directory (default: eval/candidates)
#   -Stdin              Read from stdin (pipe mode)

param(
    [string]$InputFile,
    [string]$OutputDir,
    [switch]$Stdin
)

$ErrorActionPreference = "Stop"

# Determine eval directory
$scriptDir = $PSScriptRoot
$evalDir = Split-Path -Parent $scriptDir
if (-not $OutputDir) {
    $OutputDir = Join-Path $evalDir "candidates"
}

# Read input
if ($Stdin) {
    $content = $input | Out-String
} elseif ($InputFile) {
    $content = [System.IO.File]::ReadAllText($InputFile)
} else {
    Write-Host "Usage: extract.ps1 -InputFile <path>  |  extract.ps1 -Stdin"
    Write-Host ""
    Write-Host "Save LLM reply to a file, then use -InputFile; or pipe via -Stdin."
    exit 1
}

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

# Extract content between === <id>.lom === markers
$pattern = '(?m)^={3,}\s*(\d{3})\.lom\s*={3,}\s*$'
$matches = [regex]::Matches($content, $pattern)

if ($matches.Count -eq 0) {
    Write-Host "No '=== <id>.lom ===' delimiters found. Check LLM output format." -ForegroundColor Red
    Write-Host "Expected format:"
    Write-Host "  === 001.lom ==="
    Write-Host "  <code>"
    Write-Host "  === 002.lom ==="
    Write-Host "  ..."
    exit 1
}

$count = 0
for ($i = 0; $i -lt $matches.Count; $i++) {
    $id = $matches[$i].Groups[1].Value
    $startIdx = $matches[$i].Index + $matches[$i].Length
    if ($i + 1 -lt $matches.Count) {
        $code = $content.Substring($startIdx, $matches[$i + 1].Index - $startIdx)
    } else {
        $code = $content.Substring($startIdx)
    }

    # Trim leading/trailing whitespace
    $code = $code.TrimStart("`r`n").TrimEnd("`r`n ")
    # Strip markdown code fences if LLM wrapped code in ```
    $fence = [string][char]96 + [string][char]96 + [string][char]96
    if ($code.StartsWith($fence)) {
        $code = $code -replace ('(?s)^' + $fence + '[^\r\n]*\r?\n'), ''
        $code = $code -replace ('(?s)\r?\n' + $fence + '\s*$'), ''
    }

    $outPath = Join-Path $OutputDir "$id.lom"
    [System.IO.File]::WriteAllText($outPath, $code + "`n")
    $count++
    Write-Host "  Extracted: $id.lom ($($code.Length) chars)"
}

Write-Host ""
Write-Host "Done. $count files extracted to: $OutputDir"
Write-Host "Run eval: eval\runner\run.ps1 -CandidatesDir `"$OutputDir`""

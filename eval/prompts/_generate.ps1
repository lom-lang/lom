# 生成 Lom 评测集提示词文件（Phase 2.8 LLM 实测）
#
# 读取 eval/tasks/*.json + _context.md + _footer.md，为每个分类生成自包含提示词 .md。
#
# 用法：powershell -ExecutionPolicy Bypass -File eval/prompts/_generate.ps1

$ErrorActionPreference = "Stop"
$promptsDir = $PSScriptRoot
$tasksDir = Join-Path (Split-Path -Parent $promptsDir) "tasks"

$context = [System.IO.File]::ReadAllText((Join-Path $promptsDir "_context.md"))
$footer = [System.IO.File]::ReadAllText((Join-Path $promptsDir "_footer.md"))

$taskFiles = Get-ChildItem -Path $tasksDir -Filter "*.json" | Sort-Object Name
foreach ($file in $taskFiles) {
    $json = [System.IO.File]::ReadAllText($file.FullName)
    $tasks = $json | ConvertFrom-Json

    $body = ""
    foreach ($task in $tasks) {
        $body += "### Task $($task.id)`r`n`r`n$($task.prompt)`r`n`r`n"
    }

    $content = $context + $body + $footer
    $promptFile = Join-Path $promptsDir "$($file.BaseName).md"
    [System.IO.File]::WriteAllText($promptFile, $content)
    Write-Host "Generated: $($file.BaseName).md ($($tasks.Count) tasks)"
}

Write-Host ""
Write-Host "Done. $($taskFiles.Count) prompt files generated in: $promptsDir"

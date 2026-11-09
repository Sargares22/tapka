# Adds, updates or removes one task in Tapka's section of tasks (references/tasks.md).
# Usage: task.ps1 -Id camera -Name Camera -Percent 46.5 -Status "clip 3 of 35"
#        task.ps1 -Id camera -Done
param(
  [Parameter(Mandatory)][string]$Id,
  [string]$Name,
  [string]$Percent,
  [string]$Status,
  [int]$Fresh,
  [switch]$Done
)
$ErrorActionPreference = 'Stop'
$dir = if ($env:TAPKA_TASKS_DIR) { $env:TAPKA_TASKS_DIR } else { Join-Path $env:USERPROFILE '.tapka\tasks' }
$path = Join-Path $dir "$Id.task"
if ($Done) { Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue; exit 0 }
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$lines = @()
if ($Name) { $lines += "name: $Name" }
if ($Percent -ne '') { $lines += "percent: $Percent" }
if ($Status) { $lines += "status: $Status" }
if ($Fresh -gt 0) { $lines += "fresh: $Fresh" }
if (-not $lines) { $lines = @("name: $Id") }
# Written beside it and moved in, so Tapka never sees half a file; the task never leaves the folder
$tmp = "$path.tmp"
[IO.File]::WriteAllText($tmp, (($lines -join "`r`n") + "`r`n"), (New-Object Text.UTF8Encoding $false))
if (Test-Path -LiteralPath $path) { [IO.File]::Replace($tmp, $path, [NullString]::Value) } else { [IO.File]::Move($tmp, $path) }

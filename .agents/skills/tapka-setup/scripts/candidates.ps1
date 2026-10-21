# Read-only. Lists what could go into Tapka on this machine, as JSON:
#   start    - Start menu shortcuts to an exe (name, exe)
#   store    - Store apps (name, target "shell:AppsFolder\...")
#   running  - running programs that have a window (name, exe, title)
#   progress - taskbar buttons that show a progress bar now (button, percent)
#   edge     - windows that stay above the others and stand at a screen edge (exe, class, title, rect)
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File candidates.ps1 > candidates.json
$ErrorActionPreference = 'SilentlyContinue'

$shell = New-Object -ComObject WScript.Shell
$start = foreach ($root in "$env:APPDATA\Microsoft\Windows\Start Menu\Programs", "$env:ProgramData\Microsoft\Windows\Start Menu\Programs") {
  Get-ChildItem $root -Recurse -Filter *.lnk | ForEach-Object {
    $t = $shell.CreateShortcut($_.FullName).TargetPath
    if ($t -like '*.exe' -and (Test-Path $t)) { [pscustomobject]@{ name = $_.BaseName; exe = $t } }
  }
}
$start = @($start | Sort-Object exe -Unique)

$store = @(Get-StartApps | Where-Object { $_.AppID -like '*!*' } |
  ForEach-Object { [pscustomobject]@{ name = $_.Name; target = "shell:AppsFolder\$($_.AppID)" } })

$running = @(Get-Process | Where-Object { $_.MainWindowHandle -ne 0 -and $_.Path } |
  Sort-Object Path -Unique |
  ForEach-Object { [pscustomobject]@{ name = $_.ProcessName; exe = $_.Path; title = $_.MainWindowTitle } })

Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$ua = [System.Windows.Automation.AutomationElement]
$bar = $ua::RootElement.FindFirst('Children', (New-Object System.Windows.Automation.PropertyCondition($ua::ClassNameProperty, 'Shell_TrayWnd')))
$isBar = New-Object System.Windows.Automation.PropertyCondition($ua::ControlTypeProperty, [System.Windows.Automation.ControlType]::ProgressBar)
$walker = [System.Windows.Automation.TreeWalker]::ControlViewWalker
$progress = @(if ($bar) {
  foreach ($p in $bar.FindAll('Descendants', $isBar)) {
    $at = $p; $id = $null
    for ($i = 0; $i -lt 4 -and $at; $i++) {
      $at = $walker.GetParent($at)
      if ($at -and $at.Current.AutomationId -like 'Appid:*') { $id = $at.Current.AutomationId; break }
    }
    $rv = $p.GetCurrentPattern([System.Windows.Automation.RangeValuePattern]::Pattern)
    [pscustomobject]@{ button = $id; percent = [math]::Round($rv.Current.Value) }
  }
})

Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class TapkaEdge {
  delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetWindowLongPtr(IntPtr h, int i);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  public static List<IntPtr> Topmost() {
    var all = new List<IntPtr>();
    EnumWindows((h, l) => {
      long ex = (long)GetWindowLongPtr(h, -20);
      if (IsWindowVisible(h) && (ex & 0x8) != 0) all.Add(h);
      return true;
    }, IntPtr.Zero);
    return all;
  }
}
"@
Add-Type -AssemblyName System.Windows.Forms
$screens = [System.Windows.Forms.Screen]::AllScreens | ForEach-Object { $_.Bounds }
$edge = @(foreach ($h in [TapkaEdge]::Topmost()) {
  $r = New-Object TapkaEdge+RECT; [void][TapkaEdge]::GetWindowRect($h, [ref]$r)
  $w = $r.R - $r.L; $hh = $r.B - $r.T
  if ($w -lt 20 -or $hh -lt 20) { continue }
  $near = $screens | Where-Object {
    $w -lt $_.Width * 0.5 -and $hh -lt $_.Height * 0.9 -and
    ([math]::Abs($r.R - $_.Right) -le 64 -or [math]::Abs($r.L - $_.Left) -le 64 -or [math]::Abs($r.T - $_.Top) -le 64)
  }
  if (-not $near) { continue }
  $procId = 0; [void][TapkaEdge]::GetWindowThreadProcessId($h, [ref]$procId)
  $proc = Get-Process -Id $procId
  if (-not $proc.Path -or $proc.Path -like "$env:WINDIR\*") { continue }
  $c = New-Object System.Text.StringBuilder 256; [void][TapkaEdge]::GetClassName($h, $c, 256)
  $t = New-Object System.Text.StringBuilder 256; [void][TapkaEdge]::GetWindowText($h, $t, 256)
  [pscustomobject]@{ exe = $proc.Path; class = "$c"; title = "$t"; rect = "$($r.L),$($r.T) ${w}x$hh" }
})

[pscustomobject]@{ start = $start; store = $store; running = $running; progress = $progress; edge = $edge } |
  ConvertTo-Json -Depth 4

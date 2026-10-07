# Lists every top-level window of a running process with its class, styles, owner and whether
# its styles give it a taskbar button (printed in red), then what the taskbar itself shows for
# the program: a button removed with ITaskbarList::DeleteTab is only seen there.
# Usage: powershell -File scripts/windows-of.ps1 [process name, default tapka]
param([string]$Name = "tapka")

Add-Type @"
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class Top {
    delegate bool EnumProc(IntPtr hwnd, IntPtr lParam);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hwnd, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr hwnd, StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern IntPtr GetWindowLongPtr(IntPtr hwnd, int index);
    [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr hwnd, uint cmd);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
    public static List<IntPtr> Of(uint[] pids) {
        var found = new List<IntPtr>();
        EnumWindows((h, l) => {
            uint pid; GetWindowThreadProcessId(h, out pid);
            if (Array.IndexOf(pids, pid) >= 0) found.Add(h);
            return true;
        }, IntPtr.Zero);
        return found;
    }
}
"@

$procs = @(Get-Process -Name $Name -ErrorAction SilentlyContinue)
if (-not $procs) { Write-Host "no process named $Name"; exit 1 }
$pids = [uint32[]]($procs | ForEach-Object { $_.Id })

$styles = [ordered]@{ WS_POPUP = 0x80000000; WS_CHILD = 0x40000000; WS_VISIBLE = 0x10000000; WS_CAPTION = 0x00C00000; WS_THICKFRAME = 0x00040000; WS_SYSMENU = 0x00080000 }
$exStyles = [ordered]@{ WS_EX_TOOLWINDOW = 0x80; WS_EX_APPWINDOW = 0x40000; WS_EX_NOACTIVATE = 0x08000000; WS_EX_TOPMOST = 0x8; WS_EX_LAYERED = 0x80000; WS_EX_TRANSPARENT = 0x20; WS_EX_NOREDIRECTIONBITMAP = 0x00200000 }
function Names($value, $table) { ($table.Keys | Where-Object { ($value -band $table[$_]) -ne 0 }) -join " " }

$buttons = 0
foreach ($h in [Top]::Of($pids)) {
    $text = New-Object System.Text.StringBuilder 256; [void][Top]::GetWindowText($h, $text, 256)
    $class = New-Object System.Text.StringBuilder 256; [void][Top]::GetClassName($h, $class, 256)
    $style = [int64][Top]::GetWindowLongPtr($h, -16) -band 0xFFFFFFFF
    $ex = [int64][Top]::GetWindowLongPtr($h, -20) -band 0xFFFFFFFF
    $owner = [Top]::GetWindow($h, 4)
    $visible = [Top]::IsWindowVisible($h)
    # The taskbar rule: a visible window gets a button if it has WS_EX_APPWINDOW, or if it has
    # no owner and no WS_EX_TOOLWINDOW. The taskbar line at the end says if one was removed.
    $button = $visible -and ((($ex -band 0x40000) -ne 0) -or ($owner -eq [IntPtr]::Zero -and ($ex -band 0x80) -eq 0))
    $line = "0x{0:X} class={1} title='{2}' visible={3} owner=0x{4:X} style=0x{5:X} [{6}] ex=0x{7:X} [{8}] taskbar={9}" -f `
        [int64]$h, $class, $text, $visible, [int64]$owner, $style, (Names $style $styles), $ex, (Names $ex $exStyles), $button
    if ($button) { $buttons++; Write-Host $line -ForegroundColor Red } else { Write-Host $line }
}
Write-Host "windows whose styles give a taskbar button: $buttons"

Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$ua = [System.Windows.Automation.AutomationElement]
$bar = $ua::RootElement.FindFirst("Children", (New-Object System.Windows.Automation.PropertyCondition($ua::ClassNameProperty, "Shell_TrayWnd")))
# A desktop program's button is named by its exe path; a Store app's by its package, not matched
$exe = $procs[0].Path
$shown = @($bar.FindAll("Descendants", [System.Windows.Automation.Condition]::TrueCondition) |
    Where-Object { $_.Current.AutomationId -like ("Appid: *\" + (Split-Path $exe -Leaf)) -and $_.Current.Name -match "running window" })
if ($shown) { $shown | ForEach-Object { Write-Host "taskbar shows: $($_.Current.Name)" -ForegroundColor Red } }
else { Write-Host "taskbar shows: nothing for $exe" }

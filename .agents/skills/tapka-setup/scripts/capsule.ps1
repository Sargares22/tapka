# Read-only. What the running Tapka capsule shows, read through UI Automation: one line per key
# and meter, as its button is named ("Snip", "C 64%"), and whether the capsule is on the screen.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File capsule.ps1
$procs = @(Get-Process -Name tapka -ErrorAction SilentlyContinue)
if (-not $procs) { Write-Output 'tapka: not running'; exit 1 }
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$ua = [System.Windows.Automation.AutomationElement]
$any = $false
foreach ($p in $procs) {
  $mine = New-Object System.Windows.Automation.PropertyCondition($ua::ProcessIdProperty, $p.Id)
  foreach ($win in $ua::RootElement.FindAll('Children', $mine)) {
    $isButton = New-Object System.Windows.Automation.PropertyCondition($ua::ControlTypeProperty, [System.Windows.Automation.ControlType]::Button)
    $buttons = @($win.FindAll('Descendants', $isButton))
    if (-not $buttons) { continue }
    $any = $true
    $r = $win.Current.BoundingRectangle
    Write-Output ("window '{0}' at {1},{2} {3}x{4}, offscreen={5}" -f $win.Current.Name, $r.X, $r.Y, $r.Width, $r.Height, $win.Current.IsOffscreen)
    foreach ($b in $buttons) { Write-Output ("  button '{0}'" -f ($b.Current.Name -replace '\s+', ' ').Trim()) }
  }
}
if (-not $any) { Write-Output 'tapka: running, no capsule buttons found (hidden, folded, or a keyboard is attached with "only without a keyboard" on)' }

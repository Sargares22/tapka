# Machine and install (mode A, steps 1–2)

## 1. Machine

```powershell
[Environment]::OSVersion.Version                 # 10.0.22000 or higher is Windows 11
$env:PROCESSOR_ARCHITECTURE                      # ARM64 or AMD64
Get-Process tapka -ErrorAction SilentlyContinue | Select-Object Path
Test-Path "$env:LOCALAPPDATA\Tapka\tapka.exe"
(Get-Item "$env:LOCALAPPDATA\Tapka\tapka.exe").VersionInfo.ProductVersion
Test-Path "$env:APPDATA\sargares22.tapka\settings.json"
```

| Finding | Action |
|---|---|
| Not Windows 11 | stop: Tapka is for Windows 11 |
| Tapka runs, latest version | skip step 2; go to candidates (mode A) or straight to mode B |
| Tapka installed, older | tell the person: Settings → General → *Update* in Tapka does it, signed |
| Not installed | step 2 |

The latest release and its files:
`https://api.github.com/repos/Sargares22/tapka/releases/latest` (`tag_name`, `assets[].name`,
`assets[].size`).

## 2. Install

1. Download the asset for the processor: `Tapka_<version>_arm64-setup.exe` for ARM64,
   `Tapka_<version>_x64-setup.exe` for AMD64, into the person's Downloads folder. Check its size
   against `assets[].size`.
2. Tell the person, in their language: "Open Downloads and run `Tapka_…-setup.exe`. Windows will
   show *Windows protected your PC*: press *More info*, then *Run anyway*. Administrator rights
   are not needed." The installer is not signed; this window is for the person to answer, not you.
3. Wait for their word, then check: `Get-Process tapka` runs from
   `%LOCALAPPDATA%\Tapka\tapka.exe`, `%APPDATA%\sargares22.tapka\settings.json` exists, and
   `panel.log` beside it has a line `settings: N items, first run`.
4. On the first run Tapka opens a small window beside the capsule that offers their programs.
   Tell the person they may press *Not now*: you add the programs yourself in the next steps.

If the person's antivirus removed or blocked the file, say so plainly and point to the Privacy
section of the README: Tapka sends key presses, which some antivirus programs dislike. Do not
work around the antivirus.

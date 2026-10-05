; Removing Tapka also removes its "start with Windows" entry. An update keeps it.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Tapka"
  ${EndIf}
!macroend

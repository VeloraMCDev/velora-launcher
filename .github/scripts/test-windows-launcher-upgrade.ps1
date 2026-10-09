param([Parameter(Mandatory=$true)][string]$Installer)
$ErrorActionPreference='Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_OS -ne 'Windows') { throw 'Run only on a disposable GitHub Windows runner.' }
$taskRegistry='HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\SCOPENET Launcher'
if (Test-Path -LiteralPath $taskRegistry) { throw 'Runner already contains a launcher installation.' }
$taskOld=Join-Path $env:RUNNER_TEMP 'velora-upgrade-from-1.3.0.exe'
Invoke-WebRequest 'https://github.com/VeloraMCDev/velora-launcher/releases/download/launcher-v1.3.0/Velora-Launcher_1.3.0_x64-setup.exe' -OutFile $taskOld
if ((Get-FileHash -LiteralPath $taskOld -Algorithm SHA256).Hash.ToLowerInvariant() -ne 'dfb835893139913d0370c1d9fd1384ff514c3c1052579b7dbbdb347cb3d75816') { throw 'Historical installer checksum differs.' }
$taskData=Join-Path $env:APPDATA 'net.scopenet.launcher'
New-Item -ItemType Directory -Path $taskData -Force | Out-Null
$taskFixture=Join-Path $taskData 'upgrade-fixture.txt'
[IO.File]::WriteAllText($taskFixture,'synthetic-upgrade-continuity')
$taskProcess=Start-Process -FilePath $taskOld -ArgumentList '/S' -WindowStyle Hidden -Wait -PassThru
if ($taskProcess.ExitCode -ne 0) { throw 'Historical installation failed.' }
if ((Get-ItemProperty -LiteralPath $taskRegistry).DisplayVersion -ne '1.3.0') { throw 'Historical version was not registered.' }
$taskProcess=Start-Process -FilePath (Resolve-Path -LiteralPath $Installer).Path -ArgumentList '/S','/UPDATE' -WindowStyle Hidden -Wait -PassThru
if ($taskProcess.ExitCode -ne 0) { throw 'Upgrade installer failed.' }
$taskInstalled=Get-ItemProperty -LiteralPath $taskRegistry
$taskVersion=(Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json).version
if ($taskInstalled.DisplayName -ne 'Velora Launcher' -or $taskInstalled.Publisher -ne 'Velora' -or $taskInstalled.DisplayVersion -ne $taskVersion) { throw 'Upgrade registry identity/display fields are incorrect.' }
$taskLocation=(Get-Item -LiteralPath 'HKCU:\Software\SCOPENET\SCOPENET Launcher').GetValue('')
if (!(Test-Path -LiteralPath (Join-Path $taskLocation 'velora-launcher.exe'))) { throw 'New executable was not installed in the previous directory.' }
if ([IO.File]::ReadAllText($taskFixture) -ne 'synthetic-upgrade-continuity') { throw 'Upgrade changed persisted user data.' }
Write-Output 'Windows 1.3.0 upgrade passed: install identity, Velora display fields, executable and persisted data.'

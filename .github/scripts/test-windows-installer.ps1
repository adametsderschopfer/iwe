# Future release-runner smoke test. Never run against a developer installation.
param([Parameter(Mandatory)][string]$Installer)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'Installer smoke tests require an isolated CI runner.' }
$directory = Join-Path $env:RUNNER_TEMP ('iwe-install-test-' + [guid]::NewGuid())
$log = "$directory.log"
$process = Start-Process $Installer -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/LANG=english', "/DIR=`"$directory`"", "/LOG=`"$log`"") -Wait -PassThru
if ($process.ExitCode -ne 0) { throw "Installer failed: $($process.ExitCode). See $log" }
if (-not (Test-Path "$directory/iwe.exe")) { throw 'Installed executable is missing.' }
$entries = @(Get-ChildItem 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall', 'HKCU:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall' -ErrorAction SilentlyContinue |
    Get-ItemProperty | Where-Object { $_.DisplayName -eq 'IWE Code' -and $_.InstallLocation.TrimEnd('\') -eq $directory })
if ($entries.Count -ne 1) { throw 'Installer registration is missing or duplicated.' }
$process = Start-Process "$directory/unins000.exe" -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART') -Wait -PassThru
if ($process.ExitCode -ne 0) { throw 'Uninstallation failed.' }
for ($attempt = 0; $attempt -lt 100 -and (Test-Path $directory); $attempt++) { Start-Sleep -Milliseconds 100 }
if (Test-Path $directory) { throw 'Uninstallation left application files behind.' }

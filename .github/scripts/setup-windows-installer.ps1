# Install the pinned, publisher-signed Inno Setup compiler on a release runner.
$ErrorActionPreference = 'Stop'
$archive = Join-Path $env:RUNNER_TEMP 'innosetup-7.1.1-x64.exe'
Invoke-WebRequest 'https://github.com/jrsoftware/issrc/releases/download/is-7_1_1/innosetup-7.1.1-x64.exe' -OutFile $archive
if ((Get-FileHash $archive -Algorithm SHA256).Hash -ne '8A5A6711C3766A6E5FB74F280BA0E3551E719CE94538D901FDBD1F8F9920B3F0') {
    throw 'Unexpected Inno Setup checksum.'
}
$signature = Get-AuthenticodeSignature $archive
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'CN=Pyrsys B.V.') {
    throw 'Invalid Inno Setup publisher signature.'
}
$directory = Join-Path $env:RUNNER_TEMP 'inno-setup'
$process = Start-Process $archive -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/DIR=`"$directory`"") -Wait -PassThru
if ($process.ExitCode -ne 0) { throw "Inno Setup installation failed: $($process.ExitCode)" }
"IWE_ISCC=$directory\ISCC.exe" >> $env:GITHUB_ENV

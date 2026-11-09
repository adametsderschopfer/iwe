param([Parameter(Mandatory)][string]$File)
$ErrorActionPreference = 'Stop'
foreach ($name in @('IWE_SIGNTOOL', 'IWE_SIGNING_DLIB', 'IWE_SIGNING_METADATA')) {
    if (-not (Test-Path ([Environment]::GetEnvironmentVariable($name)))) { throw "Missing signing tool: $name" }
}
& $env:IWE_SIGNTOOL sign /v /fd SHA256 /tr http://timestamp.acs.microsoft.com /td SHA256 /dlib $env:IWE_SIGNING_DLIB /dmdf $env:IWE_SIGNING_METADATA $File
if ($LASTEXITCODE -ne 0) { throw "Signing failed: $File" }
$signature = Get-AuthenticodeSignature $File
if ($signature.Status -ne 'Valid' -or -not $signature.TimeStamperCertificate) {
    throw "Invalid or untimestamped signature: $File"
}

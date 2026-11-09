# Prepare Microsoft Artifact Signing. Azure login uses GitHub OIDC, without a client secret.
$ErrorActionPreference = 'Stop'
foreach ($name in @('IWE_SIGNING_ENDPOINT', 'IWE_SIGNING_ACCOUNT', 'IWE_SIGNING_PROFILE')) {
    if (-not [Environment]::GetEnvironmentVariable($name)) { throw "Missing signing setting: $name" }
}
$directory = Join-Path $env:RUNNER_TEMP 'artifact-signing'
New-Item $directory -ItemType Directory -Force | Out-Null
$archive = Join-Path $directory 'client.zip'
Invoke-WebRequest 'https://api.nuget.org/v3-flatcontainer/microsoft.artifactsigning.client/1.0.151/microsoft.artifactsigning.client.1.0.151.nupkg' -OutFile $archive
if ((Get-FileHash $archive -Algorithm SHA256).Hash -ne '35375A4BC4E1102DA729FC3D8D9925F774C3BAAE6B8B7FF6398D948CA9AB0D8A') {
    throw 'Unexpected Artifact Signing client checksum.'
}
Expand-Archive $archive $directory -Force
$signTool = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe' |
    Where-Object { [version]$_.Directory.Parent.Name -ge [version]'10.0.22621.0' } |
    Sort-Object { [version]$_.Directory.Parent.Name } -Descending | Select-Object -First 1
if (-not $signTool) { throw 'A supported Windows SDK signing tool is required.' }
$metadata = Join-Path $directory 'metadata.json'
@{
    Endpoint = $env:IWE_SIGNING_ENDPOINT
    CodeSigningAccountName = $env:IWE_SIGNING_ACCOUNT
    CertificateProfileName = $env:IWE_SIGNING_PROFILE
    ExcludeCredentials = @('EnvironmentCredential', 'WorkloadIdentityCredential', 'ManagedIdentityCredential',
        'SharedTokenCacheCredential', 'VisualStudioCredential', 'VisualStudioCodeCredential',
        'AzurePowerShellCredential', 'AzureDeveloperCliCredential', 'InteractiveBrowserCredential')
} | ConvertTo-Json | Set-Content $metadata -Encoding utf8
"IWE_SIGNTOOL=$($signTool.FullName)" >> $env:GITHUB_ENV
"IWE_SIGNING_DLIB=$directory\bin\x64\Azure.CodeSigning.Dlib.dll" >> $env:GITHUB_ENV
"IWE_SIGNING_METADATA=$metadata" >> $env:GITHUB_ENV

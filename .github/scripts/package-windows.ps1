# Package an existing release executable. This script never compiles the application.
$ErrorActionPreference = 'Stop'
foreach ($name in @('BUILD_TARGET', 'ASSET_PLATFORM', 'RELEASE_VERSION', 'IWE_ISCC')) {
    if (-not [Environment]::GetEnvironmentVariable($name)) { throw "Missing packaging setting: $name" }
}
$root = (Resolve-Path "$PSScriptRoot/../..").Path
$stage = Join-Path $env:RUNNER_TEMP "iwe-installer-$env:ASSET_PLATFORM"
$dist = Join-Path $root 'dist'
New-Item $stage, $dist -ItemType Directory -Force | Out-Null
try {
    Copy-Item "$root/target/$env:BUILD_TARGET/release/iwe.exe" "$stage/iwe.exe"
    Copy-Item "$root/README.md", "$root/assets/icon.ico", "$root/assets/icon.png" $stage
    & python "$PSScriptRoot/installer-messages.py" "$stage/messages.iss"
    if ($LASTEXITCODE -ne 0) { throw 'Installer translation export failed.' }
    Add-Type -AssemblyName System.Drawing
    $canvas = [System.Drawing.Bitmap]::new(656, 1256)
    $graphics = [System.Drawing.Graphics]::FromImage($canvas)
    $icon = [System.Drawing.Image]::FromFile("$stage/icon.png")
    $font = [System.Drawing.Font]::new('Segoe UI', 38, [System.Drawing.FontStyle]::Regular)
    $brush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
    $format = [System.Drawing.StringFormat]::new()
    try {
        $graphics.Clear([System.Drawing.Color]::FromArgb(24, 24, 24))
        $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.DrawImage($icon, 176, 390, 304, 304)
        $format.Alignment = [System.Drawing.StringAlignment]::Center
        $graphics.DrawString('IWE Code', $font, $brush, [System.Drawing.RectangleF]::new(0, 750, 656, 100), $format)
        $canvas.Save("$stage/welcome.png", [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $format.Dispose(); $brush.Dispose(); $font.Dispose(); $icon.Dispose(); $graphics.Dispose(); $canvas.Dispose()
    }
    $signed = $env:IWE_WINDOWS_SIGNING_ENABLED -eq 'true'
    if ($signed) { & "$PSScriptRoot/sign-windows.ps1" -File "$stage/iwe.exe" }
    $architecture = if ($env:ASSET_PLATFORM -eq 'windows-x64') { 'x64' } elseif ($env:ASSET_PLATFORM -eq 'windows-x86') { 'x86' } else { throw 'Unsupported Windows architecture.' }
    $version = $env:RELEASE_VERSION
    $numeric = ($version -split '[-+]')[0]
    if ($numeric -notmatch '^\d+\.\d+\.\d+$') { throw 'Invalid release version.' }
    $arguments = @("/DAppVersion=$version", "/DFileVersion=$numeric.0", "/DAppArchitecture=$architecture",
        "/DPayloadDirectory=$stage", "/DOutputDirectory=$dist")
    if ($signed) {
        $arguments += '/DSignedBuild=1'
        $arguments += '/SIweCode=pwsh.exe -NoProfile -File $q' + "$PSScriptRoot/sign-windows.ps1" + '$q -File $f'
    }
    & $env:IWE_ISCC @arguments "$root/packaging/windows/setup.iss"
    if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed.' }
    $setup = "$dist/iwe-code-$version-$env:ASSET_PLATFORM-setup.exe"
    if (-not (Test-Path $setup)) { throw 'Installer output is missing.' }
    if ($signed -and (Get-AuthenticodeSignature $setup).Status -ne 'Valid') { throw 'Invalid installer signature.' }
    & tar -czf "$dist/iwe-$version-$env:ASSET_PLATFORM-update.tar.gz" -C $stage iwe.exe
    if ($LASTEXITCODE -ne 0) { throw 'Update archive creation failed.' }
    Compress-Archive -Path "$stage/iwe.exe", "$stage/README.md" -DestinationPath "$dist/iwe-code-$version-$env:ASSET_PLATFORM.zip" -Force
    & "$PSScriptRoot/test-windows-installer.ps1" -Installer $setup
} finally {
    Remove-Item $stage -Recurse -Force
}

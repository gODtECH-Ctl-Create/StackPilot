$ErrorActionPreference = "Stop"

$Repo = "gODtECH-Ctl-Create/StackPilot"
$Version = if ($env:STACKPILOT_VERSION) { $env:STACKPILOT_VERSION.Trim() } else { "latest" }
$InstallDir = if ($env:STACKPILOT_INSTALL_DIR) {
    $env:STACKPILOT_INSTALL_DIR
} else {
    Join-Path $env:LOCALAPPDATA "Programs\StackPilot"
}

if (-not [Environment]::Is64BitOperatingSystem) {
    throw "StackPilot currently publishes a Windows x86_64 binary and requires 64-bit Windows."
}

$Asset = "stackpilot-windows-x86_64.zip"
if ($Version -eq "latest") {
    $Url = "https://github.com/$Repo/releases/latest/download/$Asset"
    $ReleaseLabel = "latest release"
} else {
    $Tag = if ($Version.StartsWith("v")) { $Version } else { "v$Version" }
    $Url = "https://github.com/$Repo/releases/download/$Tag/$Asset"
    $ReleaseLabel = $Tag
}

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("stackpilot-" + [System.Guid]::NewGuid())
New-Item -ItemType Directory -Force -Path $TempDir | Out-Null

try {
    $Archive = Join-Path $TempDir $Asset
    Write-Host "Installing StackPilot for Windows x86_64 from $ReleaseLabel"

    try {
        Invoke-WebRequest -Uri $Url -OutFile $Archive
    }
    catch {
        throw @"
StackPilot installation failed because the release asset '$Asset' could not be downloaded.

Requested: $Url

This usually means the selected GitHub release does not contain the Windows binary yet.
Check the published release assets at:
https://github.com/$Repo/releases

To install a specific release, set STACKPILOT_VERSION first, for example:
`$env:STACKPILOT_VERSION = "0.1.1"
"@
    }

    Expand-Archive -Path $Archive -DestinationPath $TempDir -Force

    $ExtractedBinary = Join-Path $TempDir "stackpilot.exe"
    if (-not (Test-Path $ExtractedBinary)) {
        throw "The downloaded archive did not contain stackpilot.exe. The release package may be invalid."
    }

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item $ExtractedBinary (Join-Path $InstallDir "stackpilot.exe") -Force

    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $PathEntries = if ($UserPath) { $UserPath -split ';' } else { @() }
    if ($PathEntries -notcontains $InstallDir) {
        $NewUserPath = if ($UserPath) { "$UserPath;$InstallDir" } else { $InstallDir }
        [Environment]::SetEnvironmentVariable("Path", $NewUserPath, "User")
        Write-Host "Added $InstallDir to your user PATH."
    }

    if (($env:Path -split ';') -notcontains $InstallDir) {
        $env:Path = "$InstallDir;$env:Path"
    }

    Write-Host "Installed StackPilot to $InstallDir\stackpilot.exe"
    Write-Host "Run 'stackpilot --help' in this PowerShell session."
}
finally {
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
}

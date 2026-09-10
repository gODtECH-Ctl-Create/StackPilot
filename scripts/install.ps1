$ErrorActionPreference = "Stop"

$Repo = "gODtECH-Ctl-Create/StackPilot"
$Version = if ($env:STACKPILOT_VERSION) { $env:STACKPILOT_VERSION } else { "latest" }
$InstallDir = if ($env:STACKPILOT_INSTALL_DIR) {
    $env:STACKPILOT_INSTALL_DIR
} else {
    Join-Path $env:LOCALAPPDATA "Programs\StackPilot"
}

$Asset = "stackpilot-windows-x86_64.zip"
if ($Version -eq "latest") {
    $Url = "https://github.com/$Repo/releases/latest/download/$Asset"
} else {
    $Tag = if ($Version.StartsWith("v")) { $Version } else { "v$Version" }
    $Url = "https://github.com/$Repo/releases/download/$Tag/$Asset"
}

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("stackpilot-" + [System.Guid]::NewGuid())
New-Item -ItemType Directory -Force -Path $TempDir | Out-Null

try {
    $Archive = Join-Path $TempDir $Asset
    Write-Host "Installing StackPilot for Windows x86_64 from $Version"
    Invoke-WebRequest -Uri $Url -OutFile $Archive
    Expand-Archive -Path $Archive -DestinationPath $TempDir -Force

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item (Join-Path $TempDir "stackpilot.exe") (Join-Path $InstallDir "stackpilot.exe") -Force

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

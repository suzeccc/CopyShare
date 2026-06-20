param(
    [string]$Name = "LanClipboardSync",
    [switch]$StopRunning
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Entry = Join-Path $Root "lan_clipboard_sync_ui.py"
$Dist = Join-Path $Root "dist"
$Build = Join-Path $Root "build"
$Target = Join-Path $Dist "$Name.exe"

$Running = Get-Process -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -and ($_.Path -ieq $Target) }

if ($Running) {
    if (-not $StopRunning) {
        $Ids = ($Running | ForEach-Object { $_.Id }) -join ", "
        throw "Executable is running and cannot be overwritten: $Target (PID: $Ids). Close it first or rerun with -StopRunning."
    }

    $Running | Stop-Process -Force
    Start-Sleep -Seconds 1
}

python -m PyInstaller `
    --noconfirm `
    --clean `
    --onefile `
    --windowed `
    --name $Name `
    --distpath $Dist `
    --workpath $Build `
    --specpath $Build `
    $Entry

Write-Host "Packaged executable: $Target"

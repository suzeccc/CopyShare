param(
    [Parameter(Mandatory = $true)]
    [string]$SessionDir
)

$stateDir = Join-Path $SessionDir 'state'
New-Item -ItemType Directory -Force -Path $stateDir | Out-Null
$logFile = Join-Path $stateDir 'server.log'

$env:BRAINSTORM_DIR = $SessionDir
$env:BRAINSTORM_HOST = '127.0.0.1'
$env:BRAINSTORM_URL_HOST = 'localhost'

Set-Location 'C:\Users\SuZe\.codex\skills\brainstorming\scripts'
& 'D:\node\node.exe' 'C:\Users\SuZe\.codex\skills\brainstorming\scripts\server.cjs' *> $logFile

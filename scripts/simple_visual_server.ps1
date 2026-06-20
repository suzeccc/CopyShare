param(
    [Parameter(Mandatory = $true)]
    [string]$Root,

    [int]$Port = 61234
)

$stateDir = Join-Path $Root 'state'
$contentFile = Join-Path $Root 'index.html'
$eventsFile = Join-Path $stateDir 'events'
$infoFile = Join-Path $stateDir 'server-info'
$logFile = Join-Path $stateDir 'server.log'

New-Item -ItemType Directory -Force -Path $stateDir | Out-Null

function Write-Log($Message) {
    Add-Content -LiteralPath $logFile -Value ("[{0}] {1}" -f (Get-Date -Format o), $Message) -Encoding UTF8
}

$listener = [System.Net.HttpListener]::new()
$prefix = "http://127.0.0.1:$Port/"
$listener.Prefixes.Add($prefix)

try {
    $listener.Start()
    $info = @{
        type = 'server-started'
        port = $Port
        url = "http://localhost:$Port"
        screen_file = $contentFile
        state_dir = $stateDir
    } | ConvertTo-Json -Compress
    Set-Content -LiteralPath $infoFile -Value $info -Encoding UTF8
    Write-Log $info

    while ($listener.IsListening) {
        $context = $listener.GetContext()
        $request = $context.Request
        $response = $context.Response

        try {
            if ($request.HttpMethod -eq 'GET' -and ($request.Url.AbsolutePath -eq '/' -or $request.Url.AbsolutePath -eq '/index.html')) {
                $html = Get-Content -LiteralPath $contentFile -Raw -Encoding UTF8
                $bytes = [System.Text.Encoding]::UTF8.GetBytes($html)
                $response.ContentType = 'text/html; charset=utf-8'
                $response.StatusCode = 200
                $response.ContentLength64 = $bytes.Length
                $response.OutputStream.Write($bytes, 0, $bytes.Length)
            } elseif ($request.HttpMethod -eq 'POST' -and $request.Url.AbsolutePath -eq '/event') {
                $reader = [System.IO.StreamReader]::new($request.InputStream, [System.Text.Encoding]::UTF8)
                $body = $reader.ReadToEnd()
                $reader.Close()
                Add-Content -LiteralPath $eventsFile -Value $body -Encoding UTF8
                $bytes = [System.Text.Encoding]::UTF8.GetBytes('{"ok":true}')
                $response.ContentType = 'application/json; charset=utf-8'
                $response.StatusCode = 200
                $response.ContentLength64 = $bytes.Length
                $response.OutputStream.Write($bytes, 0, $bytes.Length)
            } else {
                $response.StatusCode = 404
            }
        } catch {
            Write-Log $_.Exception.ToString()
            $response.StatusCode = 500
        } finally {
            $response.OutputStream.Close()
        }
    }
} catch {
    Write-Log $_.Exception.ToString()
    throw
} finally {
    if ($listener.IsListening) {
        $listener.Stop()
    }
    $listener.Close()
}

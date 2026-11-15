# Prove the packaged Windows desktop profile, not just the in-tree supervisor.
#
# This is release tier. It silently installs the NSIS artifact into a throwaway
# directory, checks the installed payload (shell, qr-desktop, six .exe sidecars,
# client, staged config, no interpreter), launches the packaged qr-desktop with
# QR_PROFILE=desktop and a fresh QR_DATA_DIR, completes the verify-mini fixture
# through the packaged gateway, and compares the canonical digest with the
# server profile oracle. It then stops the supervisor and requires every
# packaged process to exit.
#
# tools/verify.sh is the shared fixture driver, so it runs under Git Bash on the
# runner. That driver builds its config bundle with the host interpreter named
# by QR_PYTHON; this script requires that variable rather than guessing, because
# the stock Windows image does not expose the versioned alias the driver
# defaults to.
#
# Usage:
#   smoke-package-parity.ps1 -Artifact path\to\QuantRule_<version>_x64-setup.exe `
#       -Shell target\release\quantrule.exe
#
# Options:
#   -Expected <digest>    Server canonical digest the package must reproduce
#                         (default: the #819 server oracle digest).
#   -ImageDigest <d>      QR_IMAGE_DIGEST both profiles record.
#   -Timeout <seconds>    Fixture timeout (default: 300).
#   -NegativeControl      Rerun the fixture with a seeded all-zero digest and
#                         require the comparison to fail.
#   -Keep                 Keep the install, data, and evidence directories.
param(
    [Parameter(Mandatory = $true)]
    [string]$Artifact,
    [string]$Shell,
    [string]$Expected = "sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa",
    [string]$ImageDigest = "sha256:1250438fd3a28c91f193f39bb45890d484c96ecddb554875f56bd995786ef4a0",
    [int]$Timeout = 300,
    [switch]$NegativeControl,
    [switch]$Keep
)

$ErrorActionPreference = "Stop"
$services = @("qr-gateway", "qr-data", "qr-model", "qr-backtest", "qr-evidence", "qr-simulation")
$seeded = "sha256:0000000000000000000000000000000000000000000000000000000000000000"
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Split-Path -Parent $here
$repository = Split-Path -Parent (Split-Path -Parent $root)
$work = Join-Path ([System.IO.Path]::GetTempPath()) ("qr-package-parity-" + [guid]::NewGuid().ToString("N"))
$install = Join-Path $work "install"
$createdTemp = $true
$script:supervisor = $null
$script:dataDir = $null
$script:sidecarDir = $null
$script:installDir = $null
$script:gateway = $null

function Fail([string]$message) {
    Write-Error "package-parity: $message" -ErrorAction Continue
    exit 1
}

function Say([string]$message) {
    Write-Host "package-parity: $message"
}

function Format-Argument([string]$Value) {
    if ($Value -match '\s') {
        return '"' + $Value.Replace('"', '\"') + '"'
    }
    return $Value
}

function Convert-ToPosix([string]$Path) {
    $value = & $script:cygpath -u $Path
    if ($LASTEXITCODE -ne 0) {
        throw "cannot convert $Path to a POSIX path"
    }
    return $value.Trim()
}

function Get-FreeBase([int]$Start) {
    foreach ($offset in 0..40) {
        $base = $Start + $offset * 20
        $listeners = @()
        $free = $true
        foreach ($port in $base..($base + 4)) {
            try {
                $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, $port)
                $listener.Start()
                $listeners += $listener
            } catch {
                $free = $false
                break
            }
        }
        foreach ($listener in $listeners) {
            $listener.Stop()
        }
        if ($free) {
            return $base
        }
    }
    throw "no free loopback port block from $Start"
}

function Get-PortsRecord([string]$Label) {
    $record = Join-Path $script:dataDir "supervisor\ports.json"
    $deadline = (Get-Date).AddSeconds(180)
    while ((Get-Date) -lt $deadline) {
        if ($script:supervisor.HasExited) {
            throw "$Label`: the packaged supervisor exited with code $($script:supervisor.ExitCode) before becoming ready"
        }
        if (Test-Path -LiteralPath $record) {
            try {
                $status = Get-Content -LiteralPath $record -Raw | ConvertFrom-Json
                if ($status.state -eq "ready") {
                    return $status
                }
            } catch {
                # A rename is atomic; a torn read is retried, not fatal.
            }
        }
        Start-Sleep -Milliseconds 200
    }
    throw "$Label`: the packaged supervisor did not become ready"
}

function Test-Health([int]$Port) {
    try {
        $body = Invoke-RestMethod -Uri "http://127.0.0.1:$Port/health" -TimeoutSec 3
        return $body.state -eq "healthy"
    } catch {
        return $false
    }
}

function Get-PackageProcesses {
    if (-not $script:sidecarDir) {
        return @()
    }
    $prefix = $script:installDir.TrimEnd("\") + "\"
    return @(Get-CimInstance Win32_Process | Where-Object {
            $_.ExecutablePath -and
            $_.ExecutablePath.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)
        })
}

function Assert-Sidecars([string]$Label, $status) {
    if ($status.profile -ne "desktop") {
        Fail "$Label`: ports.json profile is $($status.profile), expected desktop"
    }
    $names = @($status.services | ForEach-Object { $_.name } | Sort-Object)
    $wanted = @($services | Sort-Object)
    # Compare-Object is silent when the sets are equal; wrap it so an empty
    # difference is not mistaken for a failed comparison.
    if ($names.Count -ne 6 -or @(Compare-Object $names $wanted).Count -gt 0) {
        Fail "$Label`: ports.json names $($names -join ', '), expected six sidecars"
    }

    $running = Get-PackageProcesses
    foreach ($name in $services) {
        $exe = "$name.exe"
        if (-not ($running | Where-Object { $_.Name -eq $exe })) {
            Fail "$Label`: $exe is not running from the package"
        }
    }
    foreach ($service in $status.services) {
        if (-not (Get-Process -Id $service.pid -ErrorAction SilentlyContinue)) {
            Fail "$Label`: guard pid $($service.pid) for $($service.name) is not alive"
        }
    }

    $ports = @()
    foreach ($name in @("qr-gateway", "qr-data", "qr-model", "qr-evidence", "qr-simulation")) {
        $service = $status.services | Where-Object { $_.name -eq $name }
        if ($null -eq $service -or $null -eq $service.port) {
            Fail "$Label`: $name has no health port"
        }
        $ports += [int]$service.port
    }
    foreach ($port in $ports) {
        $healthy = $false
        foreach ($attempt in 1..100) {
            if (Test-Health $port) {
                $healthy = $true
                break
            }
            Start-Sleep -Milliseconds 100
        }
        if (-not $healthy) {
            Fail "$Label`: the service on port $port is not healthy"
        }
    }
    Say "$Label`: six sidecars running; five health endpoints healthy; qr-backtest worker alive"
}

function Invoke-Fixture([string]$Label, [string]$ExpectedDigest, [string]$Evidence, [string]$ConfigRoot) {
    $config = Join-Path $ConfigRoot "verify-mini\run.json"
    $replica = Join-Path $ConfigRoot "verify-mini-replica\run.json"
    if (-not (Test-Path -LiteralPath $config)) {
        Fail "$Label`: the package has no staged verify-mini config"
    }
    if (-not (Test-Path -LiteralPath $replica)) {
        Fail "$Label`: the package has no staged verify-mini replica config"
    }
    $arguments = @(
        (Convert-ToPosix (Join-Path $repository "tools\verify.sh")),
        "--api-url", "http://127.0.0.1:$($script:gateway)/api/v1",
        "--config", (Convert-ToPosix $config),
        "--replica", (Convert-ToPosix $replica),
        "--evidence-dir", (Convert-ToPosix $Evidence),
        "--expected", $ExpectedDigest,
        "--timeout", "$Timeout"
    )
    $env:QUANTRULE_SHELL = Convert-ToPosix $Shell
    Say "$Label`: running the fixture from the staged config with the host client"
    Push-Location $repository
    try {
        $output = & $script:bash @arguments 2>&1 | Out-String
        $code = $LASTEXITCODE
    } finally {
        Pop-Location
    }
    Write-Host $output.TrimEnd()
    return @{ Code = $code; Output = $output }
}

function Stop-Stack([string]$Label) {
    if ($null -eq $script:supervisor) {
        return
    }
    $pids = @(Get-PackageProcesses | ForEach-Object { $_.ProcessId })
    if (-not $script:supervisor.HasExited) {
        Stop-Process -Id $script:supervisor.Id -Force
        $script:supervisor.WaitForExit(15000) | Out-Null
    }
    $deadline = (Get-Date).AddSeconds(30)
    while ((Get-Date) -lt $deadline) {
        $alive = @($pids | Where-Object { Get-Process -Id $_ -ErrorAction SilentlyContinue })
        if ($alive.Count -eq 0) {
            break
        }
        Start-Sleep -Milliseconds 200
    }
    $alive = @(Get-PackageProcesses)
    if ($alive.Count -ne 0) {
        $names = ($alive | ForEach-Object {
                if ($_.Name) { "$($_.Name) ($($_.ProcessId))" } else { "pid $($_.Id)" }
            }) -join ", "
        foreach ($process in $alive) {
            $id = if ($process.ProcessId) { $process.ProcessId } else { $process.Id }
            Stop-Process -Id $id -Force -ErrorAction SilentlyContinue
        }
        Fail "$Label`: packaged processes survived the supervisor stop: $names"
    }
    Say "$Label`: supervisor and every packaged child stopped"
    $script:supervisor = $null
}

function Start-Stack([string]$Label, [int]$Base, [string]$Prefix) {
    $script:dataDir = Join-Path $Prefix "data"
    $logs = Join-Path $Prefix "logs"
    New-Item -ItemType Directory -Path $script:dataDir, $logs -Force | Out-Null
    $env:QR_DATA_DIR = $script:dataDir
    $arguments = @(
        "--profile", "desktop",
        "--resource-dir", $script:installDir,
        "--sidecar-dir", $script:sidecarDir,
        "--data-dir", $script:dataDir,
        "--gateway-port", "$Base",
        "--internal-base", "$($Base + 10)",
        "--health-timeout-ms", "120000",
        "--shutdown-grace-ms", "10000"
    )
    Say "$Label`: starting the packaged supervisor on $Base with a fresh data directory"
    $script:supervisor = Start-Process -FilePath $script:supervisorPath `
        -ArgumentList @($arguments | ForEach-Object { Format-Argument $_ }) `
        -PassThru `
        -RedirectStandardOutput (Join-Path $logs "supervisor.out.log") `
        -RedirectStandardError (Join-Path $logs "supervisor.err.log")
    $status = Get-PortsRecord $Label
    Copy-Item -LiteralPath (Join-Path $script:dataDir "supervisor\ports.json") `
        -Destination (Join-Path $Prefix "ports-ready.json") -Force
    $script:gateway = ([string]$status.gateway).Split(":")[-1]
    Say "$Label`: gateway healthy on $($script:gateway)"
    return $status
}

try {
    if (-not $IsWindows -and $PSVersionTable.PSVersion.Major -ge 6) {
        Fail "this smoke installs the Windows NSIS artifact and runs only on Windows"
    }
    if (-not (Test-Path -LiteralPath $Artifact)) {
        Fail "the installer $Artifact does not exist"
    }
    $Artifact = (Resolve-Path -LiteralPath $Artifact).Path
    if (-not $Shell) {
        $candidate = Join-Path $repository "target\release\quantrule.exe"
        if (-not (Test-Path -LiteralPath $candidate)) {
            $candidate = Join-Path $repository "target\debug\quantrule.exe"
        }
        $Shell = $candidate
    }
    if (-not (Test-Path -LiteralPath $Shell)) {
        Fail "the host client $Shell is missing; build it with cargo build --release -p quantrule-shell"
    }
    $Shell = (Resolve-Path -LiteralPath $Shell).Path
    if ($Timeout -le 0) {
        Fail "-Timeout must be greater than zero"
    }

    $bash = Get-Command bash.exe -ErrorAction SilentlyContinue
    if ($null -eq $bash) {
        $bashPath = "C:\Program Files\Git\bin\bash.exe"
        if (-not (Test-Path -LiteralPath $bashPath)) {
            Fail "Git Bash is required to run tools/verify.sh"
        }
    } else {
        $bashPath = $bash.Source
    }
    $script:bash = $bashPath
    $gitRoot = Split-Path -Parent (Split-Path -Parent $bashPath)
    $script:cygpath = Join-Path $gitRoot "usr\bin\cygpath.exe"
    if (-not (Test-Path -LiteralPath $script:cygpath)) {
        Fail "cygpath is missing beside Git Bash at $gitRoot"
    }

    New-Item -ItemType Directory -Path $work -Force | Out-Null

    if (-not $env:QR_PYTHON) {
        Fail "set QR_PYTHON to the host interpreter command so tools/verify.sh can build the config bundle"
    }

    # Silent per-user install into a throwaway directory. /D= must be last and
    # unquoted, exactly as check-package.ps1 does it.
    Say "installing the NSIS artifact into $install"
    $process = Start-Process -FilePath $Artifact -ArgumentList @("/S", "/D=$install") -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        Fail "the installer exited with code $($process.ExitCode)"
    }

    $sidecar = Get-ChildItem -Path $install -Recurse -Directory -Filter "sidecars" | Select-Object -First 1
    if ($null -eq $sidecar) {
        Fail "the installed package has no sidecars directory"
    }
    $script:sidecarDir = $sidecar.FullName
    $script:installDir = Split-Path -Parent $script:sidecarDir
    $script:supervisorPath = Join-Path $script:sidecarDir "qr-desktop.exe"
    if (-not (Test-Path -LiteralPath $script:supervisorPath)) {
        Fail "the installed package has no qr-desktop.exe"
    }
    $shellExe = Get-ChildItem -Path $install -Recurse -File -Filter "quantrule-desktop-shell.exe" | Select-Object -First 1
    if ($null -eq $shellExe) {
        Fail "the installed package has no quantrule-desktop-shell.exe"
    }
    foreach ($name in $services) {
        if (-not (Test-Path -LiteralPath (Join-Path $script:sidecarDir "$name.exe"))) {
            Fail "the installed package has no $name.exe sidecar"
        }
    }
    $client = Join-Path $script:installDir "client\index.html"
    if (-not (Test-Path -LiteralPath $client)) {
        Fail "the installed package has no client\index.html"
    }
    $configRoot = Join-Path $script:installDir "config"
    foreach ($name in @("verify-mini", "verify-mini-replica")) {
        if (-not (Test-Path -LiteralPath (Join-Path $configRoot "$name\run.json"))) {
            Fail "the installed package has no staged $name config"
        }
        if (-not (Test-Path -LiteralPath (Join-Path $configRoot "$name\candles.csv"))) {
            Fail "the installed package has no staged $name data"
        }
    }
    $interpreters = Get-ChildItem -Path $install -Recurse -File |
        Where-Object { $_.BaseName -match "^(py|bun|node)" -or $_.Extension -eq ".py" }
    if ($interpreters) {
        Fail "the package carries an interpreter: $($interpreters.Name -join ', ')"
    }
    Say "package contents ok: shell, qr-desktop, six sidecars, client, configs, no interpreter"

    $env:QR_PROFILE = "desktop"
    $env:QR_IMAGE_DIGEST = $ImageDigest
    $env:DOCKER_HOST = "npipe:////./pipe/quantrule-package-smoke-none"

    $base = Get-FreeBase 28800
    $status = Start-Stack "package" $base (Join-Path $work "run")
    Assert-Sidecars "package" $status
    $positive = Invoke-Fixture "package" $Expected (Join-Path $work "run\evidence") $configRoot
    if ($positive.Code -ne 0) {
        Fail "the packaged fixture failed with exit code $($positive.Code)"
    }
    $observed = "sha256:" + (Get-Content -LiteralPath (Join-Path $work "run\evidence\digest") -Raw).Trim()
    if ($observed -ne $Expected) {
        Fail "the packaged digest $observed does not match the server oracle $Expected"
    }
    $artifacts = @(Get-ChildItem -Path (Join-Path $work "run\evidence\artifacts") -Recurse -File).Count
    Say "packaged run reproduced $Expected; $artifacts canonical artifact files downloaded"
    Stop-Stack "package"

    if ($NegativeControl) {
        $base = Get-FreeBase 28900
        $status = Start-Stack "negative" $base (Join-Path $work "negative")
        Assert-Sidecars "negative" $status
        $negative = Invoke-Fixture "negative" $seeded (Join-Path $work "negative\evidence") $configRoot
        if ($negative.Code -eq 0) {
            Fail "the seeded mismatch did not fail; the digest comparison is vacuous"
        }
        if ($negative.Output -notmatch [regex]::Escape($seeded)) {
            Fail "the seeded mismatch failed without naming the expected digest"
        }
        Say "negative control ok: the seeded mismatch $seeded failed as required"
        Stop-Stack "negative"
    }

    Write-Host "package-parity ok: $observed"
    exit 0
}
finally {
    # Cleanup never throws: the primary failure already reported above.
    try {
        if ($script:supervisor -and -not $script:supervisor.HasExited) {
            Stop-Process -Id $script:supervisor.Id -Force -ErrorAction SilentlyContinue
            $script:supervisor.WaitForExit(15000) | Out-Null
        }
        if ($script:installDir) {
            Get-PackageProcesses | ForEach-Object {
                Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
            }
        }
    } catch {
        Write-Warning "cleanup: process stop failed: $_"
    }
    $uninstaller = $null
    if (Test-Path -LiteralPath $install) {
        $uninstaller = Get-ChildItem -Path $install -Recurse -File -Filter "uninstall.exe" -ErrorAction SilentlyContinue |
            Select-Object -First 1
    }
    if ($uninstaller) {
        try {
            $process = Start-Process -FilePath $uninstaller.FullName -ArgumentList @("/S") -Wait -PassThru
            if ($process.ExitCode -ne 0) {
                Write-Warning "uninstall exited with code $($process.ExitCode)"
            }
        } catch {
            Write-Warning "uninstall failed: $_"
        }
    }
    if (-not $Keep -and $createdTemp) {
        foreach ($attempt in 1..20) {
            try {
                Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction Stop
                break
            } catch {
                Start-Sleep -Milliseconds 250
            }
        }
    } elseif ($Keep) {
        Write-Host "package-parity: evidence kept in $work"
    }
}

# Check the contents of a Windows desktop package.
#
# The NSIS installer is the only Windows artifact. It must carry the shell, the
# supervisor, the six .exe service binaries, the built client, and no
# interpreter. This is a static check: it does not start the profile or run a
# backtest.
#
# Usage:
#   check-package.ps1 -Artifact path\to\QuantRule_<version>_x64-setup.exe
#   check-package.ps1 -Tree <installed directory>   # a CI or local tree
param(
    [string]$Artifact,
    [string]$Tree
)

$ErrorActionPreference = "Stop"
$services = @("qr-gateway", "qr-data", "qr-model", "qr-backtest", "qr-evidence", "qr-simulation", "qr-desktop")
$status = 0
$install = $null
$createdTemp = $false

function Fail([string]$message) {
    Write-Error "package: $message" -ErrorAction Continue
    $script:status = 1
}

if (-not $Tree) {
    if (-not $Artifact -or -not (Test-Path -LiteralPath $Artifact)) {
        Write-Error "usage: check-package.ps1 -Artifact <setup.exe> | -Tree <directory>" -ErrorAction Continue
        exit 2
    }
    $temp = Join-Path $env:RUNNER_TEMP ("qr-package-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $temp | Out-Null
    $createdTemp = $true
    $install = Join-Path $temp "install"
    # NSIS is silent with /S and installs under /D=; /D= must be the last
    # argument and must not be quoted.
    $process = Start-Process -FilePath $Artifact -ArgumentList @("/S", "/D=$install") -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        Write-Error "package: the installer exited with code $($process.ExitCode)" -ErrorAction Continue
        exit 1
    }
    $Tree = $install
}

try {
    $shell = Get-ChildItem -Path $Tree -Recurse -File -Filter "quantrule-desktop-shell.exe" | Select-Object -First 1
    if (-not $shell) { Fail "the shell binary is missing" }

    $sidecarDir = Get-ChildItem -Path $Tree -Recurse -Directory -Filter "sidecars" | Select-Object -First 1
    if (-not $sidecarDir) {
        Fail "the sidecars directory is missing"
    } else {
        foreach ($name in $services) {
            $found = Get-ChildItem -Path $sidecarDir.FullName -File -Filter "$name.exe" | Select-Object -First 1
            if (-not $found) { Fail "sidecars\$name.exe is missing" }
        }
        $stub = Get-ChildItem -Path $sidecarDir.FullName -File -Filter "stub-sidecar*" | Select-Object -First 1
        if ($stub) { Fail "the test stub is packaged" }
    }

    $client = Get-ChildItem -Path $Tree -Recurse -File -Filter "index.html" |
        Where-Object { $_.DirectoryName -match "[\\/]client$" } |
        Select-Object -First 1
    if (-not $client) { Fail "the built client (client\index.html) is missing" }

    $interpreters = Get-ChildItem -Path $Tree -Recurse -File |
        Where-Object { $_.BaseName -match '^(py|bun|node)' -or $_.Extension -eq ".py" }
    if ($interpreters) {
        Fail "the package carries an interpreter: $($interpreters.Name -join ', ')"
    }

    if ($status -ne 0) {
        exit 1
    }
    Write-Output "package ok: shell, supervisor, six sidecars, client, no interpreter"
}
finally {
    if ($createdTemp) {
        $uninstall = Get-ChildItem -Path $Tree -Recurse -File -Filter "uninstall.exe" -ErrorAction SilentlyContinue |
            Select-Object -First 1
        if ($uninstall) {
            Start-Process -FilePath $uninstall.FullName -ArgumentList "/S" -Wait | Out-Null
        }
    }
    if ($createdTemp -and $temp -and (Test-Path -LiteralPath $temp)) {
        Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

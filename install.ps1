# The manifest's [[build]] on Windows (ADR-599b6f424271), install.sh's
# counterpart under the same contract: puts the release binary in
# .\bin\herdr-ank.exe, checked against SHA256SUMS, and falls back to
# cargo build --release only when that fails and cargo is present.
#
# Windows PowerShell 5.1 is the floor: it is what a clean Windows ships, so
# every cmdlet here is one 5.1 carries, and TLS 1.2 is turned on explicitly
# because 5.1 still defaults to protocols GitHub refuses.
#
# HERDR_ANK_RELEASE_BASE replaces the release URL; it may also name a local
# directory holding the archive and SHA256SUMS.

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol =
    [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

Set-Location -LiteralPath $PSScriptRoot

function Fail([string]$Message) {
    [Console]::Error.WriteLine("herdr-ank: $Message")
}

$version = $null
foreach ($line in Get-Content -LiteralPath 'herdr-plugin.toml') {
    if ($line -match '^version\s*=\s*"(.*)"') {
        $version = $Matches[1]
        break
    }
}
if (-not $version) {
    Fail 'no version in herdr-plugin.toml'
    exit 1
}

$base = $env:HERDR_ANK_RELEASE_BASE
if (-not $base) {
    $base = "https://github.com/haksolot/herdr-ank/releases/download/v$version"
}

# A 32-bit PowerShell on a 64-bit Windows reports x86 here and the machine's
# architecture in PROCESSOR_ARCHITEW6432.
$arch = $env:PROCESSOR_ARCHITEW6432
if (-not $arch) { $arch = $env:PROCESSOR_ARCHITECTURE }
switch ($arch) {
    'AMD64' { $target = 'x86_64-pc-windows-msvc' }
    default { $target = $null }
}

# Copies from a local directory, downloads otherwise.
function Fetch([string]$Name, [string]$Destination) {
    if (Test-Path -LiteralPath $base -PathType Container) {
        Copy-Item -LiteralPath (Join-Path $base $Name) -Destination $Destination
    } else {
        Invoke-WebRequest -UseBasicParsing -Uri "$base/$Name" -OutFile $Destination
    }
}

# Downloads, checks and unpacks the release archive; $false on any failure.
function Download([string]$Asset) {
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ([IO.Path]::GetRandomFileName())
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        $archive = Join-Path $tmp $Asset
        $sums = Join-Path $tmp 'SHA256SUMS'
        try {
            Fetch $Asset $archive
            Fetch 'SHA256SUMS' $sums
        } catch {
            Fail "$($_.Exception.Message)"
            return $false
        }
        $expected = $null
        foreach ($line in Get-Content -LiteralPath $sums) {
            $fields = $line -split '\s+', 2
            if ($fields.Count -eq 2 -and ($fields[1] -eq $Asset -or $fields[1] -eq "*$Asset")) {
                $expected = $fields[0]
                break
            }
        }
        $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash
        if (-not $expected -or $expected -ne $actual) {
            Fail "$Asset does not match its sum in $base/SHA256SUMS"
            return $false
        }
        $unpacked = Join-Path $tmp 'unpacked'
        try {
            Expand-Archive -LiteralPath $archive -DestinationPath $unpacked
        } catch {
            Fail "$Asset could not be unpacked: $($_.Exception.Message)"
            return $false
        }
        $exe = Join-Path $unpacked 'herdr-ank.exe'
        if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
            Fail "$Asset holds no herdr-ank.exe"
            return $false
        }
        New-Item -ItemType Directory -Force -Path 'bin' | Out-Null
        Copy-Item -LiteralPath $exe -Destination 'bin\herdr-ank.exe' -Force
        return $true
    } finally {
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if ($target) {
    $asset = "herdr-ank-$version-$target.zip"
    $url = "$base/$asset"
    if (Download $asset) {
        exit 0
    }
    $tried = "$url could not be downloaded and checked"
} else {
    $tried = "no release binary for architecture '$arch'"
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Fail "$tried, and cargo is not on PATH to build from source"
    exit 1
}
Fail "$tried; building with cargo"
& cargo build --release
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
New-Item -ItemType Directory -Force -Path 'bin' | Out-Null
Copy-Item -LiteralPath 'target\release\herdr-ank.exe' -Destination 'bin\herdr-ank.exe' -Force
exit 0

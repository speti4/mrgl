<#
.SYNOPSIS
Builds the release exes and packs the portable zip: dist\mrgl-<version>.zip, with its checksum and its
release notes next to it.

.DESCRIPTION
The version is the workspace Cargo.toml one, the only place it is written. The zip holds mrgl.exe (the
command line), mrgl-app.exe (the window) and THIRD-PARTY-LICENSES.html (cargo-about over the crates compiled
into the exes, with about.toml and scripts\third-party-licenses.hbs). Next to the zip go SHA256SUMS.txt (the
zip's SHA-256 in `sha256sum -c` format) and release-notes-<version>.md (scripts\changelog.py --notes, the
body of the GitHub release). CI runs this script on a v<version> tag with -Tag set, which must match the
version; by hand the tag check is skipped. Needs cargo, cargo-about in the version
scripts\cargo-about-version.txt names (cargo install cargo-about --version (Get-Content
scripts\cargo-about-version.txt) --locked --features cli), Python 3 and git with the release tags.

.EXAMPLE
scripts\release.ps1
scripts\release.ps1 -SkipBuild -Out C:\tmp\dist
#>
[CmdletBinding()]
param(
    # Pack what target\release already holds instead of building.
    [switch] $SkipBuild,
    # Where the zip, the checksum file and the notes go.
    [string] $Out = "dist",
    # The git tag being released (CI); must equal "v<version>".
    [string] $Tag = ""
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$cargoToml = Get-Content (Join-Path $root "Cargo.toml") -Raw
if ($cargoToml -notmatch '(?m)^\[workspace\.package\][^\[]*?^version = "([^"]+)"') {
    throw "no [workspace.package] version in Cargo.toml"
}
$version = $Matches[1]
if ($Tag -and $Tag -ne "v$version") {
    throw "tag '$Tag' does not match the Cargo.toml version 'v$version'"
}

# A tool from PATH, else from ~\.cargo\bin (a shell started before rustup's installer changed PATH lacks it).
function Find-Tool([string] $Name) {
    $tool = Get-Command $Name -ErrorAction SilentlyContinue
    if (-not $tool) {
        $fallback = Join-Path $env:USERPROFILE ".cargo\bin\$Name.exe"
        if (Test-Path $fallback) { $tool = Get-Command $fallback } else { throw "$Name not found" }
    }
    $tool.Source
}

# What can fail fast runs before the build: the tools and the release notes.
$cargo = Find-Tool "cargo"
$cargoAbout = Find-Tool "cargo-about"
# The cargo-about CI installs: scripts\cargo-about-version.txt is the one place its version is written, so a
# page rendered here is the page a release renders.
$aboutWanted = (Get-Content (Join-Path $root "scripts\cargo-about-version.txt") -Raw).Trim()
$aboutFound = "$(& $cargoAbout --version)".Trim() -replace '^cargo-about\s+', ''
if ($aboutFound -ne $aboutWanted) {
    throw "cargo-about $aboutFound found, $aboutWanted wanted: cargo install cargo-about --version $aboutWanted --locked --features cli"
}
# Python as `python`, else through the `py` launcher, which is all some Windows installs put on PATH.
$python = (Get-Command python -ErrorAction SilentlyContinue).Source
$pythonArgs = @()
if (-not $python) {
    $python = (Get-Command py -ErrorAction SilentlyContinue).Source
    $pythonArgs = @("-3")
}
if (-not $python) { throw "python not found (neither python nor py on PATH)" }

New-Item -ItemType Directory -Force $Out | Out-Null
$outDir = (Resolve-Path $Out).Path
$notes = Join-Path $outDir "release-notes-$version.md"
& $python @pythonArgs -I (Join-Path $root "scripts\changelog.py") --notes $version --out $notes
if ($LASTEXITCODE -ne 0) { throw "release notes failed ($LASTEXITCODE)" }

if (-not $SkipBuild) {
    & $cargo build --release --locked -p mrgl-cli -p mrgl-app
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
}

$cli = Join-Path $root "target\release\mrgl.exe"
$app = Join-Path $root "target\release\mrgl-app.exe"
foreach ($file in @($cli, $app)) {
    if (-not (Test-Path $file)) { throw "missing: $file" }
}

# tauri-build stamps the workspace version into the window exe's properties; a stale build shows here.
$stamped = (Get-Item $app).VersionInfo.ProductVersion
if ($stamped -ne $version) {
    throw "the app exe carries version '$stamped', Cargo.toml says '$version' (stale build?)"
}

$stage = Join-Path ([System.IO.Path]::GetTempPath()) "mrgl-release-$version"
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item $cli (Join-Path $stage "mrgl.exe")
Copy-Item $app (Join-Path $stage "mrgl-app.exe")

# The notices of the crates compiled into the exes (their MIT, Apache-2.0, BSD and MPL-2.0 licenses ask for
# them). cargo-about reads the graph through `cargo metadata`, which wants the sources of every platform's
# crates, more than the build fetched, so `cargo fetch` (all targets) runs first; then --frozen: Cargo.lock as
# it is and no network, so the same lock file gives the same page. The app crate's graph holds the CLI's.
& $cargo fetch --locked
if ($LASTEXITCODE -ne 0) { throw "cargo fetch failed ($LASTEXITCODE)" }
$licenses = Join-Path $stage "THIRD-PARTY-LICENSES.html"
& $cargoAbout generate --frozen --fail `
    --manifest-path (Join-Path $root "Cargo.toml") `
    --workspace `
    --config (Join-Path $root "about.toml") `
    --output-file $licenses `
    (Join-Path $root "scripts\third-party-licenses.hbs")
if ($LASTEXITCODE -ne 0) { throw "cargo-about failed ($LASTEXITCODE)" }
$licensesKb = [math]::Round((Get-Item $licenses).Length / 1KB)

$zip = Join-Path $outDir "mrgl-$version.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip -CompressionLevel Optimal
Remove-Item -Recurse -Force $stage

# "<sha256>  <file name>" and a LF, as sha256sum writes it, so `sha256sum -c SHA256SUMS.txt` checks the
# download; Get-FileHash prints the same value in capitals.
$hash = (Get-FileHash -Algorithm SHA256 $zip).Hash.ToLowerInvariant()
$sums = Join-Path $outDir "SHA256SUMS.txt"
[System.IO.File]::WriteAllText($sums, "$hash  mrgl-$version.zip`n", (New-Object System.Text.UTF8Encoding $false))

$cliMb = [math]::Round((Get-Item $cli).Length / 1MB, 1)
$appMb = [math]::Round((Get-Item $app).Length / 1MB, 1)
$zipMb = [math]::Round((Get-Item $zip).Length / 1MB, 1)
Write-Host "version:  $version (app exe properties: $stamped)"
Write-Host "cli:      $cli ($cliMb MB)"
Write-Host "app:      $app ($appMb MB)"
Write-Host "zip:      $zip ($zipMb MB; THIRD-PARTY-LICENSES.html $licensesKb KB inside)"
Write-Host "sha256:   $hash ($sums)"
Write-Host "notes:    $notes"

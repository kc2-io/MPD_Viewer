$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Rust/Cargo is required. See README.md for Windows prerequisites.'
}
cargo run -p mpd-tabber --features custom-protocol
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

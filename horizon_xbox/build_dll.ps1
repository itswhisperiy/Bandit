$ErrorActionPreference = "Stop"
rustup update stable
if (-not (Test-Path horizon-src)) {
    git clone --depth 1 https://github.com/horizon-rust/horizon.git horizon-src
    if ($LASTEXITCODE -ne 0) { throw "git clone failed" }
}
$crate = "horizon-src\crates\horizon-xbox"
New-Item -ItemType Directory -Force "$crate\src" | Out-Null
Copy-Item horizon_xbox\src\*.rs "$crate\src\" -Force
$toml = [System.IO.File]::ReadAllText("$PWD\horizon_xbox\Cargo.toml")
$toml = $toml.Replace("[dependencies]", "[dependencies]`nhorizon-client = { path = `"../horizon-client`", default-features = false }")
[System.IO.File]::WriteAllText("$PWD\$crate\Cargo.toml", $toml)
$root = "horizon-src\Cargo.toml"
$t = Get-Content $root -Raw
if ($t -notmatch '"crates/\*"' -and $t -notmatch 'crates/horizon-xbox') {
    $t = $t -replace 'members\s*=\s*\[', "members = [`n  `"crates/horizon-xbox`","
    Set-Content $root $t
}
$env:RUSTFLAGS = "-C target-feature=+crt-static --cfg horizon_linked -C link-arg=/DELAYLOAD:uxtheme.dll -C link-arg=delayimp.lib"
Push-Location horizon-src
cargo build --release -p horizon_xbox
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "cargo build failed" }
Pop-Location
Copy-Item horizon-src\target\release\horizon_xbox.dll horizon_xbox.dll -Force

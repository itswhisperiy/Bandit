$ErrorActionPreference = "Stop"
rustup update stable
if (-not (Test-Path horizon-src)) {
    git clone --depth 1 https://github.com/horizon-rust/horizon.git horizon-src
    if ($LASTEXITCODE -ne 0) { throw "git clone failed" }
}

# Engine DLL: Horizon's client in a small cdylib, built inside Horizon's workspace
$crate = "horizon-src\crates\horizon-engine"
New-Item -ItemType Directory -Force "$crate\src" | Out-Null
@'
[package]
name = "horizon-engine"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
horizon-client = { path = "../horizon-client", default-features = false }
'@ | Set-Content "$crate\Cargo.toml"
@'
#[no_mangle]
pub extern "system" fn horizon_engine_probe() -> usize {
    let f: fn() = horizon_client::entry::main;
    f as usize
}
'@ | Set-Content "$crate\src\lib.rs"

$root = "horizon-src\Cargo.toml"
$t = Get-Content $root -Raw
if ($t -notmatch '"crates/\*"' -and $t -notmatch 'crates/horizon-engine') {
    $t = $t -replace 'members\s*=\s*\[', "members = [`n  `"crates/horizon-engine`","
    Set-Content $root $t
}

$env:RUSTFLAGS = "-C target-feature=+crt-static -C link-arg=/DELAYLOAD:uxtheme.dll -C link-arg=/DELAYLOAD:user32.dll -C link-arg=delayimp.lib"
Push-Location horizon-src
cargo build --release -p horizon-engine
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "engine build failed" }
Pop-Location

# List every function the engine imports, for the stub to check on the Xbox
$vs = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -latest -property installationPath
$d = Get-ChildItem "$vs\VC\Tools\MSVC" -Recurse -Filter dumpbin.exe | Where-Object { $_.FullName -like '*Hostx64\x64*' } | Select-Object -First 1
$out = & $d.FullName /imports horizon-src\target\release\horizon_engine.dll
$dll = $null
$lines = @()
foreach ($l in $out) {
    if ($l -match 'delay load imports') { break }
    if ($l -match '^\s{4}(\S+\.dll)\s*$') { $dll = $Matches[1]; continue }
    if ($dll -and $l -match '^\s+[0-9A-Fa-f]+\s+([A-Za-z_][A-Za-z0-9_@?$]*)\s*$') { $lines += "$dll!$($Matches[1])" }
}
$lines | Sort-Object -Unique | Set-Content horizon_xbox\src\imports.txt
Write-Host "Engine imports listed: $($lines.Count)"

# Stub DLL (small, known to load)
$env:RUSTFLAGS = "-C target-feature=+crt-static"
cargo build --release --manifest-path horizon_xbox\Cargo.toml
if ($LASTEXITCODE -ne 0) { throw "stub build failed" }

Copy-Item horizon_xbox\target\release\horizon_xbox.dll horizon_xbox.dll -Force
Copy-Item horizon-src\target\release\horizon_engine.dll horizon_engine.dll -Force

# Make build.ps1 package both DLLs
$bp = Join-Path $PWD "build.ps1"
$b = [System.IO.File]::ReadAllText($bp)
$needle = 'Copy-Item $mcExe (Join-Path $pkg "MC.Xbox.exe")'
if (-not $b.Contains('horizon_xbox.dll')) {
    $n = ([regex]::Matches($b, [regex]::Escape($needle))).Count
    if ($n -ne 1) { throw "build.ps1: expected one MC.Xbox.exe copy line, found $n" }
    $add = $needle + "`r`n" + 'Copy-Item (Join-Path $root "horizon_xbox.dll") (Join-Path $pkg "horizon_xbox.dll")' + "`r`n" + 'Copy-Item (Join-Path $root "horizon_engine.dll") (Join-Path $pkg "horizon_engine.dll")'
    [System.IO.File]::WriteAllText($bp, $b.Replace($needle, $add), (New-Object System.Text.UTF8Encoding($true)))
    Write-Host "build.ps1 patched"
}

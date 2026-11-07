$ErrorActionPreference = "Stop"
rustup update stable
if (-not (Test-Path horizon-src)) {
    git clone --depth 1 https://github.com/horizon-rust/horizon.git horizon-src
    if ($LASTEXITCODE -ne 0) { throw "git clone failed" }
}
.\horizon_xbox\patch_wgpu.ps1

$crate = "horizon-src\crates\horizon-engine"
New-Item -ItemType Directory -Force "$crate\src" | Out-Null
Copy-Item horizon_xbox\engine\Cargo.toml.txt "$crate\Cargo.toml" -Force
Copy-Item horizon_xbox\engine\lib.rs.txt "$crate\src\lib.rs" -Force

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

$env:RUSTFLAGS = "-C target-feature=+crt-static"
cargo build --release --manifest-path horizon_xbox\Cargo.toml
if ($LASTEXITCODE -ne 0) { throw "stub build failed" }

Copy-Item horizon_xbox\target\release\horizon_xbox.dll horizon_xbox.dll -Force
Copy-Item horizon-src\target\release\horizon_engine.dll horizon_engine.dll -Force

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

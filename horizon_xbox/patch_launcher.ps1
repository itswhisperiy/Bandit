$ErrorActionPreference = "Stop"

# 1. build.ps1: copy the Rust DLL into the package
$bp = Join-Path $PWD "build.ps1"
$b = [System.IO.File]::ReadAllText($bp)
$needle = 'Copy-Item $mcExe (Join-Path $pkg "MC.Xbox.exe")'
$dllLine = 'Copy-Item (Join-Path $root "horizon_xbox.dll") (Join-Path $pkg "horizon_xbox.dll")'
if (-not $b.Contains('horizon_xbox.dll')) {
    $n = ([regex]::Matches($b, [regex]::Escape($needle))).Count
    if ($n -ne 1) { throw "build.ps1: expected one MC.Xbox.exe copy line, found $n" }
    $b = $b.Replace($needle, $needle + "`r`n" + $dllLine)
    [System.IO.File]::WriteAllText($bp, $b, (New-Object System.Text.UTF8Encoding($true)))
    Write-Host "build.ps1 patched"
} else {
    Write-Host "build.ps1 already has the DLL line"
}

# 2. minecraft_launch.cpp: run the Rust DLL once, on first Play after install
$path = Join-Path $PWD "MC.Xbox\launch\minecraft_launch.cpp"
$text = [System.IO.File]::ReadAllText($path)
if ($text.Contains('RunHorizonXbox')) {
    Write-Host "minecraft_launch.cpp already patched"
    return
}

$fnAnchor = 'bool RunEmbeddedMinecraft(const std::wstring& exeDir,'
$callAnchor = 'const LoaderId loaderId = ParseLoaderId(loader);'
foreach ($a in @($fnAnchor, $callAnchor)) {
    $count = ([regex]::Matches($text, [regex]::Escape($a))).Count
    if ($count -ne 1) { throw "minecraft_launch.cpp: expected one match for [$a], found $count" }
}

$fn = @'
typedef int (__cdecl* HorizonXboxRun_t)(const wchar_t* logDir);

static bool RunHorizonXbox(const std::wstring& logDir) {
    HMODULE module = LoadPackagedLibrary(L"horizon_xbox.dll", 0);
    if (!module) {
        WriteLogF(L"LoadPackagedLibrary(horizon_xbox.dll) failed err=%u", GetLastError());
        return false;
    }
    auto run = reinterpret_cast<HorizonXboxRun_t>(GetProcAddress(module, "horizon_xbox_run"));
    if (!run) {
        WriteLogF(L"horizon_xbox_run not found err=%u", GetLastError());
        return false;
    }
    WriteLog(L"Calling horizon_xbox_run");
    const int rc = run(logDir.c_str());
    WriteLogF(L"horizon_xbox_run returned %d", rc);
    return rc == 0;
}

'@

$call = @'
{
        EnsureDirectoryTree(gameDir);
        const std::wstring horizonMarker = gameDir + L"\\horizon_test_done.txt";
        if (GetFileAttributesW(horizonMarker.c_str()) == INVALID_FILE_ATTRIBUTES) {
            WriteTextFile(horizonMarker, L"Horizon test ran once. Delete this file to run it again.\n");
            const std::wstring horizonLogDir = LogsCurrentDir(exeDir);
            EnsureDirectoryTree(horizonLogDir);
            WriteLog(L"First launch after install: running the Rust DLL instead of Java");
            const bool horizonOk = RunHorizonXbox(horizonLogDir);
            g_minecraftRunning.store(false);
            return horizonOk;
        }
    }

    
'@

$text = $text.Replace($fnAnchor, $fn + $fnAnchor)
$text = $text.Replace($callAnchor, $call + $callAnchor)
[System.IO.File]::WriteAllText($path, $text, (New-Object System.Text.UTF8Encoding($true)))
Write-Host "minecraft_launch.cpp patched"

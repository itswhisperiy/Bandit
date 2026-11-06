$ErrorActionPreference = "Stop"
$path = Join-Path $PWD "MC.Xbox\launch\minecraft_launch.cpp"
$text = [System.IO.File]::ReadAllText($path)

$fnAnchor = 'bool RunEmbeddedMinecraft(const std::wstring& exeDir,'
$callAnchor = 'const LoaderId loaderId = ParseLoaderId(loader);'

foreach ($a in @($fnAnchor, $callAnchor)) {
    $count = ([regex]::Matches($text, [regex]::Escape($a))).Count
    if ($count -ne 1) { throw "Expected exactly one match for [$a] but found $count" }
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
if (DirectoryExists(gameDir + L"\\horizon_test")) {
        const std::wstring horizonLogDir = LogsCurrentDir(exeDir);
        EnsureDirectoryTree(horizonLogDir);
        WriteLog(L"horizon_test folder found: running the Rust DLL instead of Java");
        const bool horizonOk = RunHorizonXbox(horizonLogDir);
        g_minecraftRunning.store(false);
        return horizonOk;
    }

    
'@

$text = $text.Replace($fnAnchor, $fn + $fnAnchor)
$text = $text.Replace($callAnchor, $call + $callAnchor)
[System.IO.File]::WriteAllText($path, $text, (New-Object System.Text.UTF8Encoding($true)))
Write-Host "minecraft_launch.cpp patched for Horizon test"

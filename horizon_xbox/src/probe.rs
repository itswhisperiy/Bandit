use std::fs::OpenOptions;
use std::io::Write;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::LibraryLoader::{LoadLibraryExW, LOAD_LIBRARY_SEARCH_SYSTEM32};

pub fn probe_dlls(dir: &str) {
    let names = [
        "user32.dll", "gdi32.dll", "opengl32.dll", "setupapi.dll", "imm32.dll",
        "dwmapi.dll", "uxtheme.dll", "iphlpapi.dll", "shell32.dll", "advapi32.dll",
        "bcrypt.dll", "bcryptprimitives.dll", "ws2_32.dll", "dxgi.dll", "d3d12.dll",
        "d3dcompiler_47.dll", "dxcompiler.dll",
    ];
    let path = format!("{}\\horizon_xbox.log", dir);
    for n in names {
        let wide: Vec<u16> = n.encode_utf16().chain(std::iter::once(0)).collect();
        let r = unsafe {
            LoadLibraryExW(PCWSTR(wide.as_ptr()), HANDLE::default(), LOAD_LIBRARY_SEARCH_SYSTEM32)
        };
        let line = match r {
            Ok(_) => format!("dll {n}: OK"),
            Err(e) => format!("dll {n}: MISSING ({e})"),
        };
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f, "{}", line);
        }
    }
}

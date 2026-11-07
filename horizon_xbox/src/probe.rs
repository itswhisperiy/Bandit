use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use windows::core::{PCSTR, PCWSTR};
use windows::Win32::Foundation::{HANDLE, HMODULE};
use windows::Win32::System::LibraryLoader::{
    GetProcAddress, LoadLibraryExW, LoadPackagedLibrary, LOAD_LIBRARY_SEARCH_SYSTEM32,
};

fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn put(dir: &str, line: &str) {
    let path = format!("{}\\horizon_xbox.log", dir);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{}", line);
    }
}

pub fn probe_dlls(dir: &str) {
    let names = [
        "user32.dll", "gdi32.dll", "opengl32.dll", "setupapi.dll", "imm32.dll",
        "dwmapi.dll", "uxtheme.dll", "iphlpapi.dll", "shell32.dll", "advapi32.dll",
        "bcrypt.dll", "bcryptprimitives.dll", "ws2_32.dll", "dxgi.dll", "d3d12.dll",
        "d3dcompiler_47.dll", "dxcompiler.dll",
    ];
    for n in names {
        let wide = w(n);
        let r = unsafe {
            LoadLibraryExW(PCWSTR(wide.as_ptr()), HANDLE::default(), LOAD_LIBRARY_SEARCH_SYSTEM32)
        };
        match r {
            Ok(_) => put(dir, &format!("dll {n}: OK")),
            Err(e) => put(dir, &format!("dll {n}: MISSING ({e})")),
        }
    }
}

pub fn probe_imports(dir: &str) {
    let list = include_str!("imports.txt");
    let mut libs: HashMap<String, Option<HMODULE>> = HashMap::new();
    let (mut total, mut missing) = (0u32, 0u32);
    for line in list.lines() {
        let Some((dll, func)) = line.trim().split_once('!') else { continue };
        total += 1;
        let h = *libs.entry(dll.to_lowercase()).or_insert_with(|| {
            let wide = w(dll);
            unsafe {
                LoadLibraryExW(PCWSTR(wide.as_ptr()), HANDLE::default(), LOAD_LIBRARY_SEARCH_SYSTEM32).ok()
            }
        });
        match h {
            None => {
                missing += 1;
                put(dir, &format!("import {dll}!{func}: DLL MISSING"));
            }
            Some(m) => {
                let name = format!("{func}\0");
                let p = unsafe { GetProcAddress(m, PCSTR(name.as_ptr())) };
                if p.is_none() {
                    missing += 1;
                    put(dir, &format!("import {dll}!{func}: MISSING"));
                }
            }
        }
    }
    put(dir, &format!("imports checked: {total}, missing: {missing}"));
}

pub fn try_load_engine(dir: &str) {
    let wide = w("horizon_engine.dll");
    match unsafe { LoadPackagedLibrary(PCWSTR(wide.as_ptr()), 0) } {
        Ok(_) => put(dir, "horizon_engine.dll: LOADED"),
        Err(e) => put(dir, &format!("horizon_engine.dll: FAILED ({e})")),
    }
}

use std::cell::RefCell;
use std::ffi::c_void;
use std::fs::OpenOptions;
use std::io::Write;

use windows::core::{Interface, HSTRING, PCSTR, PCWSTR};
use windows::ApplicationModel::Core::CoreApplication;
use windows::UI::Core::{CoreProcessEventsOption, CoreWindow};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadPackagedLibrary};

thread_local! {
    static WIN: RefCell<Option<CoreWindow>> = const { RefCell::new(None) };
}

unsafe fn read_wide(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
}

fn log(dir: &str, msg: &str) {
    let path = format!("{}\\horizon_xbox.log", dir);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{}", msg);
    }
}

fn get_core_window() -> windows::core::Result<CoreWindow> {
    let props = CoreApplication::Properties()?;
    let obj = props.Lookup(&HSTRING::from("EGLNativeWindowTypeProperty"))?;
    obj.cast::<CoreWindow>()
}

/// Called by the engine every frame. Returns 1 if it ran on the UI thread and pumped events.
unsafe extern "system" fn pump() -> i32 {
    WIN.with(|w| {
        if let Some(win) = w.borrow().as_ref() {
            if let Ok(d) = win.Dispatcher() {
                if d.HasThreadAccess().unwrap_or(false) {
                    let _ = d.ProcessEvents(CoreProcessEventsOption::ProcessAllIfPresent);
                    return 1;
                }
            }
        }
        0
    })
}

type GpuTest = unsafe extern "system" fn(*mut c_void, u32, u32, *const u16, usize) -> i32;

#[no_mangle]
pub unsafe extern "system" fn horizon_xbox_run(local_state: *const u16) -> i32 {
    let dir = read_wide(local_state);
    log(&dir, "horizon_xbox_run called");
    let window = match get_core_window() {
        Ok(w) => w,
        Err(e) => {
            log(&dir, &format!("no CoreWindow: {e}"));
            return 1;
        }
    };
    let b = match window.Bounds() {
        Ok(b) => b,
        Err(e) => {
            log(&dir, &format!("Bounds() failed: {e}"));
            return 2;
        }
    };
    log(&dir, &format!("bounds: {}x{}", b.Width, b.Height));
    WIN.with(|w| *w.borrow_mut() = Some(window.clone()));

    let name: Vec<u16> = "horizon_engine.dll".encode_utf16().chain(Some(0)).collect();
    let module = match LoadPackagedLibrary(PCWSTR(name.as_ptr()), 0) {
        Ok(m) => m,
        Err(e) => {
            log(&dir, &format!("horizon_engine.dll: FAILED ({e})"));
            return 4;
        }
    };
    log(&dir, "horizon_engine.dll: LOADED");
    let Some(f) = GetProcAddress(module, PCSTR(b"horizon_engine_gpu_test\0".as_ptr())) else {
        log(&dir, "horizon_engine_gpu_test not found");
        return 5;
    };
    let f: GpuTest = std::mem::transmute(f);
    let rc = f(window.as_raw(), b.Width as u32, b.Height as u32, local_state, pump as usize);
    log(&dir, &format!("gpu test returned {rc}"));
    WIN.with(|w| *w.borrow_mut() = None);
    rc
}

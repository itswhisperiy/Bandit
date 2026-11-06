use std::fs::OpenOptions;
use std::io::Write;
use windows::core::{Interface, HSTRING};
use windows::ApplicationModel::Core::CoreApplication;
use windows::UI::Core::CoreWindow;

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

#[no_mangle]
pub unsafe extern "system" fn horizon_xbox_run(local_state: *const u16) -> i32 {
    let dir = read_wide(local_state);
    log(&dir, "horizon_xbox_run called");
    match get_core_window() {
        Ok(window) => {
            log(&dir, "got CoreWindow");
            match window.Bounds() {
                Ok(b) => log(&dir, &format!("bounds: {}x{}", b.Width, b.Height)),
                Err(e) => log(&dir, &format!("Bounds() failed: {e}")),
            }
        }
        Err(e) => {
            log(&dir, &format!("no CoreWindow: {e}"));
            return 1;
        }
    }
    0
}

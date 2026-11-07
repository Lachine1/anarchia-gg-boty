// patched for anarchia-boty: resolve WebView2Loader.dll at runtime instead of an import-table
// dependency, so the exe starts without the DLL next to it. Path: $WEBVIEW2_LOADER, else search path.
use std::sync::OnceLock;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> isize;
    fn GetProcAddress(m: isize, name: *const u8) -> *const core::ffi::c_void;
}

fn module() -> isize {
    static M: OnceLock<isize> = OnceLock::new();
    *M.get_or_init(|| {
        let path = std::env::var("WEBVIEW2_LOADER").unwrap_or_else(|_| "WebView2Loader.dll".into());
        let w: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        unsafe { LoadLibraryW(w.as_ptr()) }
    })
}

pub(crate) fn get(name: &[u8]) -> Option<*const core::ffi::c_void> {
    let m = module();
    if m == 0 {
        return None;
    }
    let p = unsafe { GetProcAddress(m, name.as_ptr()) };
    (!p.is_null()).then_some(p)
}

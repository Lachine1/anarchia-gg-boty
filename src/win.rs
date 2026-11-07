// Copyright (c) 2026 lachine. AGPL v3 + Commons Clause - see LICENSE.md
//! Windows-only helpers (no extra crates: raw kernel32/psapi bindings).
#![allow(non_snake_case)]

#[repr(C)]
#[derive(Default)]
struct PROCESS_MEMORY_COUNTERS {
    cb: u32,
    PageFaultCount: u32,
    PeakWorkingSetSize: usize,
    WorkingSetSize: usize,
    QuotaPeakPagedPoolUsage: usize,
    QuotaPagedPoolUsage: usize,
    QuotaPeakNonPagedPoolUsage: usize,
    QuotaNonPagedPoolUsage: usize,
    PagefileUsage: usize,
    PeakPagefileUsage: usize,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentProcess() -> isize;
    fn K32GetProcessMemoryInfo(h: isize, p: *mut PROCESS_MEMORY_COUNTERS, cb: u32) -> i32;
    fn GetConsoleWindow() -> isize;
    fn GetConsoleProcessList(list: *mut u32, count: u32) -> u32;
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
    fn GetExitCodeProcess(h: isize, code: *mut u32) -> i32;
    fn CloseHandle(h: isize) -> i32;
}

pub fn process_alive(pid: u32) -> bool {
    unsafe {
        let h = OpenProcess(0x1000, 0, pid); // PROCESS_QUERY_LIMITED_INFORMATION
        if h == 0 {
            return false;
        }
        let mut code = 0u32;
        let ok = GetExitCodeProcess(h, &mut code) != 0;
        CloseHandle(h);
        ok && code == 259 // STILL_ACTIVE
    }
}
#[link(name = "user32")]
extern "system" {
    fn ShowWindow(h: isize, cmd: i32) -> i32;
}

pub fn rss_kb() -> u64 {
    let mut c = PROCESS_MEMORY_COUNTERS { cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32, ..Default::default() };
    unsafe {
        if K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) != 0 {
            return (c.WorkingSetSize / 1024) as u64;
        }
    }
    0
}

/// Double-clicked exe: the console belongs only to us → hide it (the app window is the UI).
/// Started from cmd/PowerShell: leave the console alone.
pub fn hide_console_if_owned() {
    unsafe {
        let mut list = [0u32; 4];
        if GetConsoleProcessList(list.as_mut_ptr(), 4) <= 1 {
            let h = GetConsoleWindow();
            if h != 0 {
                ShowWindow(h, 0);
            }
        }
    }
}

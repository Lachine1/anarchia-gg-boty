// Copyright (c) 2026 lachine. AGPL v3 + Commons Clause - see LICENSE.md
//! Desktop window: the panel runs as a chromeless app window (Edge on Windows, Chrome/Chromium on Linux,
//! default browser as fallback). Closing the window stops the bots.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn candidates() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    #[cfg(windows)]
    {
        for base in ["ProgramFiles(x86)", "ProgramFiles", "LOCALAPPDATA"] {
            if let Ok(b) = std::env::var(base) {
                v.push(PathBuf::from(&b).join("Microsoft\\Edge\\Application\\msedge.exe"));
                v.push(PathBuf::from(&b).join("Google\\Chrome\\Application\\chrome.exe"));
                v.push(PathBuf::from(&b).join("BraveSoftware\\Brave-Browser\\Application\\brave.exe"));
            }
        }
    }
    #[cfg(not(windows))]
    {
        for n in ["google-chrome", "google-chrome-stable", "chromium", "chromium-browser", "microsoft-edge", "microsoft-edge-stable", "brave-browser", "vivaldi"] {
            if let Ok(path) = std::env::var("PATH") {
                for d in path.split(':') {
                    let p = PathBuf::from(d).join(n);
                    if p.is_file() {
                        v.push(p);
                    }
                }
            }
        }
        #[cfg(target_os = "macos")]
        v.push(PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"));
    }
    v.into_iter().filter(|p| p.is_file()).collect()
}

fn profile_dir() -> PathBuf {
    let d = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("data").join(".window"))).unwrap_or_else(|| PathBuf::from("data/.window"));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn spawn_app_window(url: &str) -> Option<Child> {
    let prof = profile_dir();
    for exe in candidates() {
        let mut c = Command::new(&exe);
        c.arg(format!("--app={url}"))
            .arg(format!("--user-data-dir={}", prof.display()))
            .arg("--window-size=1180,760")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-extensions")
            .arg("--disable-sync")
            .arg("--disable-features=Translate,MediaRouter")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Ok(ch) = c.spawn() {
            return Some(ch);
        }
    }
    None
}

fn open_default(url: &str) {
    #[cfg(windows)]
    let _ = Command::new("cmd").args(["/C", "start", "", url]).spawn();
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = Command::new("xdg-open").arg(url).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

/// Blocks until the window is closed (or forever when only a browser tab could be opened).
pub fn run(url: &str, _on_close: impl FnOnce()) {
    #[cfg(windows)]
    crate::win::hide_console_if_owned();
    match spawn_app_window(url) {
        Some(mut child) => {
            let started = Instant::now();
            let _ = child.wait();
            // Chromium hands off to an already-running instance and exits immediately; keep running then.
            if started.elapsed() < Duration::from_secs(3) {
                wait_forever(url);
            }
        }
        None => {
            open_default(url);
            wait_forever(url);
        }
    }
}

fn wait_forever(url: &str) {
    println!("\nPanel: {url}\nZamknij to okno konsoli (lub Ctrl+C), aby zatrzymać boty.");
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}

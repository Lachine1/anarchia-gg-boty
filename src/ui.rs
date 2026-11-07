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
    let d = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("data").join(".webview"))).unwrap_or_else(|| PathBuf::from("data/.webview"));
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
pub fn run(url: &str, on_close: impl FnOnce() + 'static) {
    #[cfg(windows)]
    {
        // native WebView2 window (no browser chrome / infobars); falls back to a browser app window
        if let Err(e) = native::run(url, on_close) {
            eprintln!("WebView2: {e}");
            run_browser(url);
        }
    }
    #[cfg(not(windows))]
    {
        let _ = on_close;
        run_browser(url);
    }
}

#[cfg(windows)]
mod native {
    use tao::dpi::LogicalSize;
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoop};
    use tao::window::WindowBuilder;
    use wry::{WebContext, WebViewBuilder};

    static LOADER: &[u8] = include_bytes!("../assets/WebView2Loader.dll");

    /// Single-file exe: unpack the WebView2 loader next to the profile and point the bindings at it.
    fn install_loader() -> anyhow::Result<()> {
        let p = super::profile_dir().parent().unwrap_or(std::path::Path::new(".")).join("WebView2Loader.dll");
        if std::fs::metadata(&p).map(|m| m.len() != LOADER.len() as u64).unwrap_or(true) {
            std::fs::write(&p, LOADER)?;
        }
        std::env::set_var("WEBVIEW2_LOADER", &p);
        Ok(())
    }

    pub fn run(url: &str, on_close: impl FnOnce() + 'static) -> anyhow::Result<()> {
        install_loader()?;
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title("anarchia-boty")
            .with_inner_size(LogicalSize::new(1180.0, 760.0))
            .with_min_inner_size(LogicalSize::new(720.0, 480.0))
            .build(&event_loop)?;
        let mut ctx = WebContext::new(Some(super::profile_dir()));
        let webview = WebViewBuilder::with_web_context(&mut ctx)
            .with_url(url)
            .with_background_color((14, 15, 17, 255))
            .with_devtools(cfg!(debug_assertions))
            .build(&window)?;
        let mut on_close = Some(on_close);
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            let _keep = (&webview, &window);
            if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
                if let Some(f) = on_close.take() {
                    f();
                }
                std::process::exit(0);
            }
        });
    }
}

fn run_browser(url: &str) {
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

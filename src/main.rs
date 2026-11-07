// Copyright (c) 2026 lachine. AGPL v3 + Commons Clause - see LICENSE.md
//! anarchia-boty v2 — lekkie boty AFK dla anarchia.gg (azalea, 1.21.4).

mod app;
mod bot;
mod net;
mod parity;
mod ui;
mod web;
#[cfg(windows)]
mod win;

use std::{net::SocketAddr, path::PathBuf};

use azalea::prelude::*;
use azalea::swarm::prelude::*;

#[derive(Default, Clone, azalea::ecs::system::Resource)]
struct SwarmState;

async fn swarm_handle(_: Swarm, ev: SwarmEvent, _: SwarmState) -> anyhow::Result<()> {
    if let SwarmEvent::Disconnect(acc, _) = ev {
        app::on_disconnect(&acc.username);
    }
    Ok(())
}

fn data_dir() -> PathBuf {
    if let Ok(d) = std::env::var("ANARCHIA_DATA") {
        return PathBuf::from(d);
    }
    // portable: data/ next to the executable
    std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("data"))).unwrap_or_else(|| PathBuf::from("data"))
}

struct Args {
    child: bool,
    headless: bool,
    bind: SocketAddr,
}

fn args() -> Args {
    let mut headless = std::env::var("TERMUX_VERSION").is_ok() || (cfg!(target_os = "linux") && std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err());
    let mut bind: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let mut child = false;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--headless" | "--web" => headless = true,
            "--child" => child = true,
            "--window" => headless = false,
            "--bind" => {
                if let Some(v) = it.next() {
                    bind = v.parse().unwrap_or_else(|_| format!("{v}:8765").parse().unwrap_or(bind));
                }
            }
            "--port" => {
                if let Some(p) = it.next().and_then(|v| v.parse::<u16>().ok()) {
                    bind.set_port(p);
                }
            }
            "-h" | "--help" => {
                println!("anarchia-boty {}\n\n  --headless        bez okna, panel w przeglądarce (VPS/Termux)\n  --bind ADDR:PORT  adres panelu (domyślnie 127.0.0.1, losowy port; z --headless 0.0.0.0:8765)\n  --port PORT\n  --window          wymuś okno\n\ndane: ./data (lub ANARCHIA_DATA)", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            _ => {}
        }
    }
    if headless && bind.port() == 0 {
        bind = "0.0.0.0:8765".parse().unwrap();
    }
    Args { child, headless, bind }
}

fn deadlock_watch() {
    std::thread::Builder::new().name("deadlock".into()).spawn(|| loop {
        std::thread::sleep(std::time::Duration::from_secs(10));
        let d = parking_lot::deadlock::check_deadlock();
        if d.is_empty() {
            continue;
        }
        let mut msg = String::from("DEADLOCK\n");
        for threads in &d {
            for t in threads {
                msg.push_str(&format!("{:?}\n", t.backtrace()));
            }
        }
        let path = data_dir().join("crash.log");
        let _ = std::fs::write(&path, &msg);
        eprintln!("{msg}");
        // a wedged ECS can't recover in-process: exit non-zero so the launcher restarts us
        std::process::exit(70);
    }).ok();
}

/// Parent process: keeps the bot process alive (restarts on crash/deadlock) and owns the window.
fn guardian(args: &Args) -> anyhow::Result<()> {
    use std::process::Command;
    let exe = std::env::current_exe()?;
    let bind = if args.bind.port() == 0 {
        let l = std::net::TcpListener::bind((args.bind.ip(), 0))?;
        l.local_addr()?
    } else {
        args.bind
    };
    let me = std::process::id().to_string();
    let spawn = move || {
        let mut c = Command::new(&exe);
        c.arg("--child").arg("--bind").arg(bind.to_string()).env("ANARCHIA_PARENT", &me);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: the bot process never shows a console
        }
        c.spawn()
    };
    let child = std::sync::Arc::new(parking_lot::Mutex::new(spawn()?));
    let c2 = child.clone();
    let spawn2 = spawn.clone();
    let headless = args.headless;
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(2));
        let mut ch = c2.lock();
        if let Ok(Some(st)) = ch.try_wait() {
            eprintln!("[guardian] proces botów zakończył się ({st}) — restart za 3s");
            std::thread::sleep(std::time::Duration::from_secs(3));
            match spawn2() {
                Ok(n) => *ch = n,
                Err(e) => eprintln!("[guardian] restart nieudany: {e}"),
            }
        }
    });
    if headless {
        let _ = tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(tokio::signal::ctrl_c());
    } else {
        // wait for the child's panel, then open the window on it
        let token = {
            let p = data_dir().join("settings.json");
            let mut t = String::new();
            for _ in 0..100 {
                if let Some(v) = std::fs::read_to_string(&p).ok().and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()) {
                    if let Some(s) = v["ui_token"].as_str() { t = s.to_string(); }
                }
                if !t.is_empty() && std::net::TcpStream::connect(("127.0.0.1", bind.port())).is_ok() { break; }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            t
        };
        ui::run(&format!("http://127.0.0.1:{}/?t={token}", bind.port()), || {});
    }
    let _ = child.lock().kill();
    Ok(())
}

/// Child: exit as soon as the guardian is gone (window closed / killed), so nothing is left running.
fn watch_parent() {
    let Some(pid) = std::env::var("ANARCHIA_PARENT").ok().and_then(|p| p.parse::<u32>().ok()) else { return };
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(2));
        if !alive(pid) {
            std::process::exit(0);
        }
    });
}

#[cfg(windows)]
fn alive(pid: u32) -> bool {
    win::process_alive(pid)
}
#[cfg(unix)]
fn alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists() || std::process::Command::new("kill").args(["-0", &pid.to_string()]).status().is_ok_and(|s| s.success())
}

fn main() -> anyhow::Result<()> {
    let args = args();
    if !args.child && std::env::var("ANARCHIA_NO_GUARD").is_err() {
        return guardian(&args);
    }
    watch_parent();
    deadlock_watch();
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().thread_name("boty").build()?;
    let a = app::App::init(data_dir());
    let addr = rt.block_on(web::serve(args.bind))?;
    let token = a.cfg.read().ui_token.clone();
    let host = if addr.ip().is_unspecified() { "127.0.0.1".to_string() } else { addr.ip().to_string() };
    let url = format!("http://{host}:{}/?t={token}", addr.port());
    a.log("", 1, format!("panel: {url}"));
    if addr.ip().is_unspecified() {
        a.log("", 1, format!("zdalnie: http://<ip-serwera>:{}/?t={token}", addr.port()));
    }

    // SwarmBuilder isn't Send: drive it from a LocalSet on its own runtime thread
    let h = rt.handle().clone();
    std::thread::Builder::new().name("swarm".into()).spawn(move || {
        let local = tokio::task::LocalSet::new();
        let _g = h.enter();
        h.block_on(local.run_until(async {
            let r = SwarmBuilder::new_without_plugins()
                .add_plugins(azalea::DefaultPlugins)
                .add_plugins(azalea::DefaultBotPlugins)
                .add_plugins(azalea::swarm::DefaultSwarmPlugins)
                .add_plugins(parity::VanillaParityPlugin)
                .set_handler(bot::handle)
                .set_swarm_handler(swarm_handle)
                .on_ready(|sw| {
                    let _ = app::app().swarm.set(sw);
                })
                .start_with_default_opts("anarchia.gg", azalea::JoinOpts::new().custom_resolved_address("127.0.0.1:25565".parse().unwrap()))
                .await;
            let Err(e) = r;
            app::app().log("", 1, format!("swarm: {e}"));
        }));
    })?;
    rt.spawn(app::supervisor());

    if args.headless || args.child {
        if args.headless { println!("\nOtwórz w przeglądarce: {url}\nCtrl+C aby zakończyć."); }
        rt.block_on(async { let _ = tokio::signal::ctrl_c().await; });
    } else {
        ui::run(&url, || {});
    }
    rt.shutdown_timeout(std::time::Duration::from_secs(1));
    Ok(())
}

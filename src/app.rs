// Copyright (c) 2026 lachine. AGPL v3 + Commons Clause - see LICENSE.md
//! Shared state, persistence and the fleet supervisor.

use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use azalea::protocol::connect::Proxy;
use azalea::swarm::Swarm;
use azalea::{Account, Client, JoinOpts};
use parking_lot::{Mutex, RwLock};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::bot::{Phase, State};
use crate::net;

static APP: OnceLock<Arc<App>> = OnceLock::new();
pub fn app() -> &'static Arc<App> {
    APP.get().expect("app not initialised")
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub server: String,
    pub password: String,
    pub join_delay_ms: u64,
    pub per_ip: usize,
    pub drop_target: String,
    pub auto_drop_minutes: u64,
    pub settings_off: bool,
    pub min_cheque: u64,
    pub ui_token: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            server: "anarchia.gg".into(),
            password: String::new(),
            join_delay_ms: 5200,
            per_ip: 25,
            drop_target: String::new(),
            auto_drop_minutes: 0,
            settings_off: true,
            min_cheque: 1000,
            ui_token: random_token(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AccountRec {
    pub name: String,
    pub enabled: bool,
    pub settings_done: bool,
    /// optional per-account password ("nick:hasło" when adding); falls back to the global one
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[derive(Clone)]
pub struct ProxyRec {
    pub addr: SocketAddr,
    pub user: Option<(String, String)>,
    pub raw: String,
    pub ip: Option<String>,
    pub checked: Option<Instant>,
}

pub struct Slot {
    pub state: State,
    pub client: Option<Client>,
    pub connecting: bool,
    pub connected_at: Option<Instant>,
    pub next_try: Instant,
    pub fails: u32,
    pub proxy: Option<String>,
    pub ip: Option<String>,
    pub last_drop: Instant,
}

#[derive(Clone, Serialize)]
pub struct LogLine {
    pub id: u64,
    pub t: u64,
    pub who: String,
    pub msg: String,
    pub lvl: u8,
}

pub struct App {
    pub dir: PathBuf,
    pub cfg: RwLock<Settings>,
    pub accounts: RwLock<Vec<AccountRec>>,
    pub slots: Mutex<HashMap<String, Slot>>,
    pub proxies: RwLock<Vec<ProxyRec>>,
    pub logs: Mutex<VecDeque<LogLine>>,
    pub log_seq: Mutex<u64>,
    pub swarm: OnceLock<Swarm>,
    pub target: RwLock<Option<(String, SocketAddr, Instant)>>,
    pub started: Instant,
    pub last_join: Mutex<Instant>,
    pub drop_queue: Mutex<VecDeque<(String, String)>>,
}

pub fn random_token() -> String {
    let mut r = rand::thread_rng();
    (0..24).map(|_| char::from(b"abcdefghijkmnpqrstuvwxyz23456789"[r.gen_range(0..32)])).collect()
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub fn valid_name(n: &str) -> bool {
    (3..=16).contains(&n.len()) && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn read_json<T: for<'a> Deserialize<'a> + Default>(p: &PathBuf) -> T {
    std::fs::read_to_string(p).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn write_atomic(p: &PathBuf, data: &[u8]) {
    let tmp = p.with_extension("tmp");
    if std::fs::write(&tmp, data).is_ok() {
        let _ = std::fs::rename(&tmp, p);
    }
}

pub fn parse_proxy(line: &str) -> Option<ProxyRec> {
    let raw = line.trim().trim_start_matches("socks5://").trim_start_matches("socks5h://");
    if raw.is_empty() || raw.starts_with('#') {
        return None;
    }
    // host:port  |  host:port:user:pass  |  user:pass@host:port
    let (auth, hp) = match raw.rsplit_once('@') {
        Some((a, h)) => (a.split_once(':').map(|(u, p)| (u.to_string(), p.to_string())), h.to_string()),
        None => {
            let parts: Vec<&str> = raw.split(':').collect();
            if parts.len() == 4 {
                (Some((parts[2].to_string(), parts[3].to_string())), format!("{}:{}", parts[0], parts[1]))
            } else {
                (None, raw.to_string())
            }
        }
    };
    let hp = hp.replace("localhost", "127.0.0.1");
    let addr: SocketAddr = hp.parse().ok().or_else(|| {
        use std::net::ToSocketAddrs;
        hp.to_socket_addrs().ok()?.find(|a| a.is_ipv4())
    })?;
    Some(ProxyRec { addr, user: auth, raw: raw.to_string(), ip: None, checked: None })
}

impl App {
    pub fn init(dir: PathBuf) -> Arc<App> {
        let _ = std::fs::create_dir_all(&dir);
        let cfg: Settings = read_json(&dir.join("settings.json"));
        let accounts: Vec<AccountRec> = read_json(&dir.join("accounts.json"));
        let proxies = std::fs::read_to_string(dir.join("proxies.txt")).unwrap_or_default().lines().filter_map(parse_proxy).collect();
        let app = Arc::new(App {
            dir,
            cfg: RwLock::new(cfg),
            accounts: RwLock::new(accounts),
            slots: Mutex::new(HashMap::new()),
            proxies: RwLock::new(proxies),
            logs: Mutex::new(VecDeque::new()),
            log_seq: Mutex::new(0),
            swarm: OnceLock::new(),
            target: RwLock::new(None),
            started: Instant::now(),
            last_join: Mutex::new(Instant::now() - Duration::from_secs(60)),
            drop_queue: Mutex::new(VecDeque::new()),
        });
        let _ = APP.set(app.clone());
        app.save_settings();
        app.save_accounts();
        app
    }

    pub fn save_settings(&self) {
        let s = serde_json::to_vec_pretty(&*self.cfg.read()).unwrap_or_default();
        write_atomic(&self.dir.join("settings.json"), &s);
    }
    pub fn save_accounts(&self) {
        let s = serde_json::to_vec_pretty(&*self.accounts.read()).unwrap_or_default();
        write_atomic(&self.dir.join("accounts.json"), &s);
    }
    pub fn save_proxies(&self) {
        let s: Vec<String> = self.proxies.read().iter().map(|p| p.raw.clone()).collect();
        write_atomic(&self.dir.join("proxies.txt"), s.join("\n").as_bytes());
    }

    pub fn log(&self, who: &str, lvl: u8, msg: impl Into<String>) {
        let msg = msg.into();
        let id = {
            let mut s = self.log_seq.lock();
            *s += 1;
            *s
        };
        println!("[{}] {}", if who.is_empty() { "*" } else { who }, msg);
        let mut l = self.logs.lock();
        l.push_back(LogLine { id, t: now_ms(), who: who.to_string(), msg, lvl });
        while l.len() > 3000 {
            l.pop_front();
        }
    }

    pub fn password_for(&self, name: &str) -> String {
        self.accounts.read().iter().find(|r| r.name.eq_ignore_ascii_case(name)).and_then(|r| r.password.clone()).filter(|p| !p.is_empty()).unwrap_or_else(|| self.cfg.read().password.clone())
    }

    pub fn mark_settings_done(&self, name: &str) {
        let mut a = self.accounts.write();
        if let Some(r) = a.iter_mut().find(|r| r.name.eq_ignore_ascii_case(name)) {
            r.settings_done = true;
        }
        drop(a);
        self.save_accounts();
    }

    pub fn settings_done(&self, name: &str) -> bool {
        self.accounts.read().iter().any(|r| r.name.eq_ignore_ascii_case(name) && r.settings_done)
    }

    pub fn set_enabled(&self, names: &[String], on: bool) {
        {
            let mut a = self.accounts.write();
            for r in a.iter_mut() {
                if names.iter().any(|n| n.eq_ignore_ascii_case(&r.name)) {
                    r.enabled = on;
                }
            }
        }
        self.save_accounts();
        if !on {
            let slots = self.slots.lock();
            for n in names {
                if let Some(s) = slots.get(n) {
                    if let Some(c) = &s.client {
                        c.disconnect();
                    }
                }
            }
        } else {
            let mut slots = self.slots.lock();
            for n in names {
                if let Some(s) = slots.get_mut(n) {
                    s.next_try = Instant::now();
                    s.fails = 0;
                }
            }
        }
    }

    pub fn client(&self, name: &str) -> Option<(Client, State)> {
        let s = self.slots.lock();
        let sl = s.get(name)?;
        Some((sl.client.clone()?, sl.state.clone()))
    }
}

/// Called by the swarm when a bot's connection ends.
pub fn on_disconnect(name: &str) {
    let a = app();
    let mut slots = a.slots.lock();
    let Some(s) = slots.get_mut(name) else { return };
    let (reason, reached_afk) = {
        let mut rt = s.state.rt.lock();
        let r = rt.last_reason.clone();
        let afk = rt.reached_afk;
        rt.reset_session();
        (r, afk)
    };
    s.client = None;
    s.connecting = false;
    s.connected_at = None;
    if reached_afk {
        s.fails = 0;
    } else {
        s.fails = (s.fails + 1).min(6);
    }
    let low = reason.to_lowercase();
    let base = if low.contains("zbyt cz") || low.contains("too fast") || low.contains("throttl") {
        Duration::from_secs(90)
    } else {
        Duration::from_secs(15) * 2u32.pow(s.fails.min(4))
    };
    let jitter = Duration::from_millis(rand::thread_rng().gen_range(0..6000));
    let d = (base + jitter).min(Duration::from_secs(300));
    s.next_try = Instant::now() + d;
    drop(slots);
    a.log(name, 1, format!("rozłączono{} — ponowna próba za {}s", if reason.is_empty() { String::new() } else { format!(": {reason}") }, d.as_secs()));
}

async fn resolve_target(a: &App) -> Option<SocketAddr> {
    let server = a.cfg.read().server.clone();
    if let Some((s, addr, t)) = a.target.read().clone() {
        if s == server && t.elapsed() < Duration::from_secs(600) {
            return Some(addr);
        }
    }
    match net::resolve(&server).await {
        Ok(addr) => {
            a.log("", 2, format!("DNS {server} -> {addr}"));
            *a.target.write() = Some((server, addr, Instant::now()));
            Some(addr)
        }
        Err(e) => {
            a.log("", 1, format!("DNS {server}: {e}"));
            a.target.read().as_ref().map(|t| t.1)
        }
    }
}

/// Probe egress IPs of all proxies (concurrently, bounded).
pub async fn probe_proxies(force: bool) {
    let a = app();
    let list: Vec<(usize, ProxyRec)> = a.proxies.read().iter().cloned().enumerate()
        .filter(|(_, p)| force || p.checked.map_or(true, |t| t.elapsed() > if p.ip.is_some() { Duration::from_secs(1800) } else { Duration::from_secs(120) }))
        .collect();
    if list.is_empty() {
        return;
    }
    let sem = Arc::new(tokio::sync::Semaphore::new(24));
    let mut tasks = Vec::new();
    for (i, p) in list {
        let sem = sem.clone();
        tasks.push(tokio::spawn(async move {
            let _g = sem.acquire().await;
            (i, p.raw.clone(), net::egress_ip(p.addr, p.user.clone()).await.ok())
        }));
    }
    let mut ok = 0;
    let mut total = 0;
    for t in tasks {
        if let Ok((i, raw, ip)) = t.await {
            total += 1;
            let mut ps = a.proxies.write();
            if let Some(p) = ps.get_mut(i).filter(|p| p.raw == raw) {
                if ip.is_some() {
                    ok += 1;
                }
                p.ip = ip;
                p.checked = Some(Instant::now());
            }
        }
    }
    let ips: std::collections::HashSet<_> = a.proxies.read().iter().filter_map(|p| p.ip.clone()).collect();
    a.log("", 2, format!("proxy: {ok}/{total} działa, {} unikalnych IP", ips.len()));
}

fn pick_proxy(a: &App, slots: &HashMap<String, Slot>, prev: Option<&str>) -> Option<(Option<ProxyRec>, String)> {
    let per_ip = a.cfg.read().per_ip.max(1);
    let mut used: HashMap<String, usize> = HashMap::new();
    let mut by_proxy: HashMap<String, usize> = HashMap::new();
    for s in slots.values() {
        if s.client.is_some() || s.connecting {
            if let Some(ip) = &s.ip {
                *used.entry(ip.clone()).or_default() += 1;
            }
            if let Some(p) = &s.proxy {
                *by_proxy.entry(p.clone()).or_default() += 1;
            }
        }
    }
    let ps = a.proxies.read();
    if ps.is_empty() {
        let n = used.get("direct").copied().unwrap_or(0);
        return (n < per_ip).then(|| (None, "direct".to_string()));
    }
    let free = |p: &&ProxyRec| p.ip.as_ref().is_some_and(|ip| used.get(ip).copied().unwrap_or(0) < per_ip);
    if let Some(prev) = prev {
        if let Some(p) = ps.iter().filter(free).find(|p| p.raw == prev) {
            return Some((Some(p.clone()), p.ip.clone().unwrap()));
        }
    }
    let best = ps.iter().filter(free).min_by_key(|p| (used.get(p.ip.as_ref().unwrap()).copied().unwrap_or(0), by_proxy.get(&p.raw).copied().unwrap_or(0)))?;
    Some((Some(best.clone()), best.ip.clone().unwrap()))
}

/// Main loop: keeps every enabled account connected, enforces the per-IP cap, watchdogs.
pub async fn supervisor() {
    let a = app();
    probe_proxies(true).await;
    let mut tick: u64 = 0;
    loop {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        tick += 1;
        if tick % 60 == 0 {
            tokio::spawn(probe_proxies(false));
        }
        let Some(swarm) = a.swarm.get().cloned() else { continue };
        let accounts = a.accounts.read().clone();

        // sync slots with accounts
        {
            let mut slots = a.slots.lock();
            for r in &accounts {
                slots.entry(r.name.clone()).or_insert_with(|| Slot {
                    state: State::new(&r.name),
                    client: None,
                    connecting: false,
                    connected_at: None,
                    next_try: Instant::now(),
                    fails: 0,
                    proxy: None,
                    ip: None,
                    last_drop: Instant::now(),
                });
            }
            let names: Vec<String> = slots.keys().cloned().collect();
            for n in names {
                if !accounts.iter().any(|r| r.name == n) {
                    if let Some(s) = slots.get(&n) {
                        if let Some(c) = &s.client {
                            c.disconnect();
                        }
                        if s.client.is_none() && !s.connecting {
                            slots.remove(&n);
                        }
                    }
                }
            }
        }

        // watchdogs
        {
            let slots = a.slots.lock();
            for (name, s) in slots.iter() {
                let Some(c) = &s.client else { continue };
                let enabled = accounts.iter().any(|r| &r.name == name && r.enabled);
                if !enabled {
                    c.disconnect();
                    continue;
                }
                if let Some(why) = s.state.watchdog(c) {
                    a.log(name, 1, format!("watchdog: {why} — reconnect"));
                    s.state.rt.lock().last_reason = why;
                    c.disconnect();
                }
            }
        }

        // auto drop
        let (target, every) = { let c = a.cfg.read(); (c.drop_target.trim().to_string(), c.auto_drop_minutes) };
        if every > 0 && !target.is_empty() {
            let mut slots = a.slots.lock();
            let mut q = a.drop_queue.lock();
            for (name, s) in slots.iter_mut() {
                let worth = { let r = s.state.rt.lock(); r.balance.unwrap_or(0.0) >= a.cfg.read().min_cheque as f64 || r.inv.iter().any(|i| crate::bot::is_key(&i.id, &i.title, &i.lore)) };
                if s.client.is_some() && s.state.phase() == Phase::Afk && worth && s.last_drop.elapsed() > Duration::from_secs(every * 60) {
                    s.last_drop = Instant::now();
                    if !q.iter().any(|(n, _)| n == name) {
                        q.push_back((name.clone(), target.clone()));
                    }
                }
            }
        }
        // drop queue: start one drop every 2s
        if tick % 2 == 0 {
            let next = a.drop_queue.lock().pop_front();
            if let Some((name, target)) = next {
                if let Some((c, st)) = a.client(&name) {
                    if let Some(s) = a.slots.lock().get_mut(&name) {
                        s.last_drop = Instant::now();
                    }
                    tokio::spawn(crate::bot::drop_to(c, st, target));
                }
            }
        }

        // joins (never without a password: the server would register the account with it)
        if a.cfg.read().password.is_empty() && !a.accounts.read().iter().any(|r| r.password.as_ref().is_some_and(|p| !p.is_empty())) {
            continue;
        }
        let delay = Duration::from_millis(a.cfg.read().join_delay_ms.max(500));
        if a.last_join.lock().elapsed() < delay {
            continue;
        }
        let candidate = {
            let slots = a.slots.lock();
            let now = Instant::now();
            accounts.iter().filter(|r| r.enabled && !a.password_for(&r.name).is_empty()).find_map(|r| {
                let s = slots.get(&r.name)?;
                (s.client.is_none() && !s.connecting && now >= s.next_try).then(|| r.name.clone())
            })
        };
        let Some(name) = candidate else { continue };
        let Some(target) = resolve_target(a).await else { continue };
        let pick = {
            let slots = a.slots.lock();
            let prev = slots.get(&name).and_then(|s| s.proxy.clone());
            pick_proxy(a, &slots, prev.as_deref())
        };
        let Some((proxy, ip)) = pick else {
            let mut slots = a.slots.lock();
            if let Some(s) = slots.get_mut(&name) {
                s.next_try = Instant::now() + Duration::from_secs(20);
                s.state.set_phase(Phase::NoProxy);
            }
            continue;
        };
        *a.last_join.lock() = Instant::now();
        let state = {
            let mut slots = a.slots.lock();
            let Some(s) = slots.get_mut(&name) else { continue };
            s.connecting = true;
            s.proxy = proxy.as_ref().map(|p| p.raw.clone());
            s.ip = Some(ip.clone());
            s.state.reset_session();
            s.state.set_phase(Phase::Connecting);
            s.state.rt.lock().password = a.password_for(&name);
            s.state.clone()
        };
        let server = a.cfg.read().server.clone();
        let mut opts = JoinOpts::new()
            .custom_address(azalea::protocol::ServerAddress { host: server, port: target.port() })
            .custom_resolved_address(target);
        if let Some(p) = &proxy {
            let auth = p.user.as_ref().map(|(u, pw)| socks5_impl::protocol::UserKey::new(u.clone(), pw.clone()));
            opts = opts.proxy(Proxy::new(p.addr, auth));
        }
        a.log(&name, 2, format!("łączenie przez {}", proxy.as_ref().map(|p| p.raw.as_str()).unwrap_or("bez proxy")));
        let mut sw = swarm.clone();
        tokio::spawn(async move {
            let acc = Account::offline(&name);
            let r = tokio::time::timeout(Duration::from_secs(60), sw.add_with_opts(&acc, state.clone(), &opts)).await;
            let a = app();
            let mut slots = a.slots.lock();
            let Some(s) = slots.get_mut(&name) else { return };
            s.connecting = false;
            match r {
                Ok(Ok(c)) => {
                    s.client = Some(c);
                    s.connected_at = Some(Instant::now());
                    state.touch();
                }
                Ok(Err(e)) => {
                    let msg = e.to_string();
                    s.fails = (s.fails + 1).min(6);
                    let low = msg.to_lowercase();
                    let base = if low.contains("zbyt cz") { 90 } else { 15 * 2u64.pow(s.fails.min(4)) };
                    s.next_try = Instant::now() + Duration::from_secs(base.min(300) + rand::thread_rng().gen_range(0..6));
                    state.set_phase(Phase::Offline);
                    state.rt.lock().last_reason = msg.chars().take(160).collect();
                    drop(slots);
                    a.log(&name, 1, format!("nie udało się połączyć: {}", msg.chars().take(160).collect::<String>()));
                }
                Err(_) => {
                    s.fails = (s.fails + 1).min(6);
                    s.next_try = Instant::now() + Duration::from_secs(30);
                    state.set_phase(Phase::Offline);
                    state.rt.lock().last_reason = "timeout połączenia".into();
                    drop(slots);
                    a.log(&name, 1, "timeout połączenia");
                }
            }
        });
    }
}

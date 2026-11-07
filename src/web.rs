// Copyright (c) 2026 lachine. AGPL v3 + Commons Clause - see LICENSE.md
//! Embedded control panel: JSON API + single static page. Same UI for the native window and the browser.

use std::{net::SocketAddr, time::Duration};

use axum::{
    extract::{Query, Request},
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::app::{app, parse_proxy, valid_name, AccountRec};

const INDEX: &str = include_str!("../ui/index.html");

async fn auth(headers: HeaderMap, q: Query<std::collections::HashMap<String, String>>, req: Request, next: Next) -> Response {
    let token = app().cfg.read().ui_token.clone();
    let cookie_ok = headers.get(header::COOKIE).and_then(|c| c.to_str().ok()).is_some_and(|c| c.split(';').any(|kv| kv.trim() == format!("t={token}")));
    let header_ok = headers.get("x-token").and_then(|v| v.to_str().ok()) == Some(token.as_str());
    let query_ok = q.get("t").map(String::as_str) == Some(token.as_str());
    if cookie_ok || header_ok {
        return next.run(req).await;
    }
    if query_ok {
        let mut r = next.run(req).await;
        if let Ok(v) = format!("t={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age=31536000").parse() {
            r.headers_mut().insert(header::SET_COOKIE, v);
        }
        return r;
    }
    (StatusCode::UNAUTHORIZED, Html("<!doctype html><meta charset=utf-8><title>anarchia-boty</title><body style=\"font:14px system-ui;background:#111;color:#bbb;display:grid;place-items:center;height:100vh;margin:0\"><form><p>Podaj token z konsoli</p><input name=t autofocus style=\"padding:8px;width:260px;background:#000;color:#eee;border:1px solid #333\"></form>")).into_response()
}

pub async fn serve(bind: SocketAddr) -> anyhow::Result<SocketAddr> {
    let api = Router::new()
        .route("/", get(|| async { ([(header::CACHE_CONTROL, "no-store")], Html(INDEX)) }))
        .route("/api/state", get(state))
        .route("/api/logs", get(logs))
        .route("/api/accounts", post(accounts))
        .route("/api/proxies", post(proxies))
        .route("/api/settings", post(settings))
        .route("/api/action", post(action))
        .layer(middleware::from_fn(auth));
    let listener = tokio::net::TcpListener::bind(bind).await?;
    let addr = listener.local_addr()?;
    tokio::spawn(async move {
        let _ = axum::serve(listener, api).await;
    });
    Ok(addr)
}

fn fmt_dur(d: Duration) -> String {
    let s = d.as_secs();
    if s >= 3600 { format!("{}h {:02}m", s / 3600, s / 60 % 60) } else if s >= 60 { format!("{}m {:02}s", s / 60, s % 60) } else { format!("{s}s") }
}

async fn state() -> Json<Value> {
    let a = app();
    let accounts = a.accounts.read().clone();
    let slots = a.slots.lock();
    let mut bots = Vec::new();
    let (mut online, mut afk, mut earned, mut keys, mut dropped, mut dkeys) = (0, 0, 0.0, 0u32, 0.0, 0u32);
    for r in &accounts {
        let s = slots.get(&r.name);
        let connecting = s.is_some_and(|s| s.connecting);
        let up_s = s.and_then(|s| s.connected_at.map(|t| t.elapsed().as_secs()));
        let (phase, reason, e, k, bal, dm, dk, up, ip, inv_keys, inv_cheq, wait) = match s {
            Some(s) => {
                let rt = s.state.rt.lock();
                let kk: i32 = rt.inv.iter().filter(|i| crate::bot::is_key(&i.id, &i.title, &i.lore)).map(|i| i.count).sum();
                let cc: i32 = rt.inv.iter().filter(|i| crate::bot::is_cheque(&i.id, &i.title, &i.lore)).map(|i| i.count).sum();
                let wait = (s.client.is_none() && !s.connecting && r.enabled).then(|| s.next_try.saturating_duration_since(std::time::Instant::now()).as_secs());
                (rt.phase, rt.last_reason.clone(), rt.earned_total, rt.keys_total, rt.balance, rt.dropped_money, rt.dropped_keys,
                 s.connected_at.map(|t| fmt_dur(t.elapsed())), s.ip.clone(), kk, cc, wait)
            }
            None => (crate::bot::Phase::Offline, String::new(), 0.0, 0, None, 0.0, 0, None, None, 0, 0, None),
        };
        let connected = s.is_some_and(|s| s.client.is_some());
        if connected { online += 1; }
        if phase == crate::bot::Phase::Afk { afk += 1; }
        earned += e; keys += k; dropped += dm; dkeys += dk;
        bots.push(json!({
            "name": r.name, "enabled": r.enabled, "connected": connected,
            "phase": phase, "label": if connected || phase != crate::bot::Phase::Offline { phase.label() } else if r.enabled { "czeka" } else { "wyłączony" },
            "reason": reason, "earned": e, "keys": k, "balance": bal, "dropped": dm, "dropped_keys": dk,
            "uptime": up, "ip": ip, "inv_keys": inv_keys, "inv_cheques": inv_cheq, "retry_in": wait,
            "settings_done": r.settings_done, "connecting": connecting, "up_s": up_s,
        }));
    }
    drop(slots);
    let proxies = a.proxies.read();
    let mut ips = std::collections::HashMap::<String, usize>::new();
    for p in proxies.iter() { if let Some(ip) = &p.ip { *ips.entry(ip.clone()).or_default() += 0; } }
    for b in &bots { if b["connected"].as_bool() == Some(true) { if let Some(ip) = b["ip"].as_str() { *ips.entry(ip.to_string()).or_default() += 1; } } }
    let working = proxies.iter().filter(|p| p.ip.is_some()).count();
    let cfg = a.cfg.read().clone();
    let rss = rss_kb();
    let rx = azalea::protocol::read::BYTES_READ.load(std::sync::atomic::Ordering::Relaxed);
    let tx = azalea::protocol::read::BYTES_WRITTEN.load(std::sync::atomic::Ordering::Relaxed);
    Json(json!({
        "bots": bots,
        "totals": { "accounts": accounts.len(), "online": online, "afk": afk, "earned": earned, "keys": keys, "dropped": dropped, "dropped_keys": dkeys },
        "proxies": { "total": proxies.len(), "working": working, "ips": ips.len(), "per_ip": cfg.per_ip, "list": proxies.iter().map(|p| json!({"raw": p.raw, "ip": p.ip})).collect::<Vec<_>>(),
                     "capacity": if proxies.is_empty() { cfg.per_ip } else { ips.len() * cfg.per_ip } },
        "settings": { "server": cfg.server, "has_password": !cfg.password.is_empty(), "join_delay_ms": cfg.join_delay_ms, "per_ip": cfg.per_ip,
                      "drop_target": cfg.drop_target, "auto_drop_minutes": cfg.auto_drop_minutes, "settings_off": cfg.settings_off, "min_cheque": cfg.min_cheque },
        "sys": { "rss_kb": rss, "rx": rx, "tx": tx, "uptime": fmt_dur(a.started.elapsed()), "target": a.target.read().as_ref().map(|t| t.1.to_string()), "version": env!("CARGO_PKG_VERSION") },
    }))
}

pub fn rss_kb() -> u64 {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/self/status").ok().and_then(|s| s.lines().find(|l| l.starts_with("VmRSS")).and_then(|l| l.split_whitespace().nth(1)?.parse().ok())).unwrap_or(0)
    }
    #[cfg(target_os = "android")]
    {
        std::fs::read_to_string("/proc/self/status").ok().and_then(|s| s.lines().find(|l| l.starts_with("VmRSS")).and_then(|l| l.split_whitespace().nth(1)?.parse().ok())).unwrap_or(0)
    }
    #[cfg(windows)]
    {
        crate::win::rss_kb()
    }
    #[cfg(not(any(target_os = "linux", target_os = "android", windows)))]
    {
        0
    }
}

#[derive(Deserialize)]
struct LogQ { after: Option<u64>, who: Option<String> }

async fn logs(Query(q): Query<LogQ>) -> Json<Value> {
    let after = q.after.unwrap_or(0);
    let l = app().logs.lock();
    let v: Vec<_> = l.iter().filter(|x| x.id > after && q.who.as_ref().map_or(true, |w| w.is_empty() || &x.who == w)).rev().take(400).cloned().collect::<Vec<_>>().into_iter().rev().collect();
    Json(json!({ "logs": v }))
}

#[derive(Deserialize)]
struct AccBody { op: String, #[serde(default)] names: Vec<String>, #[serde(default)] text: String }

async fn accounts(Json(b): Json<AccBody>) -> Json<Value> {
    let a = app();
    match b.op.as_str() {
        "add" => {
            let mut added = 0; let mut bad = Vec::new();
            {
                let mut acc = a.accounts.write();
                // one entry per line ("nick" or "nick:hasło"); commas/semicolons also separate
                for tok in b.text.split(['\n', '\r', ',', ';']).map(str::trim).filter(|s| !s.is_empty()) {
                    let tok = tok.trim_matches(|c: char| c == '"' || c == '\'');
                    let (n, pw) = match tok.split_once(':') { Some((n, p)) => (n, Some(p.to_string())), None => (tok, None) };
                    if !valid_name(n) { bad.push(n.to_string()); continue; }
                    if let Some(r) = acc.iter_mut().find(|r| r.name.eq_ignore_ascii_case(n)) { if pw.is_some() { r.password = pw; } continue; }
                    acc.push(AccountRec { name: n.to_string(), enabled: true, settings_done: false, password: pw });
                    added += 1;
                }
            }
            a.save_accounts();
            if added > 0 { a.log("", 1, format!("dodano {added} kont")); }
            Json(json!({ "ok": true, "added": added, "invalid": bad }))
        }
        "remove" => {
            a.set_enabled(&b.names, false);
            a.accounts.write().retain(|r| !b.names.iter().any(|n| n.eq_ignore_ascii_case(&r.name)));
            a.save_accounts();
            Json(json!({ "ok": true }))
        }
        "enable" | "disable" => { a.set_enabled(&b.names, b.op == "enable"); Json(json!({ "ok": true })) }
        "reconnect" => {
            for n in &b.names { if let Some((c, _)) = a.client(n) { c.disconnect(); } }
            let mut s = a.slots.lock();
            for n in &b.names { if let Some(s) = s.get_mut(n) { s.next_try = std::time::Instant::now(); s.fails = 0; } }
            Json(json!({ "ok": true }))
        }
        "resettings" => {
            { let mut acc = a.accounts.write(); for r in acc.iter_mut() { if b.names.iter().any(|n| n == &r.name) { r.settings_done = false; } } }
            a.save_accounts();
            Json(json!({ "ok": true }))
        }
        _ => Json(json!({ "ok": false, "error": "nieznana operacja" })),
    }
}

#[derive(Deserialize)]
struct ProxyBody { op: String, #[serde(default)] text: String }

async fn proxies(Json(b): Json<ProxyBody>) -> Json<Value> {
    let a = app();
    match b.op.as_str() {
        "set" => {
            let list: Vec<_> = b.text.lines().filter_map(parse_proxy).collect();
            let n = list.len();
            {
                let mut ps = a.proxies.write();
                let old: std::collections::HashMap<String, _> = ps.drain(..).map(|p| (p.raw.clone(), p)).collect();
                *ps = list.into_iter().map(|p| old.get(&p.raw).cloned().unwrap_or(p)).collect();
            }
            a.save_proxies();
            tokio::spawn(crate::app::probe_proxies(false));
            Json(json!({ "ok": true, "count": n }))
        }
        "check" => { tokio::spawn(crate::app::probe_proxies(true)); Json(json!({ "ok": true })) }
        _ => Json(json!({ "ok": false })),
    }
}

async fn settings(Json(v): Json<Value>) -> Json<Value> {
    let a = app();
    {
        let mut c = a.cfg.write();
        if let Some(s) = v["server"].as_str() { if !s.trim().is_empty() { c.server = s.trim().to_string(); } }
        if let Some(s) = v["password"].as_str() { if !s.is_empty() { c.password = s.to_string(); } }
        if let Some(n) = v["join_delay_ms"].as_u64() { c.join_delay_ms = n.clamp(500, 120_000); }
        if let Some(n) = v["per_ip"].as_u64() { c.per_ip = (n as usize).clamp(1, 25); }
        if let Some(s) = v["drop_target"].as_str() { c.drop_target = s.trim().to_string(); }
        if let Some(n) = v["auto_drop_minutes"].as_u64() { c.auto_drop_minutes = n.min(24 * 60); }
        if let Some(b) = v["settings_off"].as_bool() { c.settings_off = b; }
        if let Some(n) = v["min_cheque"].as_u64() { c.min_cheque = n.max(1); }
    }
    a.save_settings();
    Json(json!({ "ok": true }))
}

#[derive(Deserialize)]
struct ActBody { op: String, #[serde(default)] names: Vec<String>, #[serde(default)] target: String, #[serde(default)] text: String }

async fn action(Json(b): Json<ActBody>) -> Json<Value> {
    let a = app();
    let names: Vec<String> = if b.names.is_empty() { a.accounts.read().iter().map(|r| r.name.clone()).collect() } else { b.names };
    match b.op.as_str() {
        "drop" => {
            let target = if b.target.trim().is_empty() { a.cfg.read().drop_target.clone() } else { b.target.trim().to_string() };
            if !valid_name(&target) { return Json(json!({ "ok": false, "error": "podaj poprawny nick odbiorcy" })); }
            let mut q = a.drop_queue.lock();
            let mut n = 0;
            for name in names { if a.client(&name).is_some() && !q.iter().any(|(x, _)| x == &name) { q.push_back((name, target.clone())); n += 1; } }
            a.log("", 1, format!("oddawanie do {target}: {n} botów w kolejce"));
            Json(json!({ "ok": true, "queued": n }))
        }
        "chat" => {
            let t = b.text.trim();
            if t.is_empty() { return Json(json!({ "ok": false })); }
            let mut n = 0;
            for name in names { if let Some((c, _)) = a.client(&name) { if let Some(cmd) = t.strip_prefix('/') { c.send_command_packet(cmd); } else { c.chat(t); } n += 1; } }
            Json(json!({ "ok": true, "sent": n }))
        }
        "afk" => {
            for name in names { if let Some((c, s)) = a.client(&name) { crate::bot::send_afk(c, s); } }
            Json(json!({ "ok": true }))
        }
        "debug" => {
            let mut out = serde_json::Map::new();
            for name in names { if let Some((_, s)) = a.client(&name) { let r = s.rt.lock(); out.insert(name, json!({"sidebar": r.sidebar, "inv": r.inv.iter().map(|i| format!("{}:{}x{} {} | {}", i.slot, i.id, i.count, i.title, i.lore)).collect::<Vec<_>>()})); } }
            Json(Value::Object(out))
        }
        _ => Json(json!({ "ok": false })),
    }
}

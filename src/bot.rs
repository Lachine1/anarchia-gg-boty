// Copyright (c) 2026 lachine. AGPL v3 + Commons Clause - see LICENSE.md
//! Per-account state machine: auth → rules → verify (anvil + movement) → mode → SMP → /settings → /afk → earn.

use std::{sync::Arc, time::{Duration, Instant}};

use azalea::inventory::components::{CustomName, ItemName, Lore};
use azalea::inventory::{operations::ClickType, ItemStack};
use azalea::prelude::*;
use azalea::protocol::packets::game::{
    s_container_click::ServerboundContainerClick, s_container_close::ServerboundContainerClose,
    s_interact::InteractionHand, s_rename_item::ServerboundRenameItem, s_use_item::ServerboundUseItem,
    ClientboundGamePacket,
};
use azalea::{ClientInformation, WalkDirection};
use parking_lot::Mutex;
use rand::Rng;
use serde::Serialize;

use crate::app::app;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize)]
pub enum Phase {
    #[default]
    Offline,
    Connecting,
    NoProxy,
    Auth,
    Verify,
    Lobby,
    Transfer,
    Smp,
    Settings,
    AfkMenu,
    Afk,
    Dropping,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Phase::Offline => "offline",
            Phase::Connecting => "łączenie",
            Phase::NoProxy => "brak wolnego IP",
            Phase::Auth => "logowanie",
            Phase::Verify => "weryfikacja",
            Phase::Lobby => "lobby",
            Phase::Transfer => "przenoszenie",
            Phase::Smp => "smp",
            Phase::Settings => "/settings",
            Phase::AfkMenu => "/afk",
            Phase::Afk => "afk",
            Phase::Dropping => "oddawanie",
        }
    }
}

#[derive(Default)]
pub struct Rt {
    pub phase: Phase,
    pub password: String,
    pub since: Option<Instant>,
    pub phase_at: Option<Instant>,
    pub last_packet: Option<Instant>,
    pub reached_afk: bool,
    pub last_reason: String,
    pub earned_session: f64,
    pub earned_total: f64,
    pub keys_total: u32,
    pub balance: Option<f64>,
    pub auth_sent: bool,
    pub walking: bool,
    pub anvil_id: Option<i32>,
    pub anvil_pending: Option<(i32, String)>,
    pub handled_window: Option<(i32, u32)>,
    pub window: Option<(i32, u32)>,
    pub seq: u32,
    pub afk_tries: u32,
    pub afk_cmd_at: Option<Instant>,
    pub settings_clicks: u32,
    pub dropped_money: f64,
    pub dropped_keys: u32,
    pub drop_busy: bool,
    pub inv: Vec<InvItem>,
    pub inv_state: u32,
    pub sidebar: std::collections::HashMap<String, String>,
}

impl Rt {
    pub fn reset_session(&mut self) {
        let keep = (self.earned_total, self.keys_total, self.dropped_money, self.dropped_keys, std::mem::take(&mut self.password), std::mem::take(&mut self.last_reason));
        *self = Rt::default();
        (self.earned_total, self.keys_total, self.dropped_money, self.dropped_keys, self.password, self.last_reason) = keep;
    }
}

#[derive(Clone, Component)]
pub struct State {
    pub name: Arc<str>,
    pub rt: Arc<Mutex<Rt>>,
}

impl Default for State {
    fn default() -> Self {
        Self::new("")
    }
}

impl State {
    pub fn new(name: &str) -> Self {
        Self { name: name.into(), rt: Arc::new(Mutex::new(Rt::default())) }
    }
    pub fn phase(&self) -> Phase {
        self.rt.lock().phase
    }
    pub fn set_phase(&self, p: Phase) {
        let mut r = self.rt.lock();
        if r.phase != p {
            r.phase = p;
            r.phase_at = Some(Instant::now());
        }
    }
    pub fn reset_session(&self) {
        self.rt.lock().reset_session();
    }
    pub fn touch(&self) {
        let mut r = self.rt.lock();
        r.since.get_or_insert_with(Instant::now);
        r.last_packet = Some(Instant::now());
    }
    fn log(&self, lvl: u8, m: impl Into<String>) {
        app().log(&self.name, lvl, m);
    }

    /// Returns a reason when the session looks stuck and should be recycled.
    pub fn watchdog(&self, _c: &Client) -> Option<String> {
        let r = self.rt.lock();
        let now = Instant::now();
        if let Some(t) = r.last_packet {
            if now - t > Duration::from_secs(45) {
                return Some("brak pakietów od 45s".into());
            }
        }
        let in_phase = r.phase_at.map(|t| now - t).unwrap_or_default();
        let limit = match r.phase {
            Phase::Afk | Phase::Dropping | Phase::Offline | Phase::NoProxy => return None,
            Phase::Settings => 60,
            Phase::Connecting | Phase::Auth | Phase::Verify | Phase::Lobby => 120,
            Phase::Transfer | Phase::Smp | Phase::AfkMenu => 150,
        };
        (in_phase > Duration::from_secs(limit)).then(|| format!("utknął w fazie {} ({}s)", r.phase.label(), in_phase.as_secs()))
    }
}

pub fn plain(t: &azalea::FormattedText) -> String {
    let s = t.to_string();
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c == '\u{a7}' {
            it.next();
        } else {
            out.push(c);
        }
    }
    out
}

fn item_title(item: &ItemStack) -> String {
    let Some(d) = item.as_present() else { return String::new() };
    if let Some(c) = d.components.get::<CustomName>() {
        return plain(&c.name);
    }
    if let Some(c) = d.components.get::<ItemName>() {
        return plain(&c.name);
    }
    String::new()
}

fn item_lore(item: &ItemStack) -> String {
    item.as_present()
        .and_then(|d| d.components.get::<Lore>())
        .map(|l| l.lines.iter().map(plain).collect::<Vec<_>>().join(" / "))
        .unwrap_or_default()
}

fn item_id(item: &ItemStack) -> String {
    item.as_present().map(|d| d.kind.to_string().trim_start_matches("minecraft:").to_string()).unwrap_or_default()
}

pub fn is_key(id: &str, title: &str, lore: &str) -> bool {
    let t = format!("{title} {lore}").to_lowercase();
    t.contains("klucz") || (id == "tripwire_hook" && !t.is_empty())
}

pub fn is_cheque(id: &str, title: &str, lore: &str) -> bool {
    let t = format!("{title} {lore}").to_lowercase();
    id == "paper" && (t.contains("czek") || t.contains('$'))
}

pub fn parse_money(t: &str) -> Option<f64> {
    // "+2$", "+2,5 $", "24.00$", "1.234,50$", "1 234$", "1,5k$", "$ Stan konta: 24.00$"
    let bytes: Vec<char> = t.chars().collect();
    let pos = bytes.iter().rposition(|&c| c == '$')?;
    let mut i = pos;
    while i > 0 && bytes[i - 1] == ' ' {
        i -= 1;
    }
    let end = i;
    while i > 0 && (bytes[i - 1].is_ascii_digit() || matches!(bytes[i - 1], '.' | ',' | ' ' | 'k' | 'K' | 'm' | 'M')) {
        i -= 1;
    }
    let raw: String = bytes[i..end].iter().filter(|c| **c != ' ').collect();
    let (num, mul) = match raw.chars().last()? {
        'k' | 'K' => (&raw[..raw.len() - 1], 1e3),
        'm' | 'M' => (&raw[..raw.len() - 1], 1e6),
        _ => (raw.as_str(), 1.0),
    };
    if num.is_empty() {
        return None;
    }
    // decimal separator = the last '.'/',' followed by 1-2 digits; others are thousands separators
    let last_sep = num.rfind(['.', ',']);
    let norm = match last_sep {
        Some(p) if num.len() - p - 1 <= 2 => format!("{}.{}", num[..p].replace(['.', ','], ""), &num[p + 1..]),
        _ => num.replace(['.', ','], ""),
    };
    norm.parse::<f64>().ok().map(|v| v * mul)
}

#[cfg(test)]
mod tests {
    use super::parse_money;
    #[test]
    fn money() {
        assert_eq!(parse_money("+2$"), Some(2.0));
        assert_eq!(parse_money("+2,5 $"), Some(2.5));
        assert_eq!(parse_money("$ Stan konta: 24.00$"), Some(24.0));
        assert_eq!(parse_money("Stan konta: 1.234,50$"), Some(1234.5));
        assert_eq!(parse_money("1 234$"), Some(1234.0));
        assert_eq!(parse_money("1,5k$"), Some(1500.0));
        assert_eq!(parse_money("12.000$"), Some(12000.0));
        assert_eq!(parse_money("brak"), None);
    }
}

fn later(ms: u64, f: impl FnOnce() + Send + 'static) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(ms)).await;
        f();
    });
}

fn jitter(ms: u64) -> u64 {
    ms + rand::thread_rng().gen_range(0..ms / 3 + 1)
}

fn click(bot: &Client, cid: i32, sid: u32, slot: usize, button: u8, kind: ClickType) {
    let _ = bot.write_packet(ServerboundContainerClick {
        container_id: cid,
        state_id: sid,
        slot_num: slot as i16,
        button_num: button,
        click_type: kind,
        changed_slots: Default::default(),
        carried_item: ItemStack::Empty,
    });
}

fn use_item(bot: &Client, st: &State) {
    let d = bot.component::<azalea::entity::LookDirection>();
    let seq = {
        let mut r = st.rt.lock();
        r.seq += 1;
        r.seq
    };
    let _ = bot.write_packet(ServerboundUseItem { hand: InteractionHand::MainHand, sequence: seq, yaw: d.y_rot, pitch: d.x_rot });
}

pub async fn handle(mut bot: Client, event: Event, st: State) -> anyhow::Result<()> {
    match event {
        Event::Init => {
            bot.set_client_information(ClientInformation { view_distance: 2, ..Default::default() }).await?;
        }
        Event::Login => {
            st.touch();
            if st.phase() == Phase::Connecting {
                st.set_phase(Phase::Auth);
            }
        }
        Event::Disconnect(r) => {
            if let Some(r) = r {
                let t = plain(&r);
                if !t.is_empty() {
                    st.rt.lock().last_reason = t.chars().take(200).collect();
                }
            }
        }
        Event::Packet(p) => {
            st.rt.lock().last_packet = Some(Instant::now());
            on_packet(&mut bot, &st, &p);
        }
        _ => {}
    }
    Ok(())
}

fn on_packet(bot: &mut Client, st: &State, p: &ClientboundGamePacket) {
    match p {
        ClientboundGamePacket::SystemChat(m) => {
            let t = plain(&m.content);
            if m.overlay {
                on_actionbar(bot, st, &t);
            } else {
                on_chat(bot, st, &t);
            }
        }
        ClientboundGamePacket::SetActionBarText(a) => on_actionbar(bot, st, &plain(&a.text)),
        ClientboundGamePacket::SetSubtitleText(s) => on_reward(st, &plain(&s.text)),
        ClientboundGamePacket::SetTitleText(s) => on_reward(st, &plain(&s.text)),
        ClientboundGamePacket::Disconnect(d) => {
            st.rt.lock().last_reason = plain(&d.reason).chars().take(200).collect();
        }
        ClientboundGamePacket::SetScore(s) => {
            let line = s.display.as_ref().map(plain).unwrap_or_else(|| s.owner.clone());
            let mut r = st.rt.lock();
            if r.sidebar.len() < 256 {
                r.sidebar.insert(format!("s:{}", s.owner), line.clone());
            }
            update_balance(&mut r, &line);
        }
        ClientboundGamePacket::SetPlayerTeam(t) => {
            use azalea::protocol::packets::game::c_set_player_team::Method;
            if let Method::Add((p, _)) | Method::Change(p) = &t.method {
                let line = format!("{}{}", plain(&p.player_prefix), plain(&p.player_suffix));
                let mut r = st.rt.lock();
                if !line.trim().is_empty() && r.sidebar.len() < 64 {
                    r.sidebar.insert(format!("t:{}", t.name), line.clone());
                }
                update_balance(&mut r, &line);
            }
        }
        ClientboundGamePacket::Login(_) | ClientboundGamePacket::Respawn(_) => {
            let mut r = st.rt.lock();
            r.walking = false;
            r.window = None;
            r.handled_window = None;
            r.sidebar.clear();
            if r.phase == Phase::Transfer {
                drop(r);
                st.set_phase(Phase::Smp);
                let (b, s) = (bot.clone(), st.clone());
                later(jitter(6000), move || after_smp_join(b, s));
            }
        }
        ClientboundGamePacket::OpenScreen(o) => {
            let title = plain(&o.title);
            st.log(3, format!("okno {} '{}'", o.container_id, title));
            let pending = {
                let mut r = st.rt.lock();
                r.window = Some((o.container_id, 0));
                r.handled_window = None;
                if o.menu_type == azalea::registry::MenuKind::Anvil {
                    r.anvil_id = Some(o.container_id);
                    r.anvil_pending.take().filter(|(c, _)| *c == o.container_id)
                } else {
                    None
                }
            };
            if let Some((_, name)) = pending {
                answer_anvil(bot, st, name);
            }
        }
        ClientboundGamePacket::ContainerClose(_) => {
            let mut r = st.rt.lock();
            r.window = None;
            r.handled_window = None;
        }
        ClientboundGamePacket::ContainerSetSlot(s) => {
            if s.container_id == 0 {
                inv_slot(st, s.slot as usize, &s.item_stack, s.state_id);
                return;
            }
            if s.slot == 0 {
                let name = item_title(&s.item_stack);
                if !name.is_empty() {
                    let anvil = st.rt.lock().anvil_id;
                    if Some(s.container_id) == anvil {
                        answer_anvil(bot, st, name);
                    } else {
                        st.rt.lock().anvil_pending = Some((s.container_id, name));
                    }
                }
            }
            let mut r = st.rt.lock();
            if let Some(w) = &mut r.window {
                if w.0 == s.container_id {
                    w.1 = s.state_id;
                }
            }
        }
        ClientboundGamePacket::ContainerSetContent(c) => {
            if c.container_id == 0 {
                let mut r = st.rt.lock();
                r.inv = c.items.iter().enumerate().filter(|(_, i)| i.is_present()).map(|(i, it)| inv_item(i, it)).collect();
                r.inv_state = c.state_id;
                return;
            }
            let items: Vec<Item> = c.items.iter().enumerate().filter(|(_, i)| i.is_present()).map(|(i, it)| Item { slot: i, id: item_id(it), title: item_title(it), lore: item_lore(it) }).collect();
            {
                let mut r = st.rt.lock();
                r.window = Some((c.container_id, c.state_id));
                if r.handled_window.is_some_and(|(id, _)| id == c.container_id) && r.phase != Phase::Settings {
                    return;
                }
                r.handled_window = Some((c.container_id, c.state_id));
            }
            let (b, s, cid, sid) = (bot.clone(), st.clone(), c.container_id, c.state_id);
            tokio::spawn(async move { on_window(b, s, cid, sid, items).await });
        }
        _ => {}
    }
}

fn update_balance(r: &mut Rt, line: &str) {
    let l = line.to_lowercase();
    if l.contains("stan konta") || l.contains("saldo") || l.contains("portfel") || l.contains("kasa:") {
        if let Some(v) = parse_money(line) {
            r.balance = Some(v);
        }
    }
}

fn inv_item(slot: usize, it: &ItemStack) -> InvItem {
    InvItem { slot, id: item_id(it), count: it.count(), title: item_title(it), lore: item_lore(it) }
}

fn inv_slot(st: &State, slot: usize, it: &ItemStack, state_id: u32) {
    let mut r = st.rt.lock();
    r.inv.retain(|x| x.slot != slot);
    if it.is_present() {
        r.inv.push(inv_item(slot, it));
    }
    r.inv_state = state_id;
}

fn on_reward(st: &State, t: &str) {
    let t = t.trim();
    if !t.starts_with('+') {
        return;
    }
    let mut r = st.rt.lock();
    if r.phase != Phase::Afk && r.phase != Phase::Dropping {
        return;
    }
    if t.contains('$') {
        if let Some(v) = parse_money(t) {
            r.earned_session += v;
            r.earned_total += v;
            if let Some(b) = &mut r.balance {
                *b += v;
            }
            drop(r);
            st.log(2, format!("+{v}$"));
        }
    } else if t.to_lowercase().contains("klucz") {
        r.keys_total += 1;
        drop(r);
        st.log(1, format!("klucz: {t}"));
    }
}

fn on_actionbar(bot: &Client, st: &State, t: &str) {
    if t.contains("Zacznij si\u{119} porusza\u{107}") {
        start_walk(bot.clone(), st.clone());
    }
    if t.contains("Zapraszam do wyboru") {
        let go = {
            let mut r = st.rt.lock();
            let g = matches!(r.phase, Phase::Verify | Phase::Auth | Phase::Connecting);
            if g {
                r.walking = false;
            }
            g
        };
        if go {
            st.set_phase(Phase::Lobby);
            st.log(2, "zweryfikowano — wybór trybu");
            let (b, s) = (bot.clone(), st.clone());
            later(jitter(1500), move || {
                let mut b2 = b.clone();
                b2.walk(WalkDirection::None);
                use_item(&b, &s);
            });
            // compass may not register first time — retry
            let (b, s) = (bot.clone(), st.clone());
            later(9000, move || {
                if s.phase() == Phase::Lobby && s.rt.lock().window.is_none() {
                    use_item(&b, &s);
                }
            });
        }
    }
}

fn on_chat(bot: &Client, st: &State, t: &str) {
    let low = t.to_lowercase();
    // never log other players' chat, only server messages
    let player_chat = t.contains(" \u{bb} ") || t.contains(" >> ");
    if !player_chat {
        st.log(3, format!("chat: {t}"));
    }
    if player_chat {
        return;
    }
    let pw = st.rt.lock().password.clone();
    if let Some(code) = register_code(t) {
        if !std::mem::replace(&mut st.rt.lock().auth_sent, true) {
            st.log(1, "rejestracja");
            let b = bot.clone();
            later(jitter(2200), move || b.send_command_packet(&format!("register {pw} {pw} {code}")));
        }
        return;
    }
    if t.contains("/login") && !low.contains("register") {
        if !std::mem::replace(&mut st.rt.lock().auth_sent, true) {
            st.log(2, "logowanie");
            let b = bot.clone();
            later(jitter(2200), move || b.send_command_packet(&format!("login {pw}")));
        }
        return;
    }
    if low.contains("nieprawid") && low.contains("has") {
        st.rt.lock().last_reason = "złe hasło".into();
        st.log(1, "serwer: złe hasło — konto wyłączone");
        app().set_enabled(&[st.name.to_string()], false);
        return;
    }
    if t.contains("zarabiasz") && t.contains("AFK") {
        st.set_phase(Phase::Afk);
        {
            let mut r = st.rt.lock();
            r.reached_afk = true;
            r.afk_tries = 0;
        }
        st.log(1, "AFK — zarabia");
        return;
    }
    if low.contains("otrzyma") && low.contains("klucz") && !low.contains("gracz ") {
        st.rt.lock().keys_total += 1;
        st.log(1, format!("klucz: {t}"));
        return;
    }
    if let Some(v) = (low.contains("saldo") || low.contains("stan konta") || low.contains("posiadasz")).then(|| parse_money(t)).flatten() {
        st.rt.lock().balance = Some(v);
    }
    if low.contains("teleportowano") || low.contains("przeteleportowano") {
        st.rt.lock().phase_at = Some(Instant::now());
    }
}

fn answer_anvil(bot: &Client, st: &State, name: String) {
    let _ = bot.write_packet(ServerboundRenameItem { name: name.clone() });
    st.log(3, format!("kowadło -> '{name}'"));
    st.rt.lock().anvil_id = None;
}

fn register_code(t: &str) -> Option<String> {
    let i = t.find("/register <has")?;
    t[i..].split_whitespace().find(|w| w.len() >= 4 && w.chars().all(|c| c.is_ascii_digit())).map(str::to_owned)
}

fn start_walk(bot: Client, st: State) {
    {
        let mut r = st.rt.lock();
        if r.walking {
            return;
        }
        r.walking = true;
    }
    st.set_phase(Phase::Verify);
    tokio::spawn(async move {
        let mut b = bot.clone();
        // the movement check needs a rotation before any step
        let d = b.component::<azalea::entity::LookDirection>();
        let yaw = d.y_rot + rand::thread_rng().gen_range(25.0..60.0);
        b.set_direction(yaw, rand::thread_rng().gen_range(-5.0..10.0));
        tokio::time::sleep(Duration::from_millis(jitter(450))).await;
        b.walk(WalkDirection::Forward);
        for i in 0..24 {
            tokio::time::sleep(Duration::from_millis(1200)).await;
            if !st.rt.lock().walking {
                break;
            }
            let d = b.component::<azalea::entity::LookDirection>();
            b.set_direction(d.y_rot + rand::thread_rng().gen_range(15.0..35.0), 0.0);
            if i % 3 == 0 {
                b.jump();
            }
        }
        b.walk(WalkDirection::None);
        st.rt.lock().walking = false;
    });
}

#[derive(Clone)]
pub struct InvItem {
    pub slot: usize,
    pub id: String,
    pub count: i32,
    pub title: String,
    pub lore: String,
}

pub struct Item {
    pub slot: usize,
    pub id: String,
    pub title: String,
    pub lore: String,
}

async fn on_window(bot: Client, st: State, cid: i32, sid: u32, items: Vec<Item>) {
    tokio::time::sleep(Duration::from_millis(jitter(500))).await;
    let phase = st.phase();
    let find = |f: &dyn Fn(&Item) -> bool| items.iter().find(|x| f(x)).map(|x| x.slot);
    if let Some(s) = find(&|x| x.id == "green_concrete" && x.title.contains("Akceptuj")) {
        st.log(2, "regulamin — akceptuj");
        click(&bot, cid, sid, s, 0, ClickType::Pickup);
    } else if let Some(s) = find(&|x| x.title.contains("AnarchiaSMP")) {
        st.log(2, "tryb -> AnarchiaSMP");
        st.set_phase(Phase::Transfer);
        st.rt.lock().walking = false;
        click(&bot, cid, sid, s, 0, ClickType::Pickup);
    } else if let Some(s) = find(&|x| x.id == "nether_star" && x.title.contains("strefy AFK")) {
        st.log(2, "menu AFK -> strefa");
        st.set_phase(Phase::AfkMenu);
        click(&bot, cid, sid, s, 0, ClickType::Pickup);
    } else if phase == Phase::Settings && items.iter().any(|x| x.title.contains("Zamknij")) {
        settings_step(&bot, &st, cid, sid, &items).await;
    } else {
        st.log(3, format!("okno {cid}: {}", items.iter().take(12).map(|x| format!("{}:{}", x.slot, x.title)).collect::<Vec<_>>().join(", ")));
    }
}

/// Settings window: toggle every option that is ON (status line in lore). One click per window refresh.
async fn settings_step(bot: &Client, st: &State, cid: i32, sid: u32, items: &[Item]) {
    let on: Vec<&Item> = items.iter().filter(|x| x.lore.contains("Status: W\u{141}\u{104}CZONE") && !x.title.contains("VoiceChat") && !x.title.contains("Scoreboard")).collect();
    let clicks = st.rt.lock().settings_clicks;
    if on.is_empty() || clicks > 30 {
        st.log(1, format!("/settings wyłączone ({clicks} zmian)"));
        let _ = bot.write_packet(ServerboundContainerClose { container_id: cid });
        app().mark_settings_done(&st.name);
        {
            let mut r = st.rt.lock();
            r.window = None;
            r.handled_window = None;
        }
        let (b, s) = (bot.clone(), st.clone());
        later(jitter(1500), move || send_afk(b, s));
        return;
    }
    let it = on[0];
    st.rt.lock().settings_clicks += 1;
    st.log(3, format!("/settings: wyłączam '{}'", it.title));
    // server asks for a RIGHT click to toggle
    click(bot, cid, sid, it.slot, 1, ClickType::Pickup);
    // menu re-opens / refreshes; allow the next refresh to be handled
    tokio::time::sleep(Duration::from_millis(jitter(350))).await;
    st.rt.lock().handled_window = None;
    // if no refresh arrives, re-open
    let (b, s) = (bot.clone(), st.clone());
    let before = s.rt.lock().settings_clicks;
    later(4000, move || {
        if s.phase() == Phase::Settings && s.rt.lock().settings_clicks == before {
            b.send_command_packet("settings");
        }
    });
}

fn after_smp_join(bot: Client, st: State) {
    if st.phase() != Phase::Smp {
        return;
    }
    if app().cfg.read().settings_off && !app().settings_done(&st.name) {
        st.set_phase(Phase::Settings);
        st.log(2, "/settings");
        bot.send_command_packet("settings");
        let (b, s) = (bot.clone(), st.clone());
        later(45000, move || {
            if s.phase() == Phase::Settings {
                s.log(1, "/settings: brak odpowiedzi — pomijam");
                send_afk(b, s);
            }
        });
    } else {
        send_afk(bot, st);
    }
}

pub fn send_afk(bot: Client, st: State) {
    st.set_phase(Phase::AfkMenu);
    let tries = {
        let mut r = st.rt.lock();
        r.afk_tries += 1;
        r.afk_cmd_at = Some(Instant::now());
        r.afk_tries
    };
    st.log(3, "/afk");
    bot.send_command_packet("afk");
    let (b, s) = (bot.clone(), st.clone());
    later(jitter(12000), move || {
        if s.phase() == Phase::AfkMenu && tries < 6 && s.rt.lock().afk_tries == tries {
            send_afk(b, s);
        }
    });
}

/// Hand over cheques (money) and crate keys to `target`: /czek, /tpa, wait until close, look, throw.
pub async fn drop_to(bot: Client, st: State, target: String) {
    if std::mem::replace(&mut st.rt.lock().drop_busy, true) {
        return;
    }
    let prev = st.phase();
    st.set_phase(Phase::Dropping);
    let res = drop_inner(&bot, &st, &target).await;
    match &res {
        Ok((m, k)) => st.log(1, format!("oddano {target}: {m:.0}$ + {k} kluczy")),
        Err(e) => st.log(1, format!("oddawanie nieudane: {e}")),
    }
    st.rt.lock().drop_busy = false;
    // back to the AFK zone
    if prev == Phase::Afk || res.is_ok() {
        tokio::time::sleep(Duration::from_millis(1500)).await;
        send_afk(bot, st);
    } else {
        st.set_phase(prev);
    }
}

async fn drop_inner(bot: &Client, st: &State, target: &str) -> anyhow::Result<(f64, u32)> {
    if !crate::app::valid_name(target) {
        anyhow::bail!("zły nick");
    }
    // 1) money → cheque (only if there is any)
    let bal = st.rt.lock().balance;
    let min = app().cfg.read().min_cheque as f64;
    let amount = bal.unwrap_or(0.0).floor();
    let keys_in_inv = st.rt.lock().inv.iter().any(|i| is_key(&i.id, &i.title, &i.lore) || is_cheque(&i.id, &i.title, &i.lore));
    let cheque = amount >= min.max(1.0);
    if !cheque && !keys_in_inv {
        anyhow::bail!("nic do oddania (saldo {amount:.0}$ < {min:.0}$, brak kluczy)");
    }
    if cheque {
        bot.send_command_packet(&format!("czek {}", amount as u64));
        tokio::time::sleep(Duration::from_millis(1500)).await;
    }
    // 2) go to spawn then /tpa target
    bot.send_command_packet("spawn");
    tokio::time::sleep(Duration::from_millis(6000)).await;
    bot.send_command_packet(&format!("tpa {target}"));
    st.log(2, format!("/tpa {target} — czekam na akceptację"));
    // 3) wait for the target to be within 6 blocks (max 90s)
    let deadline = Instant::now() + Duration::from_secs(90);
    let target_pos = loop {
        if Instant::now() > deadline {
            anyhow::bail!("{target} nie zaakceptował /tpa");
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
        if let Some(p) = player_pos(bot, target) {
            if p.distance_to(&bot.position()) < 6.0 {
                break p;
            }
        }
    };
    tokio::time::sleep(Duration::from_millis(800)).await;
    let mut b = bot.clone();
    b.look_at(target_pos.up(1.0));
    tokio::time::sleep(Duration::from_millis(300)).await;
    // 4) throw every cheque and key stack from the player inventory
    let inv = st.rt.lock().inv.clone();
    let mut money = 0.0;
    let mut keys = 0u32;
    for it in inv {
        if it.slot < 9 || it.slot > 45 {
            continue; // crafting/armor
        }
        let cheque = is_cheque(&it.id, &it.title, &it.lore);
        let key = is_key(&it.id, &it.title, &it.lore);
        if !(cheque || key) {
            continue;
        }
        if cheque {
            money += parse_money(&format!("{} {}", it.title, it.lore)).unwrap_or(0.0) * it.count as f64;
        } else {
            keys += it.count as u32;
        }
        let slot = it.slot;
        let sid = st.rt.lock().inv_state;
        // ctrl+Q on the slot = throw whole stack
        click(bot, 0, sid, slot, 1, ClickType::Throw);
        tokio::time::sleep(Duration::from_millis(jitter(250))).await;
    }
    if money == 0.0 && cheque {
        money = amount;
    }
    {
        let mut r = st.rt.lock();
        r.dropped_money += money;
        r.dropped_keys += keys;
        r.earned_session = 0.0;
        r.balance = r.balance.map(|b| (b - money).max(0.0));
    }
    Ok((money, keys))
}

fn player_pos(bot: &Client, name: &str) -> Option<azalea::Vec3> {
    let tab = bot.tab_list();
    let uuid = tab.values().find(|p| p.profile.name.eq_ignore_ascii_case(name))?.uuid;
    let mut ecs = bot.ecs.lock();
    let mut q = ecs.query::<(&azalea::entity::EntityUuid, &azalea::entity::Position, &azalea::world::InstanceName)>();
    let my_world = ecs.get::<azalea::world::InstanceName>(bot.entity).cloned()?;
    q.iter(&ecs).find(|(u, _, w)| ***u == uuid && **w == my_world).map(|(_, p, _)| azalea::Vec3::from(p))
}

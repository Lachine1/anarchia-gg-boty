// Copyright (c) 2026 lachine. AGPL v3 + Commons Clause - see LICENSE.md
//! Decode every packet dumped by AZ_DUMP_BAD (bad/*.bin); prints failures by packet type.
use std::io::Cursor;
use azalea::protocol::{packets::game::ClientboundGamePacket, read::deserialize_packet};

#[test]
#[ignore]
fn sweep_bad_dir() {
    let dir = std::env::var("SWEEP_DIR").unwrap_or_else(|_| "bad".into());
    let mut fails = std::collections::BTreeMap::<String, (u32, String)>::new();
    let mut n = 0;
    for e in std::fs::read_dir(&dir).unwrap() {
        let b = std::fs::read(e.unwrap().path()).unwrap();
        n += 1;
        if let Err(err) = deserialize_packet::<ClientboundGamePacket>(&mut Cursor::new(&b)) {
            let s = format!("{err:?}");
            let k = s.split("packet_name: ").nth(1).and_then(|x| x.split(',').next()).unwrap_or("?").to_string();
            let ent = fails.entry(k).or_insert((0, s.chars().take(300).collect()));
            ent.0 += 1;
        }
    }
    println!("swept {n}");
    for (k, (c, s)) in &fails { println!("FAIL {k} x{c}: {s}"); }
}

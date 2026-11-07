// Copyright (c) 2026 lachine. AGPL v3 + Commons Clause - see LICENSE.md
//! Packets captured live from anarchia.gg that stock azalea 0.11 fails to decode.
use std::io::Cursor;
use azalea::protocol::{packets::game::ClientboundGamePacket, read::deserialize_packet};

fn decode(bytes: &[u8]) -> ClientboundGamePacket {
    deserialize_packet::<ClientboundGamePacket>(&mut Cursor::new(bytes)).expect("decode")
}

#[test]
fn mode_selector_hopper() {
    let p = decode(include_bytes!("fixtures/mode_selector_set_content.bin"));
    let ClientboundGamePacket::ContainerSetContent(c) = p else { panic!("wrong packet") };
    assert!(c.items.iter().any(|i| i.is_present()));
}

#[test]
fn set_equipment() {
    decode(include_bytes!("fixtures/set_equipment.bin"));
}

#[test]
fn sound() {
    decode(include_bytes!("fixtures/sound.bin"));
}

#[test]
fn set_equipment_enchanted() {
    decode(include_bytes!("fixtures/set_equipment2.bin"));
}

#[test]
fn set_entity_data() {
    decode(include_bytes!("fixtures/set_entity_data.bin"));
}

#[test]
fn settings_window() {
    let p = decode(include_bytes!("fixtures/settings_set_content.bin"));
    let ClientboundGamePacket::ContainerSetContent(c) = p else { panic!("wrong packet") };
    assert!(c.items.iter().filter(|i| i.is_present()).count() > 20);
}

#[test]
fn entity_effect_particles() {
    decode(include_bytes!("fixtures/set_entity_data_small.bin"));
}

#[test]
fn display_entity_with_item() {
    decode(include_bytes!("fixtures/set_entity_data_big.bin"));
}

#[test]
fn equipment_with_potion_and_modifiers() {
    decode(include_bytes!("fixtures/set_equipment_potion.bin"));
}

#[test]
fn chat_extra_not_duplicated() {
    // {"text":"", "extra":[{"text":"Zostałeś pomyślnie "},{"text":"zalogowany!"}]}
    use azalea::buf::AzaleaRead;
    let mut nbt = simdnbt::owned::NbtCompound::new();
    nbt.insert("text", "");
    nbt.insert("extra", simdnbt::owned::NbtList::Compound(vec![
        { let mut c = simdnbt::owned::NbtCompound::new(); c.insert("text", "a "); c },
        { let mut c = simdnbt::owned::NbtCompound::new(); c.insert("text", "b"); c },
    ]));
    let mut buf = Vec::new();
    simdnbt::owned::NbtTag::Compound(nbt).write(&mut buf);
    let t = azalea::FormattedText::azalea_read(&mut Cursor::new(&buf[..])).unwrap();
    assert_eq!(t.to_string(), "a b");
}

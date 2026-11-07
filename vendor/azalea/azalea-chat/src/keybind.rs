//! Keybind text components, resolved the way a vanilla client with default
//! controls renders them (KeyMapping defaults + en_us key names).

fn default_key(name: &str) -> Option<&'static str> {
    Some(match name {
        "key.forward" => "key.keyboard.w",
        "key.left" => "key.keyboard.a",
        "key.back" => "key.keyboard.s",
        "key.right" => "key.keyboard.d",
        "key.jump" => "key.keyboard.space",
        "key.sneak" => "key.keyboard.left.shift",
        "key.sprint" => "key.keyboard.left.control",
        "key.inventory" => "key.keyboard.e",
        "key.swapOffhand" => "key.keyboard.f",
        "key.drop" => "key.keyboard.q",
        "key.use" => "key.mouse.right",
        "key.attack" => "key.mouse.left",
        "key.pickItem" => "key.mouse.middle",
        "key.chat" => "key.keyboard.t",
        "key.playerlist" => "key.keyboard.tab",
        "key.command" => "key.keyboard.slash",
        "key.socialInteractions" => "key.keyboard.p",
        "key.screenshot" => "key.keyboard.f2",
        "key.togglePerspective" => "key.keyboard.f5",
        "key.smoothCamera" => "key.keyboard.unknown",
        "key.fullscreen" => "key.keyboard.f11",
        "key.spectatorOutlines" => "key.keyboard.unknown",
        "key.advancements" => "key.keyboard.l",
        "key.saveToolbarActivator" => "key.keyboard.c",
        "key.loadToolbarActivator" => "key.keyboard.x",
        "key.hotbar.1" => "key.keyboard.1",
        "key.hotbar.2" => "key.keyboard.2",
        "key.hotbar.3" => "key.keyboard.3",
        "key.hotbar.4" => "key.keyboard.4",
        "key.hotbar.5" => "key.keyboard.5",
        "key.hotbar.6" => "key.keyboard.6",
        "key.hotbar.7" => "key.keyboard.7",
        "key.hotbar.8" => "key.keyboard.8",
        "key.hotbar.9" => "key.keyboard.9",
        _ => return None,
    })
}

/// Render a keybind name (e.g. `key.inventory`) to its display string ("E").
pub fn resolve(name: &str) -> String {
    let Some(key) = default_key(name) else {
        // unknown mapping: vanilla shows the raw translation of the name
        return azalea_language::get(name).unwrap_or(name).to_string();
    };
    if let Some(s) = azalea_language::get(key) {
        return s.to_string();
    }
    // printable keys have no lang entry; vanilla upper-cases the glyph
    let glyph = key.rsplit('.').next().unwrap_or(key);
    match glyph {
        "slash" => "/".to_string(),
        g => g.to_uppercase(),
    }
}

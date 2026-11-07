<!-- Copyright (c) 2026 lachine. AGPL v3 + Commons Clause - see LICENSE.md -->
# anarchia-boty v2

Boty AFK dla anarchia.gg (1.21.4). Jeden plik, bez Node, bez config.yaml.

## Pobierz
Z zakładki **Releases**:
- `anarchia-boty.exe` — Windows (okno aplikacji)
- `anarchia-boty-linux-x86_64` — Linux / VPS (statyczny)
- `anarchia-boty-linux-aarch64` — Termux / ARM VPS (statyczny)

## Start
- Windows: dwuklik `anarchia-boty.exe`.
- Linux z pulpitem: `./anarchia-boty-linux-x86_64` (okno Chrome/Chromium/Edge w trybie aplikacji).
- VPS / Termux: `./anarchia-boty-linux-aarch64 --headless` → otwórz wypisany link
  (`--bind 0.0.0.0:8765` żeby wejść z innego urządzenia; link zawiera token dostępu).

Potem w panelu: **Ustawienia** → hasło kont, **Konta** → nicki (jeden w linii, opcjonalnie `nick:hasło`),
**Proxy** (opcjonalnie) → SOCKS5, max 25 botów na IP.

## Co robią boty
Logowanie/rejestracja → weryfikacja (kowadło + ruch) → AnarchiaSMP → wyłączenie `/settings` (raz na konto)
→ `/afk` → zarabianie. Saldo czytane ze scoreboardu, klucze z ekwipunku.
**Oddaj**: `/czek` z salda (od kwoty minimalnej), `/tpa` do odbiorcy, wyrzucenie czeków i kluczy.
Auto-oddawanie co N minut w Ustawieniach.

Stabilność: proces nadzorcy restartuje boty po awarii, watchdog pakietów/faz, backoff przy reconnectach.
Dane trzymane w `data/` obok pliku.

## Budowanie
```
cargo build --release                                   # host
cargo build --release --target x86_64-pc-windows-gnu    # Windows (mingw)
cargo build --release --target aarch64-unknown-linux-musl  # Termux (zig cc, tools/)
cargo test --test decode                                # testy dekodowania pakietów 1.21.4
```
Biblioteka: [azalea](https://github.com/azalea-rs/azalea) 0.11 w `vendor/` z poprawkami 1.21.4.

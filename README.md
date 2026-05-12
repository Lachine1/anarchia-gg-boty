# Anarchia.gg Boty

Manager wielu kont Minecraft na serwerze **anarchia.gg** z terminalowym interfejsem (TUI) i integracją Discord. Stworzony dla operatorów, którzy chcą obsługiwać do **45 botów** jednocześnie z jednego komputera lub VPS.

> Projekt jest **open source** na licencji **AGPL v3 + Commons Clause**.
> **Możesz** korzystać z niego do własnych celów, samodzielnie hostować i modyfikować.
> **Nie możesz** sprzedawać go, odsprzedawać dostępu, ani świadczyć płatnych usług których wartość pochodzi z tego oprogramowania. Zobacz [Licencja](#licencja).

---

## Funkcje

- Obsługa do **45 botów** jednocześnie (twardy limit narzucony przez serwer)
- **Auto-login / auto-register** dla każdego konta na podstawie wspólnego hasła
- **Auto-reconnect** z rosnącym opóźnieniem po rozłączeniu (do *N* prób na bota)
- **Tryb AFK** z opcjonalnym śledzeniem zarobków per bot
- **Drop** zarobionej kasy (per bot lub `all`)
- **TPA / collect** wysyłane z wybranego bota lub wszystkich naraz
- **Czat zbiorowy** — wysyłanie wiadomości z jednego bota lub `all`
- **Terminalowy interfejs (TUI)** — pasek postępu, statystyki botów, logi w czasie rzeczywistym
- **Integracja Discord** — okresowe statystyki przez webhook, powiadomienia o kickach, alert startowy
- Konfiguracja w prostym pliku **`config.yaml`**

---

## Wymagania

- **Node.js** 18+ (zalecane 20+)
- **npm** (dołączony do Node.js)
- Terminal obsługujący UTF-8 i kolory (Windows Terminal, iTerm2, większość terminali Linux)
- Konta Minecraft kompatybilne z `anarchia.gg`
- (Opcjonalnie) Discord webhook do powiadomień

---

## Instalacja

### Windows

Najprostsza droga — uruchom dołączony skrypt:

```bat
instalacja.bat
```

Skrypt zainstaluje wszystkie zależności (`mineflayer`, `js-yaml`, `minecraft-data`, `minecraft-protocol`, `blessed`).

### Linux / macOS

```bash
git clone https://github.com/Lachine1/anarchia-gg-boty.git
cd anarchia-gg-boty
npm install mineflayer js-yaml minecraft-data minecraft-protocol blessed
```

---

## Konfiguracja

Edytuj plik **`config.yaml`** w głównym folderze projektu. Wszystkie pola są opisane komentarzami:

```yaml
# Autoryzacja - hasło używane przy /login i /register
auth:
  password: "twoje_hasło"

# Discord Webhook (opcjonalne)
discord:
  enabled: true
  webhook_url: "https://discord.com/api/webhooks/..."
  bot_name: "Odpalone Boty"
  notify_kicks: true              # powiadom o wyrzuceniu bota

# Opóźnienia (milisekundy)
delays:
  connection: 5200                # między startami kolejnych botów
  spawn_command: 5100             # po wykryciu spawnu przed komendą
  spawn_retry: 8000               # przed ponowieniem /spawn
  afk_retry: 5000                 # przed ponowieniem /afk
  tpa_timeout: 60000              # timeout dla TPA
  stats_display_console: 60000    # odświeżanie statystyk w konsoli
  stats_display_discord: 300000   # raport na Discord (5 min)
  bot_timeout: 120000             # timeout dołączenia botów
  connect_timeout: 120000

reconnect:
  max_attempts: 50                # maks. prób reconnectu na bota

# Zachowanie
behavior:
  drop_distance: 5                # blok od gracza do wyrzucania czeków
  min_money_drop: 1000            # minimalna kasa do dropu ($)
  estimated_earnings: false       # false = z tytułów, true = ze scoreboardu
  earnings_interval: 5            # co ile minut bot dostaje hajs

# Logowanie
logging:
  level: 1                        # 1 = podstawowe, 2 = szczegółowe, 3 = debug
  colors: true

# Lista botów (maksymalnie 45)
bots:
  accounts:
    - Bot01
    - Bot02
    - Bot03
    # ...dodaj więcej tutaj
```

> **Uwaga:** Zbyt niskie `delays.connection` może doprowadzić do bana IP. Wartości domyślne są bezpieczne — modyfikuj świadomie.

> **Bezpieczeństwo:** `config.yaml` zawiera hasło i webhook — **nigdy** nie commituj go do publicznego repo. Zalecane dodać go do `.gitignore`.

---

## Uruchomienie

### Windows

```bat
start.bat
```

### Linux / macOS

```bash
node boty.js
```

Po starcie zobaczysz terminalowy interfejs z paskiem postępu, statystykami i logami. Komendy wpisujesz w polu na dole ekranu.

---

## Komendy

Większość komend przyjmuje albo nazwę konkretnego bota, albo `all` (dla wszystkich aktywnych).

| Komenda | Opis |
|---------|------|
| `help` / `h` | Lista wszystkich komend |
| `tpa <bot\|all> <gracz>` | Wyślij TPA od bota (lub wszystkich) do gracza |
| `collect <gracz>` | Skrót: TPA od **wszystkich** botów do gracza |
| `talk <bot\|all> <wiadomość>` (alias `t`) | Wyślij wiadomość na czat |
| `money <bot\|all>` (alias `m`) | Pokaż stan kasy bota / sumę wszystkich |
| `drop <bot\|all>` | Wyrzuć kasę z bota / wszystkich |
| `afk <bot\|all>` | Wróć bota / wszystkich do trybu AFK |
| `retry` | Ponów boty, które utknęły w menu AFK |
| `cancel` | Anuluj aktywne TPA i wróć do AFK |

---

## Licencja

**Copyright © 2026 lachine.** Projekt udostępniony na **AGPL v3** z warunkiem **Commons Clause** — pełny tekst w [`LICENSE.md`](LICENSE.md).

### Co możesz

- Korzystać z bota do własnych celów (osobiście lub w społeczności)
- Modyfikować kod na własny użytek
- Dystrybuować zmodyfikowane wersje **pod tą samą licencją** z udostępnionym źródłem
- Hostować zmodyfikowaną wersję jako usługę sieciową — **pod warunkiem** udostępnienia źródła użytkownikom tej usługi

### Czego nie możesz

- **Sprzedawać** kodu ani dostępu do bota (zakaz z Commons Clause)
- Odsprzedawać na Sellix, Discord, ani żadnej innej platformie
- Świadczyć **płatnych usług**, których wartość wynika z tego oprogramowania
- Usuwać nagłówków licencyjnych, copyrightów ani atrybucji autora
- Sublicencjonować pod innymi warunkami

### Ostrzeżenie prawne

Powielanie, odsprzedaż lub relicencjonowanie tego oprogramowania bez upoważnienia stanowi przestępstwo zgodnie z **art. 115–117 ustawy z dnia 4 lutego 1994 r. o prawie autorskim i prawach pokrewnych**, zagrożone karą:

- do **2 lat pozbawienia wolności** — za nieuprawnione zwielokrotnianie lub rozpowszechnianie
- do **3 lat** — w przypadku działania w celu osiągnięcia korzyści majątkowej (np. sprzedaż na Sellixie, Discordzie)
- do **5 lat** — za działalność zorganizowaną lub w sposób powtarzający się

Każdy przypadek nieautoryzowanej redystrybucji może skutkować zgłoszeniem **DMCA** oraz formalną skargą do **CERT Polska**.

---

## Wsparcie

- Discord: [dc.gg/odpalonyconfig](https://dc.gg/odpalonyconfig)
- Bugi i propozycje: [GitHub Issues](https://github.com/Lachine1/anarchia-gg-boty/issues)

---

## Autor

**lachine** — [@Lachine1](https://github.com/Lachine1)

# 📅 Högskolan i Borås (HB) - Schema TUI

Ett modernt, snabbt och lättnavigerat terminalbaserat gränssnitt (TUI) skrivet i **Rust** för att visa, söka i och navigera scheman från Högskolan i Borås ([schema.hb.se / KronoX](https://schema.hb.se)).

---

## ✨ Funktioner

- 🚀 **Blixtsnabb & Offline-redo**: Smart lokal cachning (`cache_ttl_minutes`), schemat laddas omedelbart även utan aktiv internetuppkoppling.
- 🎯 **Överskådliga Vyer**:
  - **Veckovy (`1` / `Tab`)**: Visar veckans alla lektioner, labbar och föreläsningar med enkel bläddring mellan veckor.
  - **Alla händelser (`2`)**: Komplett tidslinje över hela terminen/läsåret.
  - **Dagvy (`3`)**: Fokusera på en enskild dag i taget.
  - **Kursöversikt (`4`)**: Sammanställning och statistik (antal föreläsningar, övningstimmar, laborationer, lokaler och lärare per kurs).
- 🔍 **Live Fritextsökning (`/`)**: Sök omedelbart efter kurser, lokaler (t.ex. *M404*, *J517*), lärare (*FAGO*, *ULM*) eller moment.
- 👥 **Gruppfiltrering (`g` / `f`)**: Kryssrutebaserad gruppfiltrering där du kan välja en eller flera grupper samtidigt (t.ex. både *Grupp 1* för övningar och *Grupp A* för laborationer). Gemensamma föreläsningar visas alltid.
- 🏢 **Campus- & Lokalguide**: Visar automatiskt vilket hus och våningsplan en sal tillhör (t.ex. `J517 (Hus J - Balder, Plan 5)`, `M404 (Hus M - Sandgärdet, Plan 4)`).
- 🔄 **Flera Schemaprofiler (`s`)**: Konfigurera flera scheman i `config.toml` och växla mellan dem med en knapptryckning.
- 🌐 **Webb- & Länkintegration**:
  - Tryck `o` för att öppna schemat i webbläsaren.
  - Tryck `w` för att öppna länkar i den markerade aktiviteten (t.ex. Studentkårens insparksschema).
- 📅 **iCal Export**: Ladda enkelt ner eller exportera kalenderfil (`.ics`).
- ⚡ **Snabbkommandon för Terminalen**: Kör `--today` eller `--week 36` för att få ut schemat direkt i skalet utan att öppna TUI:t.

---

## ⌨️ Kortkommandon

| Tangent | Åtgärd |
|---|---|
| `↑` / `↓` eller `k` / `j` | Flytta markering upp / ner |
| `←` / `→` eller `h` / `l` / `p` / `n` | Föregående / Nästa vecka (eller dag i dagvyn) |
| `t` | Hoppa direkt till idag / aktuell vecka |
| `w` | Gå till ett specifikt veckonummer |
| `Tab` / `BackTab` | Växla mellan vyer (Vecka -> Alla -> Dag -> Kurser) |
| `1`, `2`, `3`, `4` | Gå direkt till vy 1, 2, 3 eller 4 |
| `/` | Fritextsökning i schemat |
| `g` eller `f` | Gruppfilter med kryssrutor (`Mellanslag` växlar, `1-9` snabbval, `c` rensa, `a` alla) |
| `Enter` eller `d` | Öppna stor detaljruta för markerad händelse |
| `s` | Växla schemaprofil |
| `r` | Tvinga omladdning/uppdatering från HB:s server |
| `o` | Öppna schemat i extern webbläsare |
| `w` / `u` | Öppna webblänk i markerad aktivitet |
| `?` / `F1` | Visa hjälpruta med alla kommandon |
| `Esc` | Rensa sökning / Stäng popup |
| `q` | Avsluta programmet |

---

## 🛠️ Installation & Körning

Kompilera och starta programmet:

```bash
# Starta det interaktiva TUI-programmet
cargo run --release

# Eller kör den färdiga binären direkt
./target/release/hb-schema
```

### CLI-alternativ

```bash
# Visa dagens schema i terminalen och avsluta
cargo run -- --today

# Visa schema för en specifik vecka (t.ex. vecka 36)
cargo run -- --week 36

# Exportera iCal (.ics) kalenderfil
cargo run -- --export-ical mitt_schema.ics

# Starta TUI med en specifik profil från konfigurationen
cargo run -- --profile dataing

# Starta med en anpassad KronoX-länk
cargo run -- --url "https://schema.hb.se/setup/jsp/Schema.jsp?startDatum=2026-08-31&intervallTyp=a&intervallAntal=1&sprak=SV&sokMedAND=true&forklaringar=true&resurser=p.KBAST26h"
```

---

## ⚙️ Konfiguration (`config.toml`)

Konfigurationen laddas automatiskt från `./config.toml` eller `~/.config/hb-schema/config.toml`:

```toml
default_profile = "tekniskt_basar"
cache_ttl_minutes = 180
theme = "modern"

[profiles.tekniskt_basar]
name = "Tekniskt Basår (KBAST26h)"
url = "https://schema.hb.se/setup/jsp/Schema.jsp?startDatum=2026-08-31&intervallTyp=a&intervallAntal=1&sprak=SV&sokMedAND=true&forklaringar=true&resurser=p.KBAST26h"
group_filter = "alla"
description = "Högskolan i Borås - Tekniskt basår HT26/VT27"

[profiles.dataing]
name = "Högskoleingenjör Datateknik (TGITI26h)"
url = "https://schema.hb.se/setup/jsp/Schema.jsp?startDatum=2026-08-31&intervallTyp=a&intervallAntal=1&sprak=SV&sokMedAND=true&forklaringar=true&resurser=p.TGITI26h"
group_filter = "alla"
description = "Högskolan i Borås - Datateknik HT26/VT27"
```

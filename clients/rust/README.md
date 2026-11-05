# Mäxchen-Client in Rust

Ausgangsbasis für eigene Bots. Der Client kümmert sich um alles Technische (Verbindung,
Registrierung, Heartbeats, Abmeldung bei Ctrl-C). Du schreibst nur, was dein Bot auf
Nachrichten des Servers antwortet. Das Protokoll steht in `protokoll.md` im
Verzeichnis darüber.

## Starten

Du brauchst Rust ab 1.88 (am einfachsten über [rustup](https://rustup.rs/)). Im Verzeichnis
`rust/`:

```sh
cargo run -- -i <server-ip> -p <port> -n <dein-name> -t simple
```

Server und Port nennt dir der Veranstalter. Der erste Aufruf lädt und übersetzt die
Abhängigkeiten und dauert etwas länger. Ctrl-C beendet den Bot und meldet ihn beim Server ab.

Für einen schnellen Start:

1. Starte den `simple`-Bot und schau ins Log. Wenn `REGISTERED` erscheint, bist du dabei.
2. Starte in einem zweiten Terminal einen weiteren Bot mit anderem Namen, zum Beispiel mit
   `-t random`. Ein Spiel braucht mindestens zwei Spieler.
3. Mit `-t spectator` schaust du nur zu und siehst alle Nachrichten im Log.

## Optionen

| Option | Bedeutung |
|---|---|
| `-i`, `--ip` | Adresse des Servers (Standard `127.0.0.1`) |
| `-p`, `--port` | Port des Servers (Standard `9000`) |
| `-n`, `--name` | Spielername; ohne Angabe wird einer mit Zufallszahl erzeugt. Er darf höchstens 20 Zeichen lang sein und keine Leerzeichen und keines der Zeichen `,` `;` `:` enthalten. Ist der Name schon vergeben, lehnt der Server dich ab |
| `-t`, `--type` | `simple`, `random` oder `spectator` (Standard `simple`) |
| `--no-auto-join` | Nicht automatisch an Runden teilnehmen: `ROUND STARTING` kommt dann bei deinem Bot an, und er muss selbst `JOIN` senden |
| `--no-watch-heartbeat` | Nicht aufgeben, wenn der Server keine Heartbeats mehr sendet |
| `-v`, `-q` | Mehr oder weniger Log-Ausgabe |

`cargo run -- --help` zeigt die Liste mit allen Standardwerten.

## Das Log lesen

```
2026-11-05 21:15:03,102 | INFO     | receive              | <---- YOUR TURN ['4711']
2026-11-05 21:15:03,103 | INFO     | send                 | ----> ROLL ['4711']
```

`<----` ist eine Nachricht des Servers, `---->` eine Nachricht deines Bots. In den eckigen
Klammern stehen die Parameter. Wenn dein Bot verliert oder nichts passiert, siehst du hier
zuerst, was er gesendet hat und was der Server dazu sagt.

## Eigenen Bot schreiben

1. Lege eine Datei wie `src/simple_bot.rs` an und implementiere das Trait `Bot` aus
   `src/bot.rs`.
2. Deklariere das Modul in `src/lib.rs` und trage den Bot in `BotType` in `src/main.rs` ein.

`on_message` bekommt jede Nachricht des Servers (außer Heartbeats) in zwei Teilen:
`YOUR TURN;4711` kommt als `"YOUR TURN"` mit den Parametern `["4711"]` an. Antworten
schickst du mit `server.send("ROLL", &[token])`. Weil `on_message` `&mut self` hat, darf
dein Bot Zustand in Feldern halten und über mehrere Nachrichten hinweg merken.

Die beiden Beispiel-Bots zeigen den Rahmen:

- `simple_bot.rs` würfelt und sagt an, was gewürfelt wurde.
- `random_bot.rs` entscheidet zufällig zwischen `ROLL` und `SEE`.

## Ideen zum Weiterbauen

- **Zustand merken:** Welche Nachrichten sagen dir, wie die Runde steht? Denk an
  `ROUND STARTED`, `ANNOUNCED`, `PLAYER LOST` und `SCORE`.
- **Entscheiden statt würfeln:** Wann lohnt sich `SEE` statt `ROLL`? Was verrät dir die
  letzte Ansage über die Chancen?
- **Ansage und Wurf trennen:** Du musst nicht ansagen, was du gewürfelt hast. Was folgt
  daraus für deine Gegner, und was für dich?
- **Würfe vergleichen:** Die Reihenfolge der Würfe ist im Protokoll beschrieben. Ein
  kleiner Helfer, der zwei Würfe vergleicht, macht jede Strategie lesbarer.
- **Gegner beobachten:** Als `spectator` oder über die Nachrichten siehst du, wie andere
  spielen.

## Entwickeln

```sh
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
```

# Mäxchen: Material für Teilnehmende

Du schreibst einen Bot, der über UDP mit dem Mäxchen-Server spielt. Der Veranstalter nennt dir
Adresse und Port des Servers. Wie die Nachrichten aussehen, steht im Protokoll:
[Deutsch](protokoll.md) oder [English](protokoll.en.md). Die Aufgabe der Runde
steht in der [Spec](spec/runde-1.md); sie lässt sich auch einer KI als Vorgabe geben.

Such dir eine Sprache aus. In jedem Verzeichnis liegt ein lauffähiger Beispiel-Bot, den du als
Ausgangsbasis nutzt. Alle drei sind gleich aufgebaut: Ein kleiner Rahmen kümmert sich um
Verbindung, Registrierung, Heartbeats und Abmeldung, du schreibst nur eine Bot-Klasse mit einer
Methode `on_message` (Java: `onMessage`), die auf Nachrichten des Servers antwortet. Die Bots
`simple`, `random` und `spectator` gibt es in allen drei Sprachen.

Jedes Verzeichnis hat eine eigene `README.md` mit allen Optionen, einem schnellen Start und
Hinweisen zum Schreiben eines eigenen Bots.

| Verzeichnis | Voraussetzung | Starten |
|---|---|---|
| `python/` | Python ab 3.11, [uv](https://docs.astral.sh/uv/) | `cd python && uv run maexchen-bot -i <server-ip> -p <port> -n <dein-name> -t simple` |
| `java/` | JDK 21 | `cd java && ./gradlew run --args="-i <server-ip> -p <port> -n <dein-name> -t simple"` |
| `rust/` | Rust ab 1.88 | `cd rust && cargo run -- -i <server-ip> -p <port> -n <dein-name> -t simple` |

Der Name darf höchstens 20 Zeichen lang sein und keine Leerzeichen und keines der Zeichen
`,` `;` `:` enthalten. Dein Bot hat für jede Antwort 250 ms Zeit.

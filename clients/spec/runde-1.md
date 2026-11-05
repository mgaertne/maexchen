# Spec Runde 1: Mäxchen-Bot

Diese Datei beschreibt alles, was du (oder deine KI) für Runde 1 braucht. Sie steht für sich:
Du musst keine andere Datei lesen, um einen spielfähigen Bot zu bauen.

## Ziel

Schreibe einen Bot, der über UDP am Mäxchen-Spiel teilnimmt und möglichst selten verliert. Der
mitgelieferte `simple`-Bot würfelt immer und sagt immer die Wahrheit. Dein Bot soll besser
entscheiden als er.

## Spielregeln

Gespielt wird mit zwei Würfeln. Die Reihenfolge der Augen spielt keine Rolle: `1,2` und `2,1` sind
derselbe Wurf. Der Server schreibt Würfe immer mit dem höheren Würfel zuerst (`2,1`, nie `1,2`).

Von niedrig nach hoch:

1. **Normale Würfe** `3,1` (niedrigster), `3,2`, `4,1`, `4,2`, `4,3`, `5,1`, `5,2`, `5,3`, `5,4`,
   `6,1`, `6,2`, `6,3`, `6,4`, `6,5` (höchster normaler Wurf).
2. **Pasche** `1,1`, `2,2`, `3,3`, `4,4`, `5,5`, `6,6`. Jeder Pasch schlägt jeden normalen Wurf.
3. **Mäxchen** (auch Meier) `2,1`. Der höchste Wurf.

Reihum würfelt jeder Spieler und sagt einen Wurf an. Er muss nicht sagen, was er gewürfelt hat.
Wer an der Reihe ist, kann statt zu würfeln auch die letzte Ansage anzweifeln (`SEE`).

Die Ansage muss **echt höher** sein als die vorherige Ansage. Die erste Ansage einer Runde ist
frei. Eine gleich hohe oder niedrigere Ansage verliert sofort.

Eine Runde endet, sobald jemand verliert. **Jeder Spieler der Runde, der nicht verliert, bekommt
einen Punkt.**

### Wer verliert wann?

| Situation | Verlierer | Grund |
|---|---|---|
| Ansage nicht höher als die vorherige | der Ansagende | `ANNOUNCED_LOSING_DICE` |
| Mäxchen angesagt, aber nicht gewürfelt | der Ansagende | `LIED_ABOUT_MIA` |
| Mäxchen angesagt und gewürfelt | **alle anderen** Spieler | `MIA` |
| `SEE`, die letzte Ansage war gelogen (der echte Wurf ist niedriger) | der Spieler davor | `CAUGHT_BLUFFING` |
| `SEE`, die letzte Ansage stimmte oder war niedriger als der echte Wurf | der, der `SEE` sagte | `SEE_FAILED` |
| `SEE` als erster Zug der Runde | der, der `SEE` sagte | `SEE_BEFORE_FIRST_ROLL` |
| keine Antwort auf `YOUR TURN` in 250 ms | der Spieler am Zug | `DID_NOT_TAKE_TURN` |
| keine Ansage nach `ROLLED` in 250 ms | der Spieler am Zug | `DID_NOT_ANNOUNCE` |

Wird Mäxchen angesagt, deckt der Server **sofort** auf, ohne dass jemand `SEE` sagen kann.

## Protokoll

Nachrichten sind Textzeilen per UDP (UTF-8), Teile mit `;` getrennt. Kommandos stehen in
Großbuchstaben. Ein **Token** ist ein vom Server erzeugter Text; jede Antwort muss das Token der
Nachricht enthalten, auf die sie antwortet. Antworten mit falschem oder altem Token ignoriert der
Server. Du hast für jede Antwort **250 ms** Zeit.

### Technisches (übernimmt das Bot-Gerüst)

Registrierung (`REGISTER`), Teilnahme an Runden (`JOIN`), Heartbeats (`HEARTBEAT` alle 2 s) und
Abmeldung (`UNREGISTER`) erledigt das Bot-Gerüst des mitgelieferten Bots. Du musst nur auf
Spielnachrichten reagieren.

### Nachrichten, die dein Bot empfängt

| Nachricht | Bedeutung |
|---|---|
| `ROUND STARTED;<Runde>;<Spieler>` | Die Runde beginnt. `<Spieler>` sind die Namen in Zugreihenfolge, durch `,` getrennt (`anna,bert,cleo`). |
| `YOUR TURN;<token>` | Du bist dran. Antworte mit `ROLL` oder `SEE`. |
| `ROLLED;<würfel>;<token>` | Dein Wurf, z. B. `ROLLED;4,3;a1b2`. Antworte mit `ANNOUNCE`. Das Token ist neu. |
| `PLAYER ROLLS;<name>` | Ein Spieler würfelt (alle bekommen es, auch du). |
| `ANNOUNCED;<name>;<würfel>` | Die Ansage eines Spielers, z. B. `ANNOUNCED;bert;6,5`. |
| `PLAYER WANTS TO SEE;<name>` | Ein Spieler zweifelt die letzte Ansage an. |
| `ACTUAL DICE;<würfel>` | Der echte Wurf, wenn aufgedeckt wird (`SEE` oder Mäxchen). |
| `PLAYER LOST;<namen>;<grund>` | Wer verloren hat (bei `MIA` mehrere, durch `,` getrennt), und warum. |
| `SCORE;<name:punkte>,...` | Punktestand nach jeder Runde, z. B. `SCORE;anna:3,bert:1`. |
| `ROUND CANCELED;<grund>` | Runde abgebrochen (`NO_PLAYERS` oder `ONLY_ONE_PLAYER`). |

Die Nachrichten `ROUND STARTING;<token>` (Einladung) und `HEARTBEAT` übernimmt das Bot-Gerüst.

### Nachrichten, die dein Bot sendet

| Nachricht | Wann |
|---|---|
| `ROLL;<token>` | Auf `YOUR TURN`: Würfeln. |
| `SEE;<token>` | Auf `YOUR TURN`: Die letzte Ansage anzweifeln. |
| `ANNOUNCE;<würfel>;<token>` | Auf `ROLLED`: Einen Wurf ansagen, z. B. `ANNOUNCE;6,5;a1b2`. |

Würfel haben die Form `<würfel>,<würfel>` mit Augen von 1 bis 6, zum Beispiel `4,2`. In deinen
Ansagen darfst du die Reihenfolge frei wählen: `2,4` liest der Server als `4,2`. Ungültige Würfel
(`7,1`, `0,3`, `42`, `a,b`) kann er nicht lesen und ignoriert die Nachricht stillschweigend. Das
kann leicht dazu führen, dass die 250 ms Antwortzeit ablaufen, in der der Server ein gültiges
Kommando von deinem Bot braucht.

## Ablauf eines Zugs

```
YOUR TURN;t1
   ├─ ROLL;t1
   │     Server: PLAYER ROLLS;<dein name>  → alle
   │     Server: ROLLED;<würfel>;t2  → nur du
   │     ANNOUNCE;<würfel>;t2
   │     Server: ANNOUNCED;<dein name>;<würfel>  → alle
   │        ├─ Ansage zu niedrig        → PLAYER LOST;<dein name>;ANNOUNCED_LOSING_DICE
   │        ├─ Ansage ist Mäxchen       → ACTUAL DICE, dann PLAYER LOST (LIED_ABOUT_MIA oder MIA)
   │        └─ sonst                    → nächster Spieler bekommt YOUR TURN
   └─ SEE;t1
         Server: PLAYER WANTS TO SEE;<dein name>  → alle
         Server: ACTUAL DICE;<echter Wurf der letzten Ansage>  → alle
         Server: PLAYER LOST;<wer>;CAUGHT_BLUFFING oder SEE_FAILED

nach jeder Runde: SCORE;...
```

Nach `PLAYER LOST` und `SCORE` lädt der Server sofort zur nächsten Runde ein.

## Randfälle

- **Zu langsam:** Antwortest du nicht innerhalb von 250 ms, verlierst du die Runde. Vermeide
  aufwendige Berechnungen, bevor du antwortest.
- **Altes Token:** Nach `ROLLED` ist das Token von `YOUR TURN` ungültig. In `ANNOUNCE` musst du
  das Token von `ROLLED` verwenden.
- **Nicht dran:** Kommandos, die du sendest, während ein anderer Spieler an der Reihe ist,
  ignoriert der Server.
- **Ungültiges Format:** Der Server ignoriert Nachrichten, die er nicht lesen kann, ohne
  Fehlermeldung. Für ihn sieht es dann so aus, als hättest du gar nicht geantwortet: Nach 250 ms
  verlierst du die Runde mit `DID_NOT_TAKE_TURN` (statt `ROLL` oder `SEE`) bzw.
  `DID_NOT_ANNOUNCE` (statt `ANNOUNCE`).
- **Erste Ansage frei:** Wer als Erster dran ist, kann jeden Wurf ansagen. `SEE` ist dann
  verboten (`SEE_BEFORE_FIRST_ROLL`).
- **Mäxchen ansagen:** Sagt jemand `2,1` an, deckt der Server sofort auf. Hat der Ansagende
  Mäxchen, verlieren alle anderen, sonst verliert er selbst. Das gilt auch für dich.
- **Mitspieler verschwinden:** Meldet sich ein Mitspieler mitten in der Runde ab, bleibt er in
  der Zugreihenfolge. Kommt er an die Reihe, verliert er nach 250 ms.
- **Nur ein Spieler:** Die Runde endet sofort mit `ROUND CANCELED;ONLY_ONE_PLAYER`. Danach
  lädt der Server zur nächsten Runde ein.

## Aufgabe

**Kannst du einen Bot schreiben, der mehr Punkte erreicht als der `simple`-Bot?**

So läuft die Challenge ab:

1. Du schreibst deinen Bot. Wie viel Zeit du dafür hast, sagt dir der Veranstalter. Du kannst
   ihn jederzeit gegen den Server starten und gegen die Bots der anderen spielen lassen.
2. Danach stoppt der Veranstalter den Server und startet ihn neu. Alle Punktestände stehen dann
   wieder auf 0. Starte deinen Bot nach dem Neustart neu.
3. Alle Bots spielen für eine Weile gegeneinander. Als Maßstab spielt ein `simple`-Bot mit, der
   immer würfelt und immer die Wahrheit sagt. Die Punkte siehst du live in der Zuschauer-Ansicht.
4. Die Endstände zeigen, wer besser abgeschnitten hat als der `simple`-Bot.

Woran du drehen kannst:

- Ob dein Bot würfelt oder die letzte Ansage anzweifelt. Dafür musst du dir merken, was der
  Vorgänger angesagt hat (`ANNOUNCED`).
- Was dein Bot ansagt. Er muss nicht die Wahrheit sagen, die Ansage muss aber höher sein als die
  letzte.

Dein Bot braucht dafür einen Zustand, der über mehrere Nachrichten gilt: die letzte Ansage und
dein eigener Wurf. Setze ihn bei `ROUND STARTED` zurück. Hilfreich ist außerdem eine kleine
Funktion, die zwei Würfe nach der Rangfolge vergleicht.

Zum Ausprobieren: Starte zusätzlich einen Bot vom Typ `spectator` (`-t spectator`). Er schreibt
alle Nachrichten ins Log und zeigt dir, was zwischen dem Server und den Bots passiert.

## Hinweise für die KI

Wenn dir diese Datei gegeben wurde, um einen Bot zu bauen:

- Arbeite im vorhandenen Bot-Gerüst. **Ändere nicht** dessen technischen Teil (Verbindung,
  Registrierung, Heartbeats, Abmeldung). Schreibe eine neue Bot-Klasse mit der Methode
  `on_message` (Java: `onMessage`) und trage sie als neuen Bot-Typ ein.
- `on_message` bekommt jede Nachricht außer Heartbeats als Kommando und Parameterliste.
  `YOUR TURN;4711` kommt als `"YOUR TURN"` mit `["4711"]`.
- Antworten schickst du mit `server.send(kommando, parameter...)`. Das Bot-Gerüst setzt die
  Teile mit `;` zusammen. Beispiel: `server.send("ANNOUNCE", "6,5", token)`.
- Das Token der Antwort ist der **letzte** Parameter der Nachricht, auf die du antwortest.
- Halte den Bot-Zustand in Feldern der Bot-Klasse.
- Nach Änderungen: Bot starten und gegen den Server spielen lassen. Das Log zeigt jede gesendete
  Nachricht als `----> BEFEHL ['parameter']`.

### Bot-Gerüst je Sprache

| Sprache | Beispiel-Bot | Eintragen in | Starten |
|---|---|---|---|
| Python | `python/src/maexchen_client/simple_bot.py` | `BOT_TYPES` in `main.py` | `cd python && uv run maexchen-bot -i <ip> -p <port> -n <name> -t <typ>` |
| Java | `java/src/main/java/maexchen/SimpleBot.java` | `Map` in `Main.java` | `cd java && ./gradlew run --args="-i <ip> -p <port> -n <name> -t <typ>"` |
| Rust | `rust/src/simple_bot.rs` | `BotType` in `main.rs` | `cd rust && cargo run -- -i <ip> -p <port> -n <name> -t <typ>` |

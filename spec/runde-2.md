# Spec Runde 2: Mäxchen-Bot mit Blindwurf

Diese Datei beschreibt alles, was du (oder deine KI) für Runde 2 braucht. Sie steht für sich:
Du musst keine andere Datei lesen, um einen spielfähigen Bot zu bauen. Sie enthält die Regeln und
das Protokoll aus Runde 1 und zusätzlich den **Blindwurf**. Neu ist alles, was in den Abschnitten
mit „Neu in Runde 2“ steht.

## Ziel

Schreibe einen Bot, der über UDP am Mäxchen-Spiel teilnimmt und möglichst selten verliert. Dein
Bot kann jetzt auch würfeln, ohne seinen Wurf zu sehen, und er trifft seine Ansagen dann ohne
Kenntnis der Würfel.

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

### Neu in Runde 2: der Blindwurf

Du kannst würfeln, ohne die Würfel zu sehen (`BLIND_ROLL`). Der Server würfelt dann für dich, sagt
dir aber nicht, was herauskam. Du musst trotzdem etwas ansagen. Das kann stimmen oder auch nicht.

- Jeder Spieler darf in seinem Zug **höchstens zweimal würfeln**.
- Der **zweite Wurf ist immer ein Blindwurf**. Der erste Wurf kann ein normaler Wurf (`ROLL`) oder
  ebenfalls ein Blindwurf sein.
- Der zweite Wurf **ersetzt den ersten**. Es zählt der Wurf, den der Server zuletzt für dich
  gewürfelt hat. Aufgedeckt wird später immer dieser letzte Wurf.
- Ein dritter Wurf ist nicht erlaubt.
- Du kannst jederzeit nach einem Wurf ansagen (`ANNOUNCE`), auch ohne ein zweites Mal zu würfeln.

Alle Mitspieler erfahren, dass du blind würfelst (`PLAYER BLIND ROLLS`).

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
| keine Ansage nach einem Wurf in 250 ms | der Spieler am Zug | `DID_NOT_ANNOUNCE` |

Wird Mäxchen angesagt, deckt der Server **sofort** auf, ohne dass jemand `SEE` sagen kann.
Das gilt auch, wenn du blind gewürfelt hast: Dann zählt der Wurf, den du nicht gesehen hast.

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
| `YOUR TURN;<token>` | Du bist dran. Antworte mit `ROLL`, `BLIND_ROLL` oder `SEE`. |
| `ROLLED;<würfel>;<token>` | Dein Wurf, z. B. `ROLLED;4,3;a1b2`. Antworte mit `ANNOUNCE` oder, einmal, mit `BLIND_ROLL`. Das Token ist neu. |
| **`BLIND_ROLLED;<token>`** | **Neu.** Dein Blindwurf ist gewürfelt, die Würfel siehst du nicht. Antworte mit `ANNOUNCE` oder, wenn es dein erster Wurf war, mit `BLIND_ROLL`. Das Token ist neu. |
| `PLAYER ROLLS;<name>` | Ein Spieler würfelt (alle bekommen es, auch du). |
| **`PLAYER BLIND ROLLS;<name>`** | **Neu.** Ein Spieler würfelt blind (alle bekommen es, auch du). |
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
| **`BLIND_ROLL;<token>`** | **Neu.** Auf `YOUR TURN`: Blind würfeln. Auf `ROLLED` oder `BLIND_ROLLED`: Noch einmal blind würfeln (siehe Regeln oben). |
| `SEE;<token>` | Auf `YOUR TURN`: Die letzte Ansage anzweifeln. |
| `ANNOUNCE;<würfel>;<token>` | Auf `ROLLED` oder `BLIND_ROLLED`: Einen Wurf ansagen, z. B. `ANNOUNCE;6,5;a1b2`. |

Würfel haben die Form `<würfel>,<würfel>` mit Augen von 1 bis 6, zum Beispiel `4,2`. In deinen
Ansagen darfst du die Reihenfolge frei wählen: `2,4` liest der Server als `4,2`. Ungültige Würfel
(`7,1`, `0,3`, `42`, `a,b`) kann er nicht lesen und ignoriert die Nachricht stillschweigend. Das
kann leicht dazu führen, dass die 250 ms Antwortzeit ablaufen, in der der Server ein gültiges
Kommando von deinem Bot braucht.

### Tokens beim Blindwurf

Jede Nachricht, auf die du antworten sollst, bringt ein neues Token mit. Nach einem zweiten Wurf
ist das Token des ersten Wurfs ungültig:

```
ROLLED;4,3;t2         ← erster Wurf, du siehst ihn
BLIND_ROLL;t2         → zweiter Wurf, ersetzt den ersten
BLIND_ROLLED;t3       ← neues Token
ANNOUNCE;6,5;t3       → nur t3 gilt, t2 wird ignoriert
```

## Ablauf eines Zugs

```
YOUR TURN;t1
   ├─ ROLL;t1
   │     Server: PLAYER ROLLS;<dein name>  → alle
   │     Server: ROLLED;<würfel>;t2  → nur du
   │        ├─ ANNOUNCE;<würfel>;t2
   │        └─ BLIND_ROLL;t2   (zweiter, blinder Wurf)
   │              Server: PLAYER BLIND ROLLS;<dein name>  → alle
   │              Server: BLIND_ROLLED;t3  → nur du
   │              ANNOUNCE;<würfel>;t3    (kein dritter Wurf)
   ├─ BLIND_ROLL;t1
   │     Server: PLAYER BLIND ROLLS;<dein name>  → alle
   │     Server: BLIND_ROLLED;t2  → nur du
   │        ├─ ANNOUNCE;<würfel>;t2
   │        └─ BLIND_ROLL;t2   (zweiter, blinder Wurf)
   │              Server: PLAYER BLIND ROLLS;<dein name>  → alle
   │              Server: BLIND_ROLLED;t3  → nur du
   │              ANNOUNCE;<würfel>;t3    (kein dritter Wurf)
   └─ SEE;t1
         Server: PLAYER WANTS TO SEE;<dein name>  → alle
         Server: ACTUAL DICE;<echter Wurf der letzten Ansage>  → alle
         Server: PLAYER LOST;<wer>;CAUGHT_BLUFFING oder SEE_FAILED

nach jeder Ansage (ANNOUNCE):
   Server: ANNOUNCED;<dein name>;<würfel>  → alle
      ├─ Ansage zu niedrig        → PLAYER LOST;<dein name>;ANNOUNCED_LOSING_DICE
      ├─ Ansage ist Mäxchen       → ACTUAL DICE, dann PLAYER LOST (LIED_ABOUT_MIA oder MIA)
      └─ sonst                    → nächster Spieler bekommt YOUR TURN

nach jeder Runde: SCORE;...
```

Nach `PLAYER LOST` und `SCORE` lädt der Server sofort zur nächsten Runde ein.

## Randfälle

- **Zu langsam:** Antwortest du nicht innerhalb von 250 ms, verlierst du die Runde. Vermeide
  aufwendige Berechnungen, bevor du antwortest.
- **Altes Token:** Nach jedem Wurf ist das Token der vorherigen Nachricht ungültig. In `ANNOUNCE`
  und in einem zweiten `BLIND_ROLL` musst du das Token des letzten Wurfs (`ROLLED` oder
  `BLIND_ROLLED`) verwenden.
- **Nicht dran:** Kommandos, die du sendest, während ein anderer Spieler an der Reihe ist,
  ignoriert der Server.
- **Dritter Wurf:** Ein `BLIND_ROLL` nach dem zweiten Wurf ignoriert der Server. Es läuft die Zeit
  für die Ansage weiter (`DID_NOT_ANNOUNCE`).
- **Zweiter Wurf mit `ROLL`:** Ein `ROLL` nach einem Wurf ignoriert der Server ebenfalls. Der
  zweite Wurf muss ein `BLIND_ROLL` sein.
- **`SEE` nach einem Wurf:** Anzweifeln kannst du nur am Anfang deines Zugs, nicht nach dem
  Würfeln. Ein `SEE` danach ignoriert der Server.
- **Ungültiges Format:** Der Server ignoriert Nachrichten, die er nicht lesen kann, ohne
  Fehlermeldung. Für ihn sieht es dann so aus, als hättest du gar nicht geantwortet: Nach 250 ms
  verlierst du die Runde mit `DID_NOT_TAKE_TURN` (statt `ROLL`, `BLIND_ROLL` oder `SEE`) bzw.
  `DID_NOT_ANNOUNCE` (statt `ANNOUNCE`).
- **Erste Ansage frei:** Wer als Erster dran ist, kann jeden Wurf ansagen. `SEE` ist dann
  verboten (`SEE_BEFORE_FIRST_ROLL`).
- **Mäxchen ansagen:** Sagt jemand `2,1` an, deckt der Server sofort auf. Hat der Ansagende
  Mäxchen, verlieren alle anderen, sonst verliert er selbst. Das gilt auch für dich, auch wenn
  du blind gewürfelt hast.
- **Mitspieler verschwinden:** Meldet sich ein Mitspieler mitten in der Runde ab, bleibt er in
  der Zugreihenfolge. Kommt er an die Reihe, verliert er nach 250 ms.
- **Nur ein Spieler:** Die Runde endet sofort mit `ROUND CANCELED;ONLY_ONE_PLAYER`. Danach
  lädt der Server zur nächsten Runde ein.

## Aufgabe

**Kannst du einen Bot schreiben, der mehr Punkte erreicht als der `simple`-Bot, obwohl jetzt auch
blind gewürfelt wird?**

Der Ablauf ist derselbe wie in Runde 1:

1. Du schreibst deinen Bot. Wie viel Zeit du dafür hast, sagt dir der Veranstalter. Du kannst
   ihn jederzeit gegen den Server starten und gegen die Bots der anderen spielen lassen.
2. Danach stoppt der Veranstalter den Server und startet ihn neu. Alle Punktestände stehen dann
   wieder auf 0. Starte deinen Bot nach dem Neustart neu.
3. Alle Bots spielen für eine Weile gegeneinander. Als Maßstab spielt ein `simple`-Bot mit, der
   immer würfelt und immer die Wahrheit sagt. Die Punkte siehst du live in der Zuschauer-Ansicht.
4. Die Endstände zeigen, wer besser abgeschnitten hat als der `simple`-Bot.

Was sich durch den Blindwurf ändert:

- Wer blind würfelt und danach ansagt, kennt seinen eigenen Wurf nicht. Du erfährst es über
  `PLAYER BLIND ROLLS`, bevor die Ansage kommt (`ANNOUNCED`). Kannst du dieser Ansage weiter
  trauen, oder willst du sie lieber aufdecken (`SEE`)?
- Du selbst darfst blind würfeln, musst es aber nicht. Wenn du es tust, bekommst du statt der
  Würfel nur `BLIND_ROLLED;<token>` und musst trotzdem etwas ansagen. Antworte dann mit dem
  Token aus dieser Nachricht.

Dein Bot braucht einen Zustand, der über mehrere Nachrichten gilt: die letzte Ansage, ob der
Ansagende blind gewürfelt hat, und dein eigener Wurf. Setze ihn bei `ROUND STARTED` zurück.
Hilfreich ist außerdem eine kleine Funktion, die zwei Würfe nach der Rangfolge vergleicht.

Zum Ausprobieren: Starte zusätzlich einen Bot vom Typ `spectator` (`-t spectator`). Er schreibt
alle Nachrichten ins Log und zeigt dir, was zwischen dem Server und den Bots passiert.

## Hinweise für die KI

Wenn dir diese Datei gegeben wurde, um einen Bot zu bauen:

- Arbeite im vorhandenen Bot-Gerüst. **Ändere nicht** dessen technischen Teil (Verbindung,
  Registrierung, Heartbeats, Abmeldung). Schreibe eine neue Bot-Klasse mit der Methode
  `on_message` (Java: `onMessage`) und trage sie als neuen Bot-Typ ein.
- `on_message` bekommt jede Nachricht außer Heartbeats als Kommando und Parameterliste.
  `YOUR TURN;4711` kommt als `"YOUR TURN"` mit `["4711"]`, `BLIND_ROLLED;4711` als
  `"BLIND_ROLLED"` mit `["4711"]` und `ROLLED;4,3;4711` als `"ROLLED"` mit `["4,3", "4711"]`.
- Antworten schickst du mit `server.send(kommando, parameter...)`. Das Bot-Gerüst setzt die
  Teile mit `;` zusammen. Beispiel: `server.send("ANNOUNCE", "6,5", token)` und
  `server.send("BLIND_ROLL", token)`.
- Das Token der Antwort ist der **letzte** Parameter der Nachricht, auf die du antwortest.
- Die Beschreibung der Nachrichten im Kommentar der Bot-Schnittstelle kennt `BLIND_ROLLED` und
  `PLAYER BLIND ROLLS` noch nicht. Verlasse dich auf diese Spec.
- Halte den Bot-Zustand in Feldern der Bot-Klasse.
- Nach Änderungen: Bot starten und gegen den Server spielen lassen. Das Log zeigt jede gesendete
  Nachricht als `----> BEFEHL ['parameter']`.

### Bot-Gerüst je Sprache

| Sprache | Beispiel-Bot | Eintragen in | Starten |
|---|---|---|---|
| Python | `python/src/maexchen_client/simple_bot.py` | `BOT_TYPES` in `main.py` | `cd python && uv run maexchen-bot -i <ip> -p <port> -n <name> -t <typ>` |
| Java | `java/src/main/java/maexchen/SimpleBot.java` | `Map` in `Main.java` | `cd java && ./gradlew run --args="-i <ip> -p <port> -n <name> -t <typ>"` |
| Rust | `rust/src/simple_bot.rs` | `BotType` in `main.rs` | `cd rust && cargo run -- -i <ip> -p <port> -n <name> -t <typ>` |

# Spec Runde 4: Mäxchen-Bot mit wechselnden Regeln

Diese Datei beschreibt alles, was du (oder deine KI) für Runde 4 braucht. Sie steht für sich:
Du musst keine andere Datei lesen, um einen spielfähigen Bot zu bauen. Sie enthält beide
Rangfolgen der Würfe und den Blindwurf. Neu ist, dass der Server **vor jeder Runde ansagt, welche
Regeln in dieser Runde gelten**. Neu ist alles, was in den Abschnitten mit „Neu in Runde 4“ steht.

## Ziel

Schreibe einen Bot, der über UDP am Mäxchen-Spiel teilnimmt und möglichst selten verliert. Die
Regeln können sich von Runde zu Runde ändern. Dein Bot muss bei jedem Rundenstart lesen, welche
Regeln gelten, und danach spielen.

## Spielregeln

Gespielt wird mit zwei Würfeln. Die Reihenfolge der Augen spielt keine Rolle: `1,2` und `2,1` sind
derselbe Wurf. Der Server schreibt Würfe immer mit dem höheren Würfel zuerst (`2,1`, nie `1,2`).

### Neu in Runde 4: die Regeln einer Runde

Jede Runde hat ihre eigenen Regeln. Sie bestehen aus zwei Teilen:

- **Mäxchen-Regel:** Entweder ist die `2,1` das Mäxchen (`mia21`), oder die `4,2` ist das Mäxchen
  und die `2,1` das halbe Mäxchen (`mia42`).
- **Blindwurf:** Er ist in dieser Runde erlaubt (`,blind`) oder nicht.

Welche Regeln gelten, sagt der Server beim Rundenstart an (siehe Protokoll). Die Regeln gelten für
die ganze Runde. In der nächsten Runde können sie anders sein.

#### Rangfolge bei `mia21`

Von niedrig nach hoch:

1. **Normale Würfe** `3,1` (niedrigster), `3,2`, `4,1`, `4,2`, `4,3`, `5,1`, `5,2`, `5,3`, `5,4`,
   `6,1`, `6,2`, `6,3`, `6,4`, `6,5` (höchster normaler Wurf).
2. **Pasche** `1,1`, `2,2`, `3,3`, `4,4`, `5,5`, `6,6`. Jeder Pasch schlägt jeden normalen Wurf.
3. **Mäxchen** (auch Meier) `2,1`. Der höchste Wurf.

Nur das Mäxchen (`2,1`) wird beim Ansagen sofort aufgedeckt.

#### Rangfolge bei `mia42`

Von niedrig nach hoch:

1. **Normale Würfe** `3,1` (niedrigster), `3,2`, `4,1`, `4,3`, `5,1`, `5,2`, `5,3`, `5,4`, `6,1`,
   `6,2`, `6,3`, `6,4`, `6,5` (höchster normaler Wurf). Die `4,2` und die `2,1` gehören nicht
   dazu.
2. **Pasche** `1,1`, `2,2`, `3,3`, `4,4`, `5,5`, `6,6`. Jeder Pasch schlägt jeden normalen Wurf.
3. **Halbes Mäxchen** `2,1`. Sie schlägt jeden Pasch, aber nicht das Mäxchen.
4. **Mäxchen** (auch Meier) `4,2`. Der höchste Wurf.

Nur das Mäxchen (`4,2`) wird beim Ansagen sofort aufgedeckt. Sagt jemand die `2,1` an, geht das
Spiel ganz normal weiter.

Reihum würfelt jeder Spieler und sagt einen Wurf an. Er muss nicht sagen, was er gewürfelt hat.
Wer an der Reihe ist, kann statt zu würfeln auch die letzte Ansage anzweifeln (`SEE`).

Die Ansage muss **echt höher** sein als die vorherige Ansage. Die erste Ansage einer Runde ist
frei. Eine gleich hohe oder niedrigere Ansage verliert sofort.

Eine Runde endet, sobald jemand verliert. **Jeder Spieler der Runde, der nicht verliert, bekommt
einen Punkt.**

### Der Blindwurf

In Runden mit `,blind` kannst du würfeln, ohne die Würfel zu sehen (`BLIND_ROLL`). Der Server
würfelt dann für dich, sagt dir aber nicht, was herauskam. Du musst trotzdem etwas ansagen. Das
kann stimmen oder auch nicht. In Runden ohne `,blind` ignoriert der Server jedes `BLIND_ROLL`.

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
| **`ROUND STARTED;<Runde>;<Regeln>;<Spieler>`** | **Neu:** Die Runde beginnt, mit den Regeln dieser Runde als zweitem Parameter. `<Regeln>` ist `mia21`, `mia42`, `mia21,blind` oder `mia42,blind`. `<Spieler>` sind die Namen in Zugreihenfolge, durch `,` getrennt (`anna,bert,cleo`). Beispiel: `ROUND STARTED;7;mia42,blind;anna,bert,cleo`. |
| `YOUR TURN;<token>` | Du bist dran. Antworte mit `ROLL`, `SEE` oder, in Runden mit `,blind`, `BLIND_ROLL`. |
| `ROLLED;<würfel>;<token>` | Dein Wurf, z. B. `ROLLED;4,3;a1b2`. Antworte mit `ANNOUNCE` oder, einmal, mit `BLIND_ROLL`. Das Token ist neu. |
| `BLIND_ROLLED;<token>` | Dein Blindwurf ist gewürfelt, die Würfel siehst du nicht. Antworte mit `ANNOUNCE` oder, wenn es dein erster Wurf war, mit `BLIND_ROLL`. Das Token ist neu. |
| `PLAYER ROLLS;<name>` | Ein Spieler würfelt (alle bekommen es, auch du). |
| `PLAYER BLIND ROLLS;<name>` | Ein Spieler würfelt blind (alle bekommen es, auch du). |
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
| `BLIND_ROLL;<token>` | Nur in Runden mit `,blind`. Auf `YOUR TURN`: Blind würfeln. Auf `ROLLED` oder `BLIND_ROLLED`: Noch einmal blind würfeln (siehe Regeln oben). |
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
- **Blindwurf ohne `,blind`:** In Runden, deren Regeln kein `,blind` enthalten, ignoriert der
  Server jedes `BLIND_ROLL`. Es läuft die Zeit weiter (`DID_NOT_TAKE_TURN` bzw.
  `DID_NOT_ANNOUNCE`).
- **Mäxchen ansagen:** Sagt jemand das Mäxchen der Runde an (`2,1` bei `mia21`, `4,2` bei
  `mia42`), deckt der Server sofort auf. Hat der Ansagende Mäxchen gewürfelt, verlieren alle
  anderen, sonst verliert er selbst. Das gilt auch für dich, auch wenn du blind gewürfelt hast.
- **Halbes Mäxchen ansagen:** Nur bei `mia42`. Sagt jemand `2,1` an, wird nicht aufgedeckt. Die
  Ansage steht wie jede andere und kann mit `SEE` angezweifelt werden. Der nächste Spieler muss
  das Mäxchen (`4,2`) ansagen, um sie zu überbieten.
- **Regeln wechseln:** Die Regeln gelten nur für die Runde, die `ROUND STARTED` ankündigt. Merke
  dir pro Runde, welche gelten, und ersetze sie beim nächsten `ROUND STARTED`.
- **Mitspieler verschwinden:** Meldet sich ein Mitspieler mitten in der Runde ab, bleibt er in
  der Zugreihenfolge. Kommt er an die Reihe, verliert er nach 250 ms.
- **Nur ein Spieler:** Die Runde endet sofort mit `ROUND CANCELED;ONLY_ONE_PLAYER`. Danach
  lädt der Server zur nächsten Runde ein.

## Aufgabe

**Kannst du einen Bot schreiben, der sich jede Runde auf die angesagten Regeln einstellt und
dabei mehr Punkte erreicht als der `simple`-Bot?**

Der Ablauf ist derselbe wie in Runde 3:

1. Du schreibst deinen Bot. Wie viel Zeit du dafür hast, sagt dir der Veranstalter. Du kannst
   ihn jederzeit gegen den Server starten und gegen die Bots der anderen spielen lassen.
2. Danach stoppt der Veranstalter den Server und startet ihn neu. Alle Punktestände stehen dann
   wieder auf 0. Starte deinen Bot nach dem Neustart neu.
3. Alle Bots spielen für eine Weile gegeneinander. Als Maßstab spielt ein `simple`-Bot mit, der
   immer würfelt und immer die Wahrheit sagt. Die Punkte siehst du live in der Zuschauer-Ansicht.
4. Die Endstände zeigen, wer besser abgeschnitten hat als der `simple`-Bot.

Was sich durch die wechselnden Regeln ändert:

- `ROUND STARTED` hat jetzt einen Parameter mehr: Die Regeln stehen zwischen der Rundennummer und
  den Spielernamen. Ein Bot, der die Spielernamen als zweiten Parameter liest, bekommt jetzt die
  Regeln.
- Dein Bot muss die Regeln der Runde lesen und danach spielen: Welcher Wurf ist das Mäxchen, wie
  ist die Rangfolge, und ist der Blindwurf erlaubt?
- Deine Vergleichsfunktion für Würfe braucht die Regeln als zusätzliche Angabe. Der Gegner
  kann in der nächsten Runde andere Regeln haben als in dieser.

Dein Bot braucht einen Zustand, der über mehrere Nachrichten gilt: die Regeln der Runde, die
letzte Ansage, ob der Ansagende blind gewürfelt hat, und dein eigener Wurf. Setze ihn bei
`ROUND STARTED` zurück und lies dabei die Regeln neu ein.

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
- `ROUND STARTED;7;mia42,blind;anna,bert` kommt als `"ROUND STARTED"` mit
  `["7", "mia42,blind", "anna,bert"]`. Die Regeln sind der zweite Parameter, die Spielernamen
  der dritte.
- Die Beschreibung der Nachrichten im Kommentar der Bot-Schnittstelle kennt `BLIND_ROLLED`,
  `PLAYER BLIND ROLLS` und die Regeln in `ROUND STARTED` noch nicht. Verlasse dich auf diese Spec.
- Baue die Rangfolge der Würfe an **einer** Stelle (eine Vergleichsfunktion, die die Regeln der
  Runde bekommt) und nutze sie überall. Bei `mia21` ist die `2,1` das Mäxchen, bei `mia42` die
  `4,2`, und die `2,1` ist das halbe Mäxchen. Die Beispiel-Bots kennen nur die Regeln von `mia21`.
- Halte den Bot-Zustand in Feldern der Bot-Klasse.
- Nach Änderungen: Bot starten und gegen den Server spielen lassen. Das Log zeigt jede gesendete
  Nachricht als `----> BEFEHL ['parameter']`.

### Bot-Gerüst je Sprache

| Sprache | Beispiel-Bot | Eintragen in | Starten |
|---|---|---|---|
| Python | `python/src/maexchen_client/simple_bot.py` | `BOT_TYPES` in `main.py` | `cd python && uv run maexchen-bot -i <ip> -p <port> -n <name> -t <typ>` |
| Java | `java/src/main/java/maexchen/SimpleBot.java` | `Map` in `Main.java` | `cd java && ./gradlew run --args="-i <ip> -p <port> -n <name> -t <typ>"` |
| Rust | `rust/src/simple_bot.rs` | `BotType` in `main.rs` | `cd rust && cargo run -- -i <ip> -p <port> -n <name> -t <typ>` |

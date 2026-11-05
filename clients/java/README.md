# Mäxchen-Client in Java

Ausgangsbasis für eigene Bots. Der Client kümmert sich um alles Technische (Verbindung,
Registrierung, Heartbeats, Abmeldung bei Ctrl-C). Du schreibst nur, was dein Bot auf
Nachrichten des Servers antwortet. Das Protokoll steht in `protokoll.md` im
Verzeichnis darüber.

## Starten

Du brauchst ein JDK 21. Gradle musst du nicht installieren, der Wrapper `gradlew` lädt es beim
ersten Aufruf (dafür braucht es Internet). Im Verzeichnis `java/`:

```sh
./gradlew run --args="-i <server-ip> -p <port> -n <dein-name> -t simple"
```

Server und Port nennt dir der Veranstalter. Ctrl-C beendet den Bot und meldet ihn beim
Server ab. Unter Windows nimmst du `gradlew.bat` statt `./gradlew`.

Für einen schnellen Start:

1. Starte den `simple`-Bot und schau ins Log. Wenn `REGISTERED` erscheint, bist du dabei.
2. Starte in einem zweiten Terminal einen weiteren Bot mit anderem Namen, zum Beispiel mit
   `-t random`. Ein Spiel braucht mindestens zwei Spieler.
3. Mit `-t spectator` schaust du nur zu und siehst alle Nachrichten im Log.

## Optionen

Die Optionen stehen alle in `--args="..."`, durch Leerzeichen getrennt.

| Option | Bedeutung |
|---|---|
| `-i`, `--ip` | Adresse des Servers (Standard `127.0.0.1`) |
| `-p`, `--port` | Port des Servers (Standard `9000`) |
| `-n`, `--name` | Spielername; ohne Angabe wird einer mit Zufallszahl erzeugt. Er darf höchstens 20 Zeichen lang sein und keine Leerzeichen und keines der Zeichen `,` `;` `:` enthalten. Ist der Name schon vergeben, lehnt der Server dich ab |
| `-t`, `--type` | `simple`, `random` oder `spectator` (Standard `simple`) |
| `--no-auto-join` | Nicht automatisch an Runden teilnehmen: `ROUND STARTING` kommt dann bei deinem Bot an, und er muss selbst `JOIN` senden |
| `--no-watch-heartbeat` | Nicht aufgeben, wenn der Server keine Heartbeats mehr sendet |

Bei einer unbekannten Option bricht der Client mit einer Hilfezeile ab.

## Das Log lesen

```
2026-11-05 21:15:03,102 | INFO     | receive              | <---- YOUR TURN ['4711']
2026-11-05 21:15:03,103 | INFO     | send                 | ----> ROLL ['4711']
```

`<----` ist eine Nachricht des Servers, `---->` eine Nachricht deines Bots. In den eckigen
Klammern stehen die Parameter. Wenn dein Bot verliert oder nichts passiert, siehst du hier
zuerst, was er gesendet hat und was der Server dazu sagt.

## Eigenen Bot schreiben

1. Kopiere `src/main/java/maexchen/SimpleBot.java` zu einer neuen Datei, zum Beispiel
   `src/main/java/maexchen/MeinBot.java`, und nenne die Klasse `MeinBot`. Sie implementiert
   das Interface `Bot`.
2. Trage die Klasse in `src/main/java/maexchen/Main.java` in `BOT_TYPES` ein, zum Beispiel
   als `"mein", MeinBot::new`.
3. Starte mit `-t mein`.

`onMessage` bekommt jede Nachricht des Servers (außer Heartbeats) in zwei Teilen:
`YOUR TURN;4711` kommt als `"YOUR TURN"` mit der Parameterliste `["4711"]` an. Antworten
schickst du mit `server.send("ROLL", token)`. Weil der Bot ein Objekt ist, darf er Zustand
in Feldern halten und über mehrere Nachrichten hinweg merken.

Die beiden Beispiel-Bots zeigen den Rahmen:

- `SimpleBot.java` würfelt und sagt an, was gewürfelt wurde.
- `RandomBot.java` entscheidet zufällig zwischen `ROLL` und `SEE`.

## Ideen zum Weiterbauen

- **Zustand merken:** Welche Nachrichten sagen dir, wie die Runde steht? Denk an
  `ROUND STARTED`, `ANNOUNCED`, `PLAYER LOST` und `SCORE`.
- **Entscheiden statt würfeln:** Wann lohnt sich `SEE` statt `ROLL`? Was verrät dir die
  letzte Ansage über die Chancen?
- **Ansage und Wurf trennen:** Du musst nicht ansagen, was du gewürfelt hast. Was folgt
  daraus für deine Gegner, und was für dich?
- **Würfe vergleichen:** Die Reihenfolge der Würfe ist im Protokoll beschrieben. Eine
  kleine Hilfsmethode, die zwei Würfe vergleicht, macht jede Strategie lesbarer.
- **Gegner beobachten:** Als `spectator` oder über die Nachrichten siehst du, wie andere
  spielen.

## Entwickeln

```sh
./gradlew spotlessApply
./gradlew build
```

`spotlessApply` formatiert den Code, `build` übersetzt ihn mit Error Prone und führt die
Tests aus. Beide Aufrufe nacheinander, nicht in einem Befehl.

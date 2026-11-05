# Mäxchen

Eine Umgebung für Mäxchen-Runden (englisch [Mia](https://en.wikipedia.org/wiki/Mia_(game))), in
der die Spielenden Bots sind, die Teilnehmende in einer Programmier-Session schreiben. Der Server
spielt das Spiel, die Bots sprechen über ein einfaches textbasiertes Protokoll per UDP mit ihm.

## Aufbau

| Verzeichnis | Inhalt |
|---|---|
| `server/` | Spielserver in Rust (mit Fuzzing in `server/fuzz` und Benchmarks) |
| `spectator/` | Zuschauer-GUI in Rust (egui) mit Punkteverlauf und Rundenanzeige |
| `clients/` | Material für Teilnehmende: Protokoll, Spec der ersten Runde, Bots in Python, Java und Rust |
| `spec/` | Specs und Protokollfassungen der Runden 2 bis 4 (nicht im Zip), siehe unten |

## Veranstalten

Voraussetzungen: Rust ab 1.88 und [just](https://just.systems/).

```sh
just server -p 9000                              # Server mit Grundregeln
just server --rules mia42 --with-blind-roll      # Server mit anderen Regeln
just spectator -p 9000                           # Zuschauer-GUI
just kit                     # maexchen.zip für die Teilnehmenden aus clients/
just check                   # Format, Lints und Tests aller Teile
```

Die Build-Artefakte landen standardmäßig in `target/`. Über die Umgebungsvariable
`CARGO_TARGET_DIR` legst du ein anderes Verzeichnis fest, wichtig vor allem in Cloud-Ordnern.

`just kit` packt den eingecheckten Stand von `clients/`. Vorher also committen.

### Runden einer Session

Jede Runde hat eine Spec für die Teilnehmenden und eine passende Fassung des Protokolls. Die
Runde 1 liegt schon im Zip (`clients/spec/runde-1.md`, `clients/protokoll.*`). Die Dateien für
die späteren Runden liegen in `spec/` und sind jeweils vollständig, also nicht nur ein Delta zur
Vorrunde.

| Runde | Variante | Spec | Protokoll (de / en) |
|---|---|---|---|
| 1 | `mia21` (Standard) | `clients/spec/runde-1.md` | `clients/protokoll.md`, `clients/protokoll.en.md` |
| 2 | `blind` | `spec/runde-2.md` | `spec/protokoll-runde-2.md`, `.en.md` |
| 3 | `mia42,blind` | `spec/runde-3.md` | `spec/protokoll-runde-3.md`, `.en.md` |
| 4 | `random,blind` | `spec/runde-4.md` | `spec/protokoll-runde-4.md`, `.en.md` |

Zwei Kurzbefehle starten Server und Zuschauer-GUI zusammen im Release-Modus, `just warmup` zum
Üben und `just finale` zum Werten. Die Variante steht dabei in der Schreibweise des Protokolls:
`mia21`, `blind` (Kurzform für `mia21,blind`), `mia42`, `mia42,blind`, `random` oder
`random,blind`. Danach folgen der Port (Standard 9000) und beim Finale die Dauer in Minuten
(Standard 5):

```sh
just warmup                        # Runde 1 üben, endet, wenn du die GUI schließt
just warmup blind 9000             # Runde 2 üben
just finale mia42,blind 9000 5     # Runde 3 werten: der Server endet nach 5 Minuten, die GUI bleibt stehen
just finale random,blind           # Runde 4 werten mit Port 9000 und 5 Minuten
```

Die Finale-Befehle starten zusätzlich einen `simple`-Bot (`simple-benchmark`, aus
`clients/python`) als Maßstab. Er beendet sich selbst, wenn der Server keine Heartbeats mehr
schickt. Sein Log liegt unter `/tmp/maexchen-benchmark-<port>.log`. Wer nach der Zeit mehr
Punkte hat als `simple-benchmark`, war besser als der naive Bot.

Der erste Aufruf baut im Release-Modus und dauert mehrere Minuten, bau also vorher einmal vor
(zum Beispiel mit `just warmup`). Für `just finale` braucht der Python-Client `uv`.

Zwischen den Runden stoppst du den Server und startest ihn mit den Optionen der nächsten Runde neu.
Das setzt die Punkte auf 0. Die Protokollfassung einer späteren Runde ersetzt `clients/protokoll.*`
für die Teilnehmenden.

### Serveroptionen

`-p` Port (9000), `-d` Startverzögerung in ms, `-a` Antwortzeit in ms (250),
`--no-start-rounds-early` startet Runden nicht früher, `-v`/`-q` Log-Ausgabe.

### Regeln

Das Regelwerk wählst du beim Start, ohne neu zu bauen:

| Option | Wirkung |
|---|---|
| `--rules mia21` | Standard: 2,1 ist Mäxchen und der höchste Wurf |
| `--rules mia42` | 4,2 ist das neue Mäxchen, 2,1 das halbe Mäxchen (schlägt alles außer dem Mäxchen) |
| `--rules random` | Die Regeln wechseln pro Runde und werden angesagt |
| `--with-blind-roll` | Blindwurf mit `BLIND_ROLL;<token>` erlauben |
| `--with-no-blind-roll` | Blindwurf verbieten (Standard) |

Bei `--rules random` heißt die Nachricht `ROUND STARTED;<Runde>;<Regeln>;<Spieler>`, wobei
`<Regeln>` `mia21`, `mia42` oder jeweils mit `,blind` ist, zum Beispiel `mia42,blind`. Mit
`--with-blind-roll` entscheidet der Zufall pro Runde auch über den Blindwurf. Bei festen Regeln
bleibt es bei `ROUND STARTED;<Runde>;<Spieler>`, damit die Bots der Teilnehmenden unverändert
laufen. Die Zuschauer-GUI kennt feste Regeln nicht von selbst; dafür gibt es in ihren Optionen
den Schalter „Halbes Mäxchen Regeln“.

## Herkunft

Die Idee stammt von Nicolas Botzet und Steven Collins, die ein ähnliches Setup (mit Poker) für
ihre [Craftsmen Coding Contest Session auf der SoCraTes 2011](http://socrates2011.pbworks.com/w/page/44002190/Sessions)
genutzt haben. Die Motivation für diese Mäxchen-Variante liefert das Projekt von Conrad Thukral:
[conradthukral/maexchen](https://github.com/conradthukral/maexchen). Der Java- und der
Python-Client gehen auf [janernsting/maexchen](https://github.com/janernsting/maexchen)
zurück. Der Rust-Server, die Rust-Bots und die Zuschauer-GUI sind Neuentwicklungen.

Mäxchen-Protokoll
=================

Grundlagen
----------
- Kommunikation zeilenweise per UDP (utf8-kodierte Strings)
- Server öffnet einen bekannten Port
- Clients melden sich auf dem Port als Spieler an und werden ab da für jede Runde angeschrieben
- Clients haben für ihre Antwort ein recht enges Zeitfenster (250 ms)
- Ein Client kann sich als Zuschauer (Spectator) registrieren. Ein Zuschauer kann nicht aktiv an den Runden teilnehmen, erhält aber alle Nachrichten, die an alle Clients geschickt werden.
- Nachrichten, die der Server nicht versteht oder die nicht zum Spielstand passen, ignoriert er ohne Fehlermeldung. Für ihn sieht es dann so aus, als hätte der Client nicht geantwortet.
- Ein Token ist ein vom Server erzeugter Text. Eine Antwort muss das Token der Nachricht enthalten, auf die sie antwortet. Antworten mit falschem oder altem Token werden ignoriert.

Anmelden
--------
- client->server: `REGISTER;name`
- client->server: `REGISTER_SPECTATOR;name`

Falls name gültig und neu ist, oder die Registrierung unter demselben Namen zuletzt von derselben IP kam:

  - Server kommuniziert mit dem Client ab jetzt über die Ursprungs-IP und den Ursprungs-Port der Register-Nachricht.
  - server->client: `REGISTERED`
  
Ansonsten:

  - server->client: `REJECTED;grund` (wobei `grund` entweder `INVALID_NAME` oder `NAME_ALREADY_TAKEN` ist)

Bei erfolgreicher Anmeldung werden all Spectators über den aktuellen Score (auch 0 Punkte eines neuen Spielers)
unterrichtet:

  - server->spectators: `SCORE;spielerpunkte*` (siehe unten)


Kriterien für gültige Spielernamen:

- enthalten keinen whitespace
- enthalten keine Doppelpunkte, Semikolons oder Kommas
- sind maximal 20 Zeichen lang

Abmelden
--------
- client->server: `UNREGISTER`
- server->client: `UNREGISTERED`

Herzschlag
----------
Alle 2 Sekunden bekommt jeder client den "Herzschlag" des Servers übermittelt, um beim Ausbleiben reagieren zu können.

- server->client: `HEARTBEAT`


Start einer Spielrunde
-----------------------
- server->clients: `ROUND STARTING;token`
- client->server: `JOIN;token`

Falls mindestens ein Spieler teilnehmen will:

- Die teilnehmenden Spieler werden vom Server in eine zufällige Reihenfolge gebracht
- server->clients: `ROUND STARTED;rundennummer;spielernamen` (wobei `spielernamen` eine kommagetrennte List der Mitspieler ist, in der Reihenfolge, in der diese Runde gespielt wird)

Ansonsten:

- server->clients: `ROUND CANCELED;NO_PLAYERS` (woraufhin eine neue Runde gestartet wird)

(Runden mit nur einem Spieler werden direkt nach dem Start mit `ROUND CANCELED;ONLY_ONE_PLAYER` abgebrochen)

Würfel und Rangfolge
--------------------
Würfel werden als `a,b` geschrieben, mit Augen von 1 bis 6, zum Beispiel `4,2`. Der Server schreibt Würfe immer mit dem höheren Würfel zuerst. In `ANNOUNCE` darf ein Client die Reihenfolge frei wählen: `2,4` liest der Server als `4,2`. Ungültige Würfel (zum Beispiel `7,1`) werden ignoriert.

Von niedrig nach hoch:

1. Normale Würfe: `3,1` (niedrigster), `3,2`, `4,1`, `4,3`, `5,1`, `5,2`, `5,3`, `5,4`, `6,1`, `6,2`, `6,3`, `6,4`, `6,5`
2. Pasche: `1,1`, `2,2`, `3,3`, `4,4`, `5,5`, `6,6`
3. Halbes Mäxchen: `2,1`
4. Mäxchen (Meier): `4,2`, der höchste Wurf

Nur das Mäxchen (`4,2`) wird beim Ansagen sofort aufgedeckt. Die `2,1` wird nicht aufgedeckt.

Eine Ansage muss echt höher sein als die vorherige. Die erste Ansage einer Runde ist frei.

Ablauf einer Spielrunde
-----------------------
Reihum:

- server->client: `YOUR TURN;token`
- client->server: `command;token` (wobei `command` eins der folgenden ist: `ROLL`, `BLIND_ROLL`, `SEE`)

Bei `ROLL`:

- server->clients: `PLAYER ROLLS;name`
- server->client: `ROLLED;dice;token`
- client->server: `ANNOUNCE;dice;token` oder, einmal, `BLIND_ROLL;token`
- server->clients: `ANNOUNCED;name;dice`

Bei `BLIND_ROLL` (als Zug oder nach `ROLL`/`BLIND_ROLL`):

- server->clients: `PLAYER BLIND ROLLS;name`
- server->client: `BLIND_ROLLED;token` (der Client erfährt die Würfel nicht)
- client->server: `ANNOUNCE;dice;token` oder, wenn es der erste Wurf war, `BLIND_ROLL;token`
- server->clients: `ANNOUNCED;name;dice`

Ein Spieler darf in seinem Zug höchstens zweimal würfeln, und der zweite Wurf ist immer ein `BLIND_ROLL`. Der zweite Wurf ersetzt den ersten; aufgedeckt wird später immer der letzte Wurf. Nach dem zweiten Wurf gilt nur noch das Token von `BLIND_ROLLED`. Weitere Würfe, ein `ROLL` nach einem Wurf und ein `SEE` nach einem Wurf werden ignoriert.

Falls Mäxchen angesagt wurde, wird sofort aufgedeckt. Wenn tatsächlich Mäxchen gewürfelt wurde, verlieren alle anderen Spieler, ansonsten der ansagende.

- server -> clients: `PLAYER LOST;names;reason` (wobei names eine kommagetrennte liste ist)

Bei `SEE`:

- Server überprüft, ob zuletzt angesagte Würfel okay sind und bestimmt, wer verloren hat
- server->clients: `PLAYER WANTS TO SEE;name`
- server->clients: `ACTUAL DICE;dice`
- server->clients: `PLAYER LOST;name;reason`

Wann immer ein Spieler nicht rechtzeitig antwortet oder etwas völlig falsch macht:

- server->clients: `PLAYER LOST;name;reason`

Jeder Spieler, der NICHT VERLIERT, bekommt einen Punkt.

Nach Ende einer Runde:

- server->clients: `SCORE;spielerpunkte*` (wobei `spielerpunkte` eine kommagetrennte Liste von Einträgen in der Form `name:punkte` ist)

Gründe, warum ein Spieler verliert
----------------------------------
- `SEE_BEFORE_FIRST_ROLL`: Spieler wollte sehen, war aber als erster am Zug (es gab also noch keine Ansage vorher)
- `LIED_ABOUT_MIA`: Spieler hat Mäxchen angesagt, ohne Mäxchen zu haben
- `ANNOUNCED_LOSING_DICE`: Spieler hat zu niedrige Würfel angesagt
- `DID_NOT_ANNOUNCE`: Spieler hat nicht (rechtzeitig) angesagt, was gewürfelt wurde
- `DID_NOT_TAKE_TURN`: Spieler hat nicht (rechtzeitig) einen Zug gemacht
- `INVALID_TURN`: Spieler hat einen ungültigen Zug gemacht
- `SEE_FAILED`: Spieler wollte sehen, Ansage des vorhergehenden Spielers war aber richtig
- `CAUGHT_BLUFFING`: Spieler hat mehr angesagt als er hatte, und der nachfolgende Spieler wollte sehen
- `MIA`: Es wurde Mäxchen aufgedeckt


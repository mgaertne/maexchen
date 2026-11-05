Mia protocol
============

Preliminaries
-------------
- Client and server communicate via UDP (using UTF-8 encoded strings)
- Server opens a known port
- Clients register themselves as player and are -- from then on -- notified for each round
- Clients have to respond within in a narrow time frame (250 ms)
- Alternatively a client could register as a spectator. Spectators are not able to actively participate in the game. Yet they will receive all messages every other client would receive
- Messages the server does not understand, or that do not fit the current state of the game, are ignored without an error message. To the server it looks as if the client did not answer at all
- A token is a text created by the server. An answer has to contain the token of the message it answers. Answers with a wrong or outdated token are ignored

Registration
------------
- client->server: `REGISTER;name`
- client->server: `REGISTER_SPECTATOR;name`

Valid names need to satisfy the following criteria:
- no whitespace
- no colons, semicolons, or commas
- up to 20 characters

The server accepts the registration (server->client: `REGISTERED`) if the name is valid and either
- a new client registers for the first time, or
- an existing client re-registers from the same client IP as before (but possibly with a different port; see below).

In case of a successful registration all spectators will be sent the latest score, 0 scores included:

  - server->spectators: `SCORE;playerpoints*` (see below)

In all other cases, the server rejects the registration request (server->client: `REJECTED;reason`, where `reason` is either `INVALID_NAME` or `NAME_ALREADY_TAKEN`).

After a successful registration, the server will send messages to the client using the IP and the port from which the registration message was sent.

Unregistration
--------------
- client->server: `UNREGISTER`
- server->client: `UNREGISTERED`

Heartbeat
---------
Every 2 seconds each client will be sent a "heartbeat" signal from the server, so that it can react when the server (or the communication) dies:

- server->client: `HEARTBEAT`


Round start
-----------

- server->clients: `ROUND STARTING;token`
- client->server: `JOIN;token`

If at least one player participates:
- the server shuffles the participating players
- server->clients: `ROUND STARTED;roundnumber;rules;playernames` (where `playernames` is a ordered, comma separated list of all participating players, in the order in which the round is going to be played, and `rules` names the rules of this round: `mia21`, `mia42`, `mia21,blind` or `mia42,blind`)

The rules may change from round to round and apply to the whole round (see below).

Else:
- server->clients: `ROUND CANCELED;NO_PLAYERS` (a new round is started immediately)

Rounds with just one player are canceled right after their start: `ROUND CANCELED;ONLY_ONE_PLAYER`


Dice and ranking
----------------
Dice are written as `a,b`, with pips from 1 to 6, for example `4,2`. The server always writes the higher die first. In `ANNOUNCE` a client may choose either order: the server reads `2,4` as `4,2`. Invalid dice (for example `7,1`) are ignored.

The rules of a round consist of the Mia rule (`mia21` or `mia42`) and the suffix `,blind` if the blind roll (`BLIND_ROLL`) is allowed in that round. In rounds without `,blind` the server ignores every `BLIND_ROLL`.

Ranking under `mia21`, from lowest to highest:

1. Ordinary rolls: `3,1` (lowest), `3,2`, `4,1`, `4,2`, `4,3`, `5,1`, `5,2`, `5,3`, `5,4`, `6,1`, `6,2`, `6,3`, `6,4`, `6,5`
2. Doubles: `1,1`, `2,2`, `3,3`, `4,4`, `5,5`, `6,6`
3. Mia: `2,1`, the highest roll

Ranking under `mia42`, from lowest to highest:

1. Ordinary rolls: `3,1` (lowest), `3,2`, `4,1`, `4,3`, `5,1`, `5,2`, `5,3`, `5,4`, `6,1`, `6,2`, `6,3`, `6,4`, `6,5`
2. Doubles: `1,1`, `2,2`, `3,3`, `4,4`, `5,5`, `6,6`
3. Half Mia: `2,1`
4. Mia: `4,2`, the highest roll

Only the Mia of the round (`2,1` under `mia21`, `4,2` under `mia42`) is revealed immediately when announced.

An announcement has to be strictly higher than the previous one. The first announcement of a round is free.

Round actions
-------------
In adherence to the previously announced order:
- server->client: `YOUR TURN;token`
- client->server: `command;token` (where `command` has to be either `ROLL`, `BLIND_ROLL` or `SEE`)

On `ROLL`:
- server->clients: `PLAYER ROLLS;name`
- server->client: `ROLLED;dice;token`
- client->server: `ANNOUNCE;dice;token` or, once, `BLIND_ROLL;token`
- server->clients: `ANNOUNCED;name;dice`

On `BLIND_ROLL` (as a turn, or after `ROLL`/`BLIND_ROLL`):
- server->clients: `PLAYER BLIND ROLLS;name`
- server->client: `BLIND_ROLLED;token` (the client does not learn the dice)
- client->server: `ANNOUNCE;dice;token` or, if it was the first roll, `BLIND_ROLL;token`
- server->clients: `ANNOUNCED;name;dice`

A player may roll at most twice per turn, and the second roll is always a `BLIND_ROLL`. The second roll replaces the first; the last roll is the one that is revealed later. After the second roll only the token of `BLIND_ROLLED` is valid. Further rolls, a `ROLL` after a roll, and a `SEE` after a roll are ignored.

When Mia is announced, the round ends and the dice are shown. Given Mia was indeed rolled, all players but the announcer lose, otherwise the announcer loses.
- server -> clients: `PLAYER LOST;names;reason` (where `names` is a comma separated list)

On `SEE`:
- Server checks if last announced dice are valid and determines the losing players
- server->clients: `PLAYER WANTS TO SEE;name`
- server->clients: `ACTUAL DICE;dice`
- server->clients: `PLAYER LOST;name;reason`

Whenever a players does not respond in time or does something wrong:
- server->clients: `PLAYER LOST;name;reason`

Every player that has NOT LOST gets a point added to their score.

At the end of each round:
- server->clients: `SCORE;playerpoints` (where `playerpoints` is a comma separated list with entries in the form of `name:points`)

Reasons for losing a round
--------------------------
- `SEE_BEFORE_FIRST_ROLL`: Player wanted to `SEE`, but was first to act (no dice were announced before)
- `LIED_ABOUT_MIA`: Player announced Mia without actually having rolled Mia
- `ANNOUNCED_LOSING_DICE`: Player announced dice that were lower than the previously announced ones
- `DID_NOT_ANNOUNCE`: Player did not announce (in time)
- `DID_NOT_TAKE_TURN`: Player did not announce turn (in time)
- `INVALID_TURN`: Player commanded an invalid turn
- `SEE_FAILED`: Player wanted to `SEE`, but previous player announced dice correctly
- `CAUGHT_BLUFFING`: Player announced higher dice than actually given and the next player wanted to `SEE`
- `MIA`: Mia was announced


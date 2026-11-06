# 01 — Vibrate in a world someone else hosts

Enshwick's game events (`HitEvent`, `EntityDiedEvent`) are raised by the server's systems. A client sees them only
while it hosts the world itself. Join a friend's world and the hits land on their server: the mod on your machine sees
nothing, and a mod on theirs would try to reach Intiface on *their* localhost.

## Options

- **Client-side health watch.** [Assumption: the client keeps a replicated copy of its own character's health for the
  HUD.] A client mod (`side = "client"`) polls or hooks that value and vibrates on a drop. Needs: whether Enshwick's
  `ui` module or a reflected component exposes it on a client; otherwise a hook found with `load.symbol` /
  [Finding the game](https://github.com/enshwick/enshwick/blob/main/book/src/finding-the-game.md). Catches fall and
  shroud damage too, which `HitEvent` misses.
- **Server relays to the client.** `side = "both"`: the host's server sees the hit and sends it over Enshwick's `net`
  messages to the hit player's client, which talks to its own Intiface. Clean, but every host must install the mod,
  and it takes part in the join check.

[Assumption: the client-side watch is preferred, since it needs nothing from the host.]

## Open

- Does a client have a readable own-health value through the API, without a raw hook?
- Is a hit also visible client-side as an event (damage numbers, hit flash) worth hooking instead?

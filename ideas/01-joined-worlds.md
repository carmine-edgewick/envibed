# 01 — Vibrate in a world someone else hosts

**Experience, death and the Shroud timer: shipped**, client-side, with the official loader (2026-11-06). What found it:

- A `server`-side mod stays on in a joined world; only `both` mods are turned off (`host/join.rs` in the loader).
- The client's ECS world raises no hit, death or XP events (probed with a loader fork that followed it:
  `~/code/enshwick`, branch `client-world`, and `probe/`). `Health`, `Experience` and `FogResistance` are server-only.
- But `ecs::NetworkPlayerAttributes` is replicated to clients: health, experience, level, `fogResistance` and more.
  The client system `player_ui` queries the local player alone; a hook on it reads that component (`src/own.rs`).
  Its iterator is 121 words: `player_ui` passes 0x3c8 to the engine's iterator init (`objdump` at RVA 0x2b8430).
- `enshwick-gamedata --example systems` (added in the fork) lists every system with what it reads and writes.

**Left: dealing damage and kills in joined worlds.** Candidates: `hitstop_effect` (client, melee only), enemy
`NetworkHealth` via `fill_ui_healthbar` (no attacker), or the UI's damage numbers (`doAllDamagePoints()` at
0x140b46024 draws a list of recent points; whatever fills it fires once per hit the server reports to you, ranged
included). The last is the real answer and needs the code that adds to that list.

The rest of this file is the analysis from before the client-side route was found.


Enshwick's game events, world reads and scheduler serve the server. A client sees them only while it hosts the world
itself. And a client runs only the mods the world it joined runs, turning its others off: Enshwick has no client-only
side (`concepts/sides-and-hosting.md`). So on a joined world Envibed does nothing unless the server runs it too.

## The way that works: the server runs it, you alone install it

- Envibed becomes `side = "both"`. The server watches events for every player and sends each reaction over
  Enshwick's mod messages (`messages.md`) to that player's client, which drives its own Intiface on its own localhost.
- The server (a friend's hosted game, or a dedicated server whose files you can reach) installs the loader and Envibed.
- Other players need nothing: with `require_enshwick = false` in the world's
  `<save>-mod-settings/enshwick/enshwick.toml`, the server admits clients without Enshwick, and a vanilla client never
  says hello, so it is never sent anything. A player who *has* Enshwick but not Envibed is kicked by the join check.
- The host can see the mod: it is in the server's mods list, `mods.lock`, and the lobby names Enshwick.
- Solo play keeps working: a hosted game never checks its host, and its server part messages its own client in process.

[Assumption: the per-player pulse is a small message (level, duration, mix) sent from the server's handlers; the
client's half owns the worker thread that today lives in the server's half.]

## Without the server's help

Not through Enshwick. Outside it, either costs far more and does less:

- **Controller rumble.** Forward whatever the game rumbles a gamepad for, through a virtual controller with force
  feedback, to Intiface. Works anywhere, but only for the game's own rumble events, not hits dealt, XP or the Shroud.
- **Reading the screen.** Watch the HUD's Shroud timer and XP bar. Works anywhere, fragile across resolutions, UI
  scale and patches.
- **An own client-side DLL** with raw hooks into the client's replicated state. Possible in principle; a
  reverse-engineering project on its own, re-done after every game patch.

# Done

What shipped from `ideas/`: what it does, where it lives, the decisions that hold weight, the traps.

## Hits and deaths vibrate the toy (2026-11-06)

`src/lib.rs`. `HitEvent` on a player character queues a pulse scaled by the share of max health lost, floored at a
quarter so scratches are felt; `EntityDiedEvent` queues 2 s at full strength. A worker thread owns the Buttplug v3
websocket, so the game thread never waits on Intiface. Tried in single player under Proton (Wine 11) with a Lovense
Lush through native Intiface Central on Linux.

- Only where the world runs: single player or a hosted world. Joined worlds are [01](01-joined-worlds.md).
- Enshwick refuses to build a mod whose `src/texts.toml` lacks any key in `[uk]` beside `[en]`
  (`Language::REQUIRED`).
- Plain `cargo build` can't link on Linux; build through `cargo enshwick install` or `cargo xwin build`.
- The read timeout keeps one thread both listening and sending; Windows reports it as `TimedOut`, not `WouldBlock`.
- Intiface Central here sends `MaxPingTime: 0`, so the ping path is untried.
- Per-world settings land beside the save, e.g. `Enshrouded/world0-mod-settings/carmine-edgewick.envibed/settings.toml`.

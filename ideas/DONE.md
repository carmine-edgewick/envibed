# Done

What shipped from `ideas/`: what it does, where it lives, the decisions that hold weight, the traps.

## Reactions: dealing damage, kills, experience, death (2026-11-06)

`src/lib.rs`. A damaging `HitEvent` whose `root_source_id` is a watched player (so arrows and spells count), and an
`EntityDiedEvent` whose `killer_player` or dead entity is one, each queue a pulse with that reaction's strength and
duration, a `Reaction` settings group per event. The first version reacted to damage *taken*, scaled by health lost;
replaced the same day by dealing damage, at the user's request.
`XpGainedEvent` vibrates at amount/100 (floor 10%) for 1 to 5 s. The worker keeps every running pulse: hit, kill and
death are `Mix::Strongest`, experience is `Mix::Stack` and adds on top, capped at 1. A worker thread owns the Buttplug v3
websocket, so the game thread never waits on Intiface. Tried in single player under Proton (Wine 11) with a Lovense
Lush through native Intiface Central on Linux.

- Only where the world runs: single player or a hosted world. Joined worlds are [01](01-joined-worlds.md).
- Enshwick refuses to build a mod whose `src/texts.toml` lacks any key in `[uk]` beside `[en]`
  (`Language::REQUIRED`).
- Plain `cargo build` can't link on Linux; build through `cargo enshwick install` or `cargo xwin build`.
- The read timeout keeps one thread both listening and sending; Windows reports it as `TimedOut`, not `WouldBlock`.
- Intiface Central here sends `MaxPingTime: 0`, so the ping path is untried.
- Per-world settings land beside the save, e.g. `Enshrouded/world0-mod-settings/carmine-edgewick.envibed/settings.toml`.

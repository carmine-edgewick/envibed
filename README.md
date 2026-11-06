# Envibed

An Enshrouded mod that makes Intiface/Buttplug.io toys vibrate when your character deals damage, makes a kill,
gains experience or dies, and rumbles while the Shroud timer runs down. Each reaction has its own strength and duration; experience scales with the amount.

Built on [Enshwick](https://github.com/enshwick/enshwick), the community mod loader (Rust API, runs under Proton).

## How it fits together

```
Enshrouded.exe (Proton)
 └─ powrprof.dll          Enshwick loader, picked up through WINEDLLOVERRIDES
     └─ mods/carmine-edgewick.envibed/envibed.dll
          HitEvent / EntityDiedEvent  ──channel──►  worker thread ──ws://127.0.0.1:12345──►  Intiface Central (native Linux)
                                                                                                 └─ Bluetooth ─► toy
```

Proton shares the host's network stack, so the Windows DLL reaches the native Linux Intiface on localhost.

## Limits

- Experience, death and the Shroud timer work in any world, worlds you join included: they read your own character as
  your game sees it, from the `NetworkPlayerAttributes` the server replicates to you (`src/own.rs`).
- Dealing damage and kills come from the server's game events, so they work only where your game runs the world:
  single player or a world you host.
- In a world you join, the mod runs on its default settings: settings files belong to worlds, and that world's host
  doesn't have Envibed's.
- Enshwick is alpha (1.0.0-alpha.12, 2026-11-05). A game patch can break the loader until it updates, and the
  `player_ui` hook (its 121-word iterator is read off the game's code) may need redoing. Back up saves.

## Setup

1. **Intiface Central** (native): `paru -S intiface-central-bin`. Start it, press *Start Server* (default port
   12345), and check your toy appears under *Devices*.
2. **Toolchain**:
   ```sh
   rustup target add x86_64-pc-windows-msvc
   cargo install --locked cargo-xwin
   cargo install --locked --git https://github.com/enshwick/enshwick --tag v1.0.0-alpha.12 cargo-enshwick
   ```
3. **Loader**: download `enshwick.zip` from the
   [v1.0.0-alpha.12 release](https://github.com/enshwick/enshwick/releases/tag/v1.0.0-alpha.12) and extract it
   into `~/.local/share/Steam/steamapps/common/Enshrouded/` (adds `powrprof.dll` and `mods/`).
4. **Launch options** (Steam → Enshrouded → Properties): `WINEDLLOVERRIDES="powrprof=n,b" %command%`
5. **Build and install the mod**:
   ```sh
   cargo enshwick install --game ~/.local/share/Steam/steamapps/common/Enshrouded
   ```
6. Start the game. *Mods* in the main menu should list Envibed. Logs are in the game folder under `logs/`:
   `enshwick_*.log` says whether the mod loaded, `carmine-edgewick.envibed_*.log` shows "connected to Intiface" and
   "toy ready".

## Settings

Per world, in `<save>-mod-settings/carmine-edgewick.envibed/settings.toml`, or the Settings button on the Mods screen.
Changes apply when the world loads again. In a world you join the mod runs on these defaults.

| Key | Default | |
| --- | --- | --- |
| `enabled` | `true` | |
| `url` | `ws://127.0.0.1:12345` | Intiface server |
| `hit_strength`, `hit_duration_ms` | `0.4`, `300` | you deal damage, arrows and spells included; worlds you host only |
| `kill_strength`, `kill_duration_ms` | `0.8`, `1000` | you kill something; worlds you host only |
| `death_strength`, `death_duration_ms` | `1.0`, `2000` | you die |
| `xp` | `true` | you gain experience: the amount is the strength in percent (floor 10%), over 1 to 5 s |
| `shroud_low` | `0.05` | Shroud rumble while plenty of time is left |
| `shroud_high` | `0.35` | Shroud rumble at zero; it rises from `shroud_low` over the last 3 minutes |
| `shroud_dangerous` | `1.0` | Shroud rumble in the dangerous Shroud, told apart by the timer draining over 2.5 s per second |

Strengths run 0 to 1, durations up to 10000 ms; 0 turns a reaction off. Every pulse holds for its duration, then fades
over 15 s: fast at first, then a long gentle tail. Of overlapping hit, kill and death pulses only the strongest counts,
so a killing blow buzzes as a kill; experience adds on top, capped at full strength. The Shroud rumble lies under
everything and stops within a second of leaving the Shroud.

Hits and kills count your own character, taken to be the server's first player slot in a world you host; each joining
player's slot is logged to check that.

## Uninstall

Delete `powrprof.dll` from the game folder and remove the launch option.

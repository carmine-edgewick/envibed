# Envibed

An Enshrouded mod that makes Intiface/Buttplug.io toys vibrate when your character gets hit, scaled by how much
health the hit took. Dying gives a 2-second full-strength buzz.

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

- Game events fire on the server side only. That covers single player and worlds **you host**. When you join someone
  else's world, the host's server sees the hits, not your game, so nothing vibrates.
- Fall and shroud damage change health without a hit event, so they don't vibrate.
- Enshwick is alpha (1.0.0-alpha.12, 2026-11-05). A game patch can break the loader until it updates. Back up saves.

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

Per world, in `<save>-mod-settings/carmine-edgewick.envibed/settings.toml`, or the Settings button on the Mods screen:

| Key | Default | |
| --- | --- | --- |
| `enabled` | `true` | |
| `url` | `ws://127.0.0.1:12345` | Intiface server |
| `player` | empty | only this character's hits; empty means every player in the world |
| `strength` | `1.0` | level for a hit that takes all your health; small hits get at least a quarter of it |
| `duration_ms` | `600` | length of one hit's buzz |

Changes apply when the world loads again.

## Uninstall

Delete `powrprof.dll` from the game folder and remove the launch option.

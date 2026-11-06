//! Logs every game event type that reaches mods, counted per 10 s, and dumps the bytes of the ones whose names
//! suggest hits, damage, experience, deaths or the Shroud. Run with the forked loader and
//! `ENSHWICK_CLIENT_WORLD_EVENTS=only` to see the client's own world in solo play.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use enshwick::{Load, Mod, Off};

const INTERESTING: &[&str] = &["hit", "damage", "xp", "experience", "level", "died", "death", "dead", "shroud", "fog", "kill", "health", "attribute"];
/// Dumps per type, so a flood of one kind doesn't bury the rest.
const DUMPS: u32 = 20;

struct Seen {
    since: Instant,
    counts: HashMap<u32, u32>,
    dumped: HashMap<u32, u32>,
}

static SEEN: Mutex<Option<Seen>> = Mutex::new(None);

/// The mod.
#[derive(Debug)]
pub struct Probe;

impl Mod for Probe {
    fn init(load: &mut Load<'_>) -> Result<Self, Off> {
        let types = load.game_event_types()?;
        enshwick::info!("{} event types on this build", types.len());
        let names: HashMap<u32, (&'static str, u32)> = types.iter().map(|t| (t.hash(), (t.name(), t.size()))).collect();
        load.on_every_raw_game_event(move |event| {
            let hash = event.type_hash();
            let name = names.get(&hash).map_or("?", |n| n.0);
            let mut guard = SEEN.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let seen = guard.get_or_insert_with(|| Seen { since: Instant::now(), counts: HashMap::new(), dumped: HashMap::new() });
            *seen.counts.entry(hash).or_default() += 1;
            let lower = name.to_ascii_lowercase();
            let dumped = seen.dumped.entry(hash).or_default();
            if *dumped < DUMPS && INTERESTING.iter().any(|w| lower.contains(w)) {
                *dumped += 1;
                let mut hex = String::new();
                for (i, b) in event.bytes().iter().enumerate() {
                    let _ = write!(hex, "{}{b:02x}", if i % 4 == 0 && i > 0 { " " } else { "" });
                }
                enshwick::info!("{name} ({} bytes): {hex}", event.bytes().len());
            }
            if seen.since.elapsed() > Duration::from_secs(10) {
                let mut counts: Vec<_> = seen.counts.drain().collect();
                counts.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
                let list: Vec<String> = counts.iter().map(|(h, n)| format!("{n}x {}", names.get(h).map_or("?", |n| n.0))).collect();
                enshwick::info!("last 10 s: {}", list.join(", "));
                seen.since = Instant::now();
            }
        })?;
        Ok(Self)
    }
}

enshwick::declare_mod!(Probe);

//! Your own character as your game sees it: health, experience and the Shroud timer, from the
//! `NetworkPlayerAttributes` the server replicates to every client. It runs in the client's own world, so it works in
//! worlds you join as well as in your own.

use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use enshwick::ecs::{ComponentType, Components, Ecs, Worlds};
use enshwick::raw::read_at;
use enshwick::{Load, Next, Off};

use crate::{Mix, pulse, react, settings, shroud_rumble, xp};

const ATTRIBUTES: &str = "keen::ecs::NetworkPlayerAttributes";
/// Words of `player_ui`'s query iterator: it hands the engine's iterator init 0x3c8 bytes.
const WORDS: usize = 121;
/// How long after the Shroud value last dropped the timer still counts as running; it drops once a second.
const TICKING: Duration = Duration::from_millis(2500);
/// A gap this long between samples means menus or another world: start from fresh, don't compare.
const STALE: Duration = Duration::from_secs(2);

struct Engine {
    ecs: Ecs,
    components: Components,
    worlds: Worlds,
    attributes: ComponentType,
    at: Offsets,
}

struct Offsets {
    health: usize,
    experience: usize,
    experience_required: usize,
    level: usize,
    fog: usize,
    fog_max: usize,
}

#[derive(Clone, Copy, Debug)]
struct Stats {
    health: u16,
    experience: u16,
    experience_required: u16,
    level: u16,
    fog: u16,
    fog_max: u16,
}

struct Seen {
    last: Stats,
    at: Instant,
    fog_dropped: Option<Instant>,
    rumbled: Instant,
}

static ENGINE: OnceLock<Engine> = OnceLock::new();
static SEEN: Mutex<Option<Seen>> = Mutex::new(None);

enshwick::detour! {
    static PLAYER_UI: unsafe extern "system" fn(ctx: *mut u64) = player_ui;
}

pub(crate) fn init(load: &mut Load<'_>) -> Result<(), Off> {
    let offset = |field| load.field_offset(ATTRIBUTES, field).map(|o| o as usize);
    let at = Offsets {
        health: offset("health")?,
        experience: offset("experience")?,
        experience_required: offset("experienceRequired")?,
        level: offset("level")?,
        fog: offset("fogResistance")?,
        fog_max: offset("fogResistanceMax")?,
    };
    let engine = Engine {
        ecs: Ecs::resolve(load)?,
        components: Components::resolve(load)?,
        worlds: Worlds::resolve(load)?,
        attributes: ComponentType::resolve(load, ATTRIBUTES)?,
        at,
    };
    let _ = ENGINE.set(engine);
    // The client's UI system for the local player: its query holds your character alone.
    let target = load.system("player_ui")?;
    // SAFETY: a system's update function takes its query context alone, as `PLAYER_UI` declares.
    unsafe { load.hook(&PLAYER_UI, target) }
}

fn player_ui(next: Next<'_, unsafe extern "system" fn(*mut u64)>, ctx: *mut u64) {
    if let Some(e) = ENGINE.get() {
        // SAFETY: the system's live context on its own thread, before vanilla walks it; the row is put back after,
        // so vanilla starts where it would have. `WORDS` is the iterator size the system itself asks for.
        unsafe {
            if !e.worlds.is_server(ctx) {
                let saved = e.ecs.row(ctx);
                e.ecs.set_row(ctx, 0);
                let mut query = e.ecs.query::<WORDS>(ctx);
                if query.next().is_some()
                    && let Some(me) = e.ecs.entity(ctx)
                    && let Some(attrs) = e.components.component(ctx, me, e.attributes)
                {
                    let read = |at| read_at::<u16>(attrs, at);
                    sample(Stats {
                        health: read(e.at.health),
                        experience: read(e.at.experience),
                        experience_required: read(e.at.experience_required),
                        level: read(e.at.level),
                        fog: read(e.at.fog),
                        fog_max: read(e.at.fog_max),
                    });
                }
                drop(query);
                e.ecs.set_row(ctx, saved);
            }
        }
    }
    // SAFETY: the original system takes its own context.
    unsafe { next.call(ctx) }
}

fn sample(now: Stats) {
    let t = Instant::now();
    let mut guard = SEEN.lock().unwrap_or_else(PoisonError::into_inner);
    let fresh = Seen { last: now, at: t, fog_dropped: None, rumbled: t };
    let seen = match guard.as_mut() {
        Some(seen) if t - seen.at < STALE => seen,
        _ => {
            enshwick::info!("watching your character: {now:?}");
            *guard = Some(fresh);
            return;
        }
    };
    let last = seen.last;
    seen.last = now;
    seen.at = t;
    if now.fog < last.fog {
        seen.fog_dropped = Some(t);
    } else if now.fog > last.fog {
        seen.fog_dropped = None;
    }
    let Some(s) = settings().filter(|s| s.enabled) else { return };
    if last.health > 0 && now.health == 0 {
        react(s.death);
    }
    // A level-up resets experience: the gain is what was left to the old level plus what is into the new one.
    let gained = match now.level.checked_sub(last.level) {
        Some(0) => now.experience.saturating_sub(last.experience).into(),
        Some(1) => u32::from(last.experience_required.saturating_sub(last.experience)) + u32::from(now.experience),
        _ => 0,
    };
    if s.xp && gained > 0 {
        xp(gained);
    }
    // At zero the timer stops dropping but keeps draining health, so it counts until the value rises again.
    let ticking = seen.fog_dropped.is_some_and(|d| t - d < TICKING || now.fog == 0);
    if s.shroud && ticking && t - seen.rumbled >= Duration::from_secs(1) {
        seen.rumbled = t;
        enshwick::info_every!(Duration::from_secs(30), "Shroud timer {} of {}", now.fog, now.fog_max);
        pulse(shroud_rumble(now.fog.into()), 1500, Mix::Rumble);
    }
}

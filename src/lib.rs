//! Vibrates Intiface/Buttplug.io devices when your character lands a hit, makes a kill, gains experience or dies, and rumbles while
//! the Shroud timer runs down.
//!
//! The game thread only drops a pulse into a channel; a thread of our own owns the websocket, so a slow or
//! missing Intiface never stalls a frame.

use std::io::ErrorKind;
use std::net::TcpStream;
use std::ops::ControlFlow;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{OnceLock, PoisonError, RwLock};
use std::time::{Duration, Instant};

use enshwick::game_events::{EntityDiedEvent, HitEvent, XpGainedEvent};
use enshwick::{Entity, Host, Load, Mod, Off};
use serde_json::{Map, Value};
use tungstenite::{Message, WebSocket};

enshwick::texts!();

const DEFAULT_URL: &str = "ws://127.0.0.1:12345";
const TICK: Duration = Duration::from_millis(20);
const RETRY: Duration = Duration::from_secs(5);
/// How long a pulse takes to fade out once its full-strength time is over.
const DECAY: Duration = Duration::from_secs(15);
/// Levels go out rounded to this, so a fade sends a command per step rather than one every tick.
const STEP: f32 = 0.05;

/// The settings, in each world's `<stem>-mod-settings/carmine-edgewick.envibed/settings.toml`.
#[enshwick::config]
#[derive(Debug, Clone)]
struct Settings {
    /// Whether the toys react at all.
    enabled: bool,
    /// Intiface Central's server address.
    url: String,
    /// Only this character counts; empty for every player.
    player: String,
    /// When you deal damage.
    #[config(group)]
    hit: Reaction,
    /// When you kill something.
    #[config(group)]
    kill: Reaction,
    /// When you die.
    #[config(group)]
    death: Reaction,
    /// Whether gaining experience vibrates: stronger and longer the more you gain, stacking when gains overlap.
    xp: bool,
    /// Whether a low rumble runs while the Shroud timer counts down, rising to 75% over the last minutes.
    shroud: bool,
}

/// How hard and how long the toy vibrates for one event.
#[enshwick::config(group)]
#[derive(Debug, Clone, Copy, Default)]
struct Reaction {
    /// Vibration strength, 0 to 1; 0 turns this reaction off.
    #[config(range = 0.0..=1.0)]
    strength: f32,
    /// How long it vibrates, in milliseconds; 0 turns this reaction off.
    #[config(range = 0..=10000)]
    duration_ms: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            url: DEFAULT_URL.into(),
            player: String::new(),
            hit: Reaction { strength: 0.4, duration_ms: 300 },
            kill: Reaction { strength: 0.8, duration_ms: 1000 },
            death: Reaction { strength: 1.0, duration_ms: 2000 },
            xp: true,
            shroud: true,
        }
    }
}

static SETTINGS: RwLock<Option<Settings>> = RwLock::new(None);
static PULSES: OnceLock<Sender<Pulse>> = OnceLock::new();

fn settings() -> Option<Settings> {
    SETTINGS.read().unwrap_or_else(PoisonError::into_inner).clone()
}

struct Pulse {
    level: f32,
    until: Instant,
    mix: Mix,
}

impl Pulse {
    /// Full level until `until`, then an ease-out over `DECAY`: it drops fast at first and trails off gently.
    fn level(&self, now: Instant) -> f32 {
        if self.mix == Mix::Rumble && now >= self.until {
            return 0.0;
        }
        let past = now.saturating_duration_since(self.until).as_secs_f32();
        let left = 1.0 - (past / DECAY.as_secs_f32()).min(1.0);
        self.level * left * left
    }
}

/// How a pulse combines with the others running.
#[derive(PartialEq)]
enum Mix {
    /// Only the strongest of these counts, so a killing blow feels like a kill, not a hit plus a kill.
    Strongest,
    /// Adds its level on top, so a burst of experience builds up.
    Stack,
    /// A steady level under everything else, refreshed every second and gone without a fade once it isn't.
    Rumble,
}

fn pulse(level: f32, ms: u32, mix: Mix) {
    if level <= 0.0 || ms == 0 {
        return;
    }
    if let Some(tx) = PULSES.get() {
        let _ = tx.send(Pulse { level, until: Instant::now() + Duration::from_millis(ms.into()), mix });
    }
}

fn react(r: Reaction) {
    pulse(r.strength, r.duration_ms, Mix::Strongest);
}

/// Low until the last three minutes, then rising on a curve to 75% as the Shroud timer reaches zero.
fn shroud_rumble(secs_left: i32) -> f32 {
    let close = 1.0 - (secs_left.max(0) as f32 / 180.0).min(1.0);
    0.1 + 0.65 * close * close
}

/// The mod.
#[derive(Debug)]
pub struct Envibed;

impl Mod for Envibed {
    fn init(load: &mut Load<'_>) -> Result<Self, Off> {
        load.texts()?;
        load.on_settings(|s: &Settings| *SETTINGS.write().unwrap_or_else(PoisonError::into_inner) = Some(s.clone()))?;
        let world = load.world()?;
        // The settings to react with, if `entity` is a player we watch.
        let watched = move |entity: Entity| {
            let s = settings().filter(|s| s.enabled)?;
            let player = world.player(entity)?;
            (s.player.is_empty() || player.name.eq_ignore_ascii_case(&s.player)).then_some(s)
        };
        load.on::<HitEvent>(move |hit| {
            // The root source credits the player for arrows and spells, not just melee.
            let Some(attacker) = hit.root_source_id.filter(|a| *a != hit.target_id) else { return };
            if hit.health_change < 0
                && let Some(s) = watched(attacker)
            {
                react(s.hit);
            }
        })?;
        load.on::<EntityDiedEvent>(move |died| {
            if let Some(s) = watched(died.dead_entity_id) {
                react(s.death);
            } else if let Some(s) = died.killer_player.and_then(watched) {
                react(s.kill);
            }
        })?;
        load.on::<XpGainedEvent>(move |gain| {
            if watched(gain.player_entity_id).is_some_and(|s| s.xp) {
                // Gains run from about 2 to 100: the amount is the strength in percent, floored at 10%, over 1 to 5 s.
                let share = (gain.amount as f32 / 100.0).clamp(0.0, 1.0);
                pulse(share.max(0.1), 1000 + (4000.0 * share) as u32, Mix::Stack);
            }
        })?;
        load.scheduler()?.every(Duration::from_secs(1), move || {
            let rumble = world
                .players()
                .into_iter()
                .filter(|p| watched(p.entity).is_some_and(|s| s.shroud) && world.in_shroud(p.entity) == Some(true))
                .filter_map(|p| world.shroud_time(p.entity))
                .map(|t| shroud_rumble(t.current))
                .fold(0.0, f32::max);
            // Outlives the next refresh by half a second, so the rumble doesn't flicker between ticks.
            pulse(rumble, 1500, Mix::Rumble);
            ControlFlow::Continue(())
        });
        Ok(Self)
    }

    fn ready(&'static self, _host: Host) {
        let (tx, rx) = mpsc::channel();
        let _ = PULSES.set(tx);
        let _ = std::thread::Builder::new().name("envibed".into()).spawn(move || run(&rx));
    }
}

enshwick::declare_mod!(Envibed);

type Res<T> = Result<T, Box<dyn std::error::Error>>;

fn run(pulses: &Receiver<Pulse>) {
    loop {
        let url = settings().map_or_else(|| DEFAULT_URL.into(), |s| s.url);
        match Toys::connect(&url) {
            Ok(mut toys) => {
                enshwick::info!("connected to Intiface at {url}");
                if let Err(e) = toys.serve(pulses) {
                    enshwick::warn!("lost Intiface: {e}");
                }
            }
            Err(e) => enshwick::warn_every!(Duration::from_secs(60), "no Intiface at {url}: {e}"),
        }
        // Hits taken while offline would all fire at once on reconnect.
        while pulses.try_recv().is_ok() {}
        std::thread::sleep(RETRY);
    }
}

/// A Buttplug v3 client session.
struct Toys {
    ws: WebSocket<TcpStream>,
    id: u32,
    /// Device index, and the indices of its vibrate actuators among its `ScalarCmd` features.
    devices: Vec<(u64, Vec<usize>)>,
    ping: Option<Duration>,
}

impl Toys {
    fn connect(url: &str) -> Res<Self> {
        let addr = url.trim_start_matches("ws://").split('/').next().unwrap_or_default();
        let stream = TcpStream::connect(addr)?;
        let (ws, _) = tungstenite::client(url, stream).map_err(|e| e.to_string())?;
        // Short reads let one thread both listen and send; set after the handshake, which wants to block.
        ws.get_ref().set_read_timeout(Some(TICK))?;
        let mut toys = Self { ws, id: 0, devices: Vec::new(), ping: None };
        toys.send("RequestServerInfo", serde_json::json!({ "ClientName": "Envibed", "MessageVersion": 3 }))?;
        toys.send("RequestDeviceList", serde_json::json!({}))?;
        // Keep scanning so toys switched on mid-game show up.
        toys.send("StartScanning", serde_json::json!({}))?;
        Ok(toys)
    }

    fn send(&mut self, kind: &str, mut body: Value) -> Res<()> {
        self.id += 1;
        body["Id"] = self.id.into();
        let msg = Value::Array(vec![Value::Object(Map::from_iter([(kind.to_owned(), body)]))]);
        self.ws.send(Message::text(msg.to_string()))?;
        Ok(())
    }

    fn vibrate(&mut self, level: f32) -> Res<()> {
        for (device, motors) in self.devices.clone() {
            let scalars: Vec<Value> = motors
                .iter()
                .map(|i| serde_json::json!({ "Index": i, "Scalar": level, "ActuatorType": "Vibrate" }))
                .collect();
            self.send("ScalarCmd", serde_json::json!({ "DeviceIndex": device, "Scalars": scalars }))?;
        }
        Ok(())
    }

    fn serve(&mut self, pulses: &Receiver<Pulse>) -> Res<()> {
        let (mut running, mut sent) = (Vec::<Pulse>::new(), 0.0);
        let mut pinged = Instant::now();
        loop {
            match pulses.recv_timeout(TICK) {
                Ok(p) => running.push(p),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return Ok(()),
            }
            let now = Instant::now();
            running.retain(|p| p.until + DECAY > now);
            let level = |mix: Mix| running.iter().filter(move |p| p.mix == mix).map(|p| p.level(now));
            let strongest = level(Mix::Strongest).fold(0.0, f32::max);
            let rumble = level(Mix::Rumble).fold(0.0, f32::max);
            let want = ((rumble + strongest + level(Mix::Stack).sum::<f32>()).min(1.0) / STEP).round() * STEP;
            if want != sent {
                self.vibrate(want)?;
                sent = want;
            }
            if self.ping.is_some_and(|max| pinged.elapsed() > max / 2) {
                self.send("Ping", serde_json::json!({}))?;
                pinged = Instant::now();
            }
            self.receive()?;
        }
    }

    fn receive(&mut self) -> Res<()> {
        loop {
            match self.ws.read() {
                Ok(Message::Text(text)) => {
                    let Ok(Value::Array(msgs)) = serde_json::from_str(&text) else { continue };
                    for (kind, body) in msgs.iter().filter_map(Value::as_object).flatten() {
                        self.handle(kind, body);
                    }
                }
                Ok(_) => {}
                // Windows (and so Proton) reports a read timeout as TimedOut, Linux as WouldBlock.
                Err(tungstenite::Error::Io(e)) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                    return Ok(());
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    fn handle(&mut self, kind: &str, body: &Value) {
        match kind {
            "ServerInfo" => {
                let ms = body["MaxPingTime"].as_u64().unwrap_or(0);
                self.ping = (ms > 0).then(|| Duration::from_millis(ms));
            }
            "DeviceList" => {
                for d in body["Devices"].as_array().into_iter().flatten() {
                    self.add(d);
                }
            }
            "DeviceAdded" => self.add(body),
            "DeviceRemoved" => {
                let index = body["DeviceIndex"].as_u64();
                self.devices.retain(|(d, _)| Some(*d) != index);
            }
            "Error" => enshwick::warn!("Intiface: {}", body["ErrorMessage"]),
            _ => {}
        }
    }

    fn add(&mut self, d: &Value) {
        let Some(index) = d["DeviceIndex"].as_u64() else { return };
        let motors: Vec<usize> = d["DeviceMessages"]["ScalarCmd"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
            .filter(|(_, f)| f["ActuatorType"] == "Vibrate")
            .map(|(i, _)| i)
            .collect();
        if motors.is_empty() || self.devices.iter().any(|(d, _)| *d == index) {
            return;
        }
        enshwick::info!("toy ready: {} ({} motors)", d["DeviceName"], motors.len());
        self.devices.push((index, motors));
    }
}

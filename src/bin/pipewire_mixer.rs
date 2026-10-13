use eframe::egui;
use midir::{Ignore, MidiInput, MidiInputConnection};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::PathBuf,
    process::Command,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

const FADERS: usize = 5;
const STEAM: &str = "steam-all";

#[derive(Clone, Debug)]
struct Stream {
    id: u64,
    name: String,
    detail: String,
    pid: Option<u32>,
    app_name: String,
}

impl Stream {
    fn is_steam(&self) -> bool {
        if self.app_name.to_lowercase().starts_with("steam_app_") {
            return true;
        }
        self.pid.and_then(|pid| fs::read(format!("/proc/{pid}/environ")).ok())
            .map(|bytes| bytes.split(|byte| *byte == 0).any(|field| {
                (field.starts_with(b"SteamAppId=") || field.starts_with(b"SteamGameId="))
                    && field.split(|byte| *byte == b'=').nth(1).is_some_and(|value| !value.is_empty())
            })).unwrap_or(false)
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Config {
    #[serde(default = "default_apps")]
    applications: Vec<String>,
    #[serde(default)]
    midi_port: String,
    #[serde(default = "default_notes", alias = "mute_ccs")]
    mute_notes: Vec<Option<u8>>,
    #[serde(default = "default_volumes")]
    fader_volumes: Vec<f32>,
}

fn default_apps() -> Vec<String> { vec![String::new(); FADERS] }
fn default_notes() -> Vec<Option<u8>> { (37..=41).map(Some).collect() }
fn default_volumes() -> Vec<f32> { vec![0.5; FADERS] }
impl Default for Config {
    fn default() -> Self { Self { applications: default_apps(), midi_port: String::new(), mute_notes: default_notes(), fader_volumes: default_volumes() } }
}
impl Config {
    fn normalize(&mut self) {
        self.applications.resize(FADERS, String::new()); self.applications.truncate(FADERS);
        self.mute_notes.resize(FADERS, None); self.mute_notes.truncate(FADERS);
        self.fader_volumes.resize(FADERS, 0.5); self.fader_volumes.truncate(FADERS);
        for value in &mut self.fader_volumes { *value = value.clamp(0.0, 1.0); }
    }
    fn path() -> PathBuf { std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join(".config/pipewire-mixer/config.json") }
    fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); }
        if let Ok(text) = serde_json::to_vec_pretty(self) {
            let temp = path.with_extension("json.tmp");
            if fs::write(&temp, text).is_ok() { let _ = fs::rename(temp, path); }
        }
    }
}

enum Event { Midi(Vec<u8>), Streams(Result<Vec<Stream>, String>), MidiPorts(Result<Vec<String>, String>) }

struct MixerApp {
    config: Config,
    streams: Vec<Stream>,
    midi_ports: Vec<String>,
    midi_rx: Receiver<Event>,
    midi_tx: Sender<Event>,
    midi_conn: Option<MidiInputConnection<()>>,
    connected_port: String,
    status: String,
    stream_status: String,
    muted: [bool; FADERS],
    learning: Option<usize>,
    last_stream_poll: Instant,
    stream_poll_pending: bool,
    last_port_poll: Instant,
    save_at: Option<Instant>,
    last_applied: Vec<(u64, u32)>,
    last_midi_refresh: Instant,
}

impl MixerApp {
    fn new() -> Self {
        let mut config = Config::default();
        if let Ok(data) = fs::read(Config::path()) { if let Ok(parsed) = serde_json::from_slice(&data) { config = parsed; } }
        config.normalize();
        let (tx, rx) = mpsc::channel();
        let mut app = Self { config, streams: vec![], midi_ports: vec![], midi_rx: rx, midi_tx: tx,
            midi_conn: None, connected_port: String::new(), status: "Searching for MIDI devices…".into(),
            stream_status: "Connecting to PipeWire…".into(), muted: [false; FADERS], learning: None,
            last_stream_poll: Instant::now() - Duration::from_secs(10), last_port_poll: Instant::now() - Duration::from_secs(10),
            stream_poll_pending: false, save_at: None, last_applied: vec![], last_midi_refresh: Instant::now() };
        app.refresh_ports(); app
    }
    fn schedule_save(&mut self) { self.save_at = Some(Instant::now() + Duration::from_millis(350)); }
    fn refresh_ports(&mut self) {
        let result = MidiInput::new("PipeWire Mixer").map(|input| input.ports().iter().filter_map(|port| input.port_name(port).ok()).collect::<Vec<_>>()).map_err(|e| e.to_string());
        match result { Ok(mut ports) => { ports.sort_by_key(|p| p.to_lowercase()); self.midi_ports = ports; self.connect_midi(); }, Err(e) => self.status = format!("MIDI unavailable · {e}") }
        self.last_port_poll = Instant::now();
    }
    fn connect_midi(&mut self) {
        if self.midi_conn.is_some() && self.midi_ports.contains(&self.connected_port) { return; }
        self.midi_conn = None; self.connected_port.clear();
        let chosen = if self.midi_ports.contains(&self.config.midi_port) { self.config.midi_port.clone() } else { self.midi_ports.first().cloned().unwrap_or_default() };
        if chosen.is_empty() { self.status = "No MIDI input found · reconnecting automatically".into(); return; }
        let mut input = match MidiInput::new("PipeWire Mixer") { Ok(value) => value, Err(e) => { self.status = format!("MIDI error · {e}"); return; } };
        input.ignore(Ignore::None);
        let Some(port) = input.ports().into_iter().find(|p| input.port_name(p).ok().as_deref() == Some(chosen.as_str())) else { self.status = "MIDI device changed · reconnecting…".into(); return; };
        let tx = self.midi_tx.clone();
        match input.connect(&port, "pipewire-mixer-input", move |_stamp, message, _| { let _ = tx.send(Event::Midi(message.to_vec())); }, ()) {
            Ok(connection) => { self.connected_port = chosen.clone(); self.config.midi_port = chosen.clone(); self.midi_conn = Some(connection); self.status = format!("Connected · {chosen}"); self.schedule_save(); }
            Err(e) => self.status = format!("Could not connect · {e} · retrying"),
        }
    }
    fn poll_streams(&mut self) {
        if self.stream_poll_pending { return; }
        self.stream_poll_pending = true;
        self.last_stream_poll = Instant::now();
        let tx = self.midi_tx.clone();
        thread::spawn(move || {
            let output = Command::new("pw-dump").output();
            let result = output.map_err(|e| format!("pw-dump: {e}" )).and_then(|out| {
            if !out.status.success() { return Err(String::from_utf8_lossy(&out.stderr).trim().to_owned()); }
            let data: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
            parse_streams(&data)
            });
            let _ = tx.send(Event::Streams(result));
        });
    }
    fn assigned(&self, i: usize) -> Vec<Stream> {
        let selected = self.config.applications.get(i).map(String::as_str).unwrap_or("");
        if selected == STEAM { self.streams.iter().filter(|s| s.is_steam()).cloned().collect() }
        else { self.streams.iter().filter(|s| s.name == selected && !selected.is_empty()).take(1).cloned().collect() }
    }
    fn apply(&mut self, index: usize) {
        let streams = self.assigned(index);
        let volume = self.config.fader_volumes[index];
        for stream in streams {
            let key = (stream.id, (volume * 10000.0) as u32);
            if !self.last_applied.contains(&key) {
                let _ = Command::new("wpctl").args(["set-volume", &stream.id.to_string(), &format!("{volume:.3}")]).status();
                self.last_applied.retain(|(id, _)| *id != stream.id); self.last_applied.push(key);
            }
            let _ = Command::new("wpctl").args(["set-mute", &stream.id.to_string(), if self.muted[index] { "1" } else { "0" }]).status();
        }
    }
    fn sync_all(&mut self) { for i in 0..FADERS { self.apply(i); } }
    fn process_events(&mut self) {
        while let Ok(event) = self.midi_rx.try_recv() {
            match event {
                Event::Midi(data) if data.len() >= 3 => {
                    let command = data[0] & 0xF0;
                    if let Some(i) = self.learning {
                        if command == 0x90 || command == 0x80 { self.config.mute_notes[i] = Some(data[1]); self.learning = None; self.schedule_save(); }
                    } else if command == 0xB0 && (1..=5).contains(&data[1]) {
                        let i = (data[1] - 1) as usize; self.config.fader_volumes[i] = data[2] as f32 / 127.0; self.apply(i); self.schedule_save();
                    } else if command == 0x90 || command == 0x80 {
                        for i in 0..FADERS { if self.config.mute_notes[i] == Some(data[1]) { self.muted[i] = command == 0x90 && data[2] > 0; self.apply(i); } }
                    }
                }
                Event::MidiPorts(Ok(ports)) => { let mut ports = ports; ports.sort_by_key(|p| p.to_lowercase()); if ports != self.midi_ports { self.midi_ports = ports; self.midi_conn = None; self.connect_midi(); } }
                Event::MidiPorts(Err(e)) => self.status = format!("MIDI unavailable · {e}"),
                Event::Streams(result) => {
                    self.stream_poll_pending = false;
                    match result { Ok(streams) => { self.streams = streams; self.stream_status = format!("{} output streams · refreshed automatically", self.streams.len()); }, Err(e) => self.stream_status = format!("PipeWire unavailable · {e}") }
                    self.sync_all();
                }
                Event::Midi(_) => {}
            }
        }
        if self.save_at.is_some_and(|at| Instant::now() >= at) { self.config.save(); self.save_at = None; }
    }
    fn fader_card(&mut self, ui: &mut egui::Ui, i: usize) {
        let assigned = self.assigned(i);
        let current = self.config.applications[i].clone();
        egui::Frame::group(ui.style()).inner_margin(egui::Margin::same(16)).show(ui, |ui| {
            ui.set_min_width(205.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{:02}", i + 1)).size(13.0).strong().color(egui::Color32::from_rgb(139, 124, 255)));
                ui.label(egui::RichText::new(format!("FADER {}", i + 1)).size(15.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.checkbox(&mut self.muted[i], "Mute"); });
            });
            ui.add_space(8.0);
            let mut selection = current.clone();
            egui::ComboBox::from_id_salt(("assignment", i)).width(ui.available_width()).selected_text(if current.is_empty() { "Choose an application…" } else if current == STEAM { "All Steam games" } else { &current }).show_ui(ui, |ui| {
                ui.selectable_value(&mut selection, String::new(), "No application");
                ui.selectable_value(&mut selection, STEAM.to_string(), "All Steam games");
                for stream in &self.streams { ui.selectable_value(&mut selection, stream.name.clone(), &stream.name); }
                if !current.is_empty() && current != STEAM && !self.streams.iter().any(|s| s.name == current) { ui.label(egui::RichText::new(format!("{current} · offline")).weak()); }
            });
            if selection != current { self.config.applications[i] = selection; self.last_applied.clear(); self.apply(i); self.schedule_save(); }
            ui.add_space(5.0);
            let subtitle = assigned.first().map(|s| s.detail.as_str()).unwrap_or(if current == STEAM { "Steam game streams" } else if current.is_empty() { "Fader has no target" } else { "Saved target is currently offline" });
            ui.label(egui::RichText::new(subtitle).size(11.0).color(ui.visuals().weak_text_color()));
            ui.add_space(7.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("LEVEL").size(10.0).strong().color(ui.visuals().weak_text_color()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(egui::RichText::new(format!("{}%", (self.config.fader_volumes[i] * 100.0).round() as u32)).monospace().strong()); });
            });
            let response = ui.add(egui::Slider::new(&mut self.config.fader_volumes[i], 0.0..=1.0).show_value(false).trailing_fill(true));
            if response.changed() { self.apply(i); self.schedule_save(); }
            ui.horizontal(|ui| {
                let note = self.config.mute_notes[i].map(|n| format!("Mute note {n}")).unwrap_or_else(|| "Bind mute".into());
                let label = if self.learning == Some(i) { "Press mute key…" } else { &note };
                if ui.small_button(label).clicked() { self.learning = if self.learning == Some(i) { None } else { Some(i) }; }
                ui.label(egui::RichText::new(if assigned.is_empty() { "Idle" } else { "Live" }).size(11.0).color(if assigned.is_empty() { ui.visuals().weak_text_color() } else { egui::Color32::from_rgb(112, 205, 155) }));
            });
        });
    }
}

fn parse_streams(data: &[serde_json::Value]) -> Result<Vec<Stream>, String> {
    let mut seen = HashSet::new(); let mut streams = Vec::new();
    for item in data {
        if item.get("type").and_then(|v| v.as_str()) != Some("PipeWire:Interface:Node") { continue; }
        let Some(id) = item.get("id").and_then(|v| v.as_u64()) else { continue; };
        let props = &item["info"]["props"];
        if props["media.class"].as_str() != Some("Stream/Output/Audio") || !seen.insert(id) { continue; }
        let app_name = props["application.name"].as_str().unwrap_or("").trim().to_owned();
        let binary = props["application.process.binary"].as_str().unwrap_or("").rsplit('/').next().unwrap_or("");
        let detail = props["node.description"].as_str().filter(|s| !s.is_empty()).unwrap_or(binary).to_owned();
        let name = if !app_name.is_empty() { app_name.clone() } else if !binary.is_empty() { binary.into() } else if !detail.is_empty() { detail.clone() } else { format!("Audio stream {id}") };
        let pid = props["application.process.id"].as_u64().or_else(|| props["application.process.id"].as_str().and_then(|s| s.parse().ok())).map(|v| v as u32);
        streams.push(Stream { id, name, detail, pid, app_name });
    }
    streams.sort_by_key(|s| (s.name.to_lowercase(), s.id)); Ok(streams)
}

impl eframe::App for MixerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_events();
        if self.last_stream_poll.elapsed() > Duration::from_secs(2) { self.poll_streams(); }
        if self.last_port_poll.elapsed() > Duration::from_secs(2) {
            let tx = self.midi_tx.clone();
            thread::spawn(move || { let result = MidiInput::new("PipeWire Mixer").map(|i| i.ports().iter().filter_map(|p| i.port_name(p).ok()).collect()).map_err(|e| e.to_string()); let _ = tx.send(Event::MidiPorts(result)); });
            self.last_port_poll = Instant::now();
        }
        if self.midi_conn.is_none() && self.last_midi_refresh.elapsed() > Duration::from_secs(2) { self.refresh_ports(); self.last_midi_refresh = Instant::now(); }
        ctx.request_repaint_after(Duration::from_millis(150));
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| { ui.label(egui::RichText::new("PIPEWIRE MIXER").size(12.0).strong().color(egui::Color32::from_rgb(152, 139, 255))); ui.label(egui::RichText::new("Five faders. Every stream in hand.").size(24.0).strong()); });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { if ui.button("↻  Refresh").clicked() { self.poll_streams(); self.refresh_ports(); } });
            });
            ui.add_space(10.0);
            egui::Frame::new().fill(ui.visuals().faint_bg_color).corner_radius(10.0).inner_margin(12.0).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("MIDI INPUT").size(10.0).strong().color(ui.visuals().weak_text_color()));
                    egui::ComboBox::from_id_salt("midi-input").selected_text(if self.connected_port.is_empty() { "No device connected" } else { &self.connected_port }).show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.config.midi_port, String::new(), "Automatic / none");
                        for port in &self.midi_ports { ui.selectable_value(&mut self.config.midi_port, port.clone(), port); }
                    });
                    if self.config.midi_port != self.connected_port { self.midi_conn = None; self.connect_midi(); self.schedule_save(); }
                    ui.colored_label(if self.midi_conn.is_some() { egui::Color32::from_rgb(112,205,155) } else { egui::Color32::from_rgb(239,180,100) }, format!("●  {}", self.status));
                    ui.separator(); ui.label(egui::RichText::new("CC 1–5 → faders").small().weak());
                });
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| { ui.label(egui::RichText::new(&self.stream_status).size(12.0).color(ui.visuals().weak_text_color())); ui.label(egui::RichText::new("·  reconnects and new streams are detected automatically").size(12.0).color(ui.visuals().weak_text_color())); });
            ui.add_space(8.0);
            egui::ScrollArea::horizontal().auto_shrink([false, true]).show(ui, |ui| {
                ui.horizontal_top(|ui| { for i in 0..FADERS { self.fader_card(ui, i); } });
            });
            ui.add_space(10.0);
            ui.label(egui::RichText::new("ASSIGNMENTS STAY SAVED WHEN AN APP CLOSES. When it returns, its fader reconnects by application name. Steam Games controls current and future Steam audio streams.").size(11.0).color(ui.visuals().weak_text_color()));
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions { viewport: egui::ViewportBuilder::default().with_title("PipeWire Mixer").with_inner_size([1220.0, 690.0]).with_min_inner_size([720.0, 560.0]), ..Default::default() };
    eframe::run_native("PipeWire Mixer", options, Box::new(|_| Ok(Box::new(MixerApp::new()))))
}

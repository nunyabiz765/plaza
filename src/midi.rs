//! USB MIDI input via midir (ALSA sequencer on Linux, WinMM on Windows,
//! CoreMIDI on macOS). Connects to the first real
//! input port it finds; the UI rescans every few seconds for hot-plugging.

use crate::audio::{send, Events};
use plaza::engine::Event;
use plaza::keys::{parse_midi, Midi};

use midir::{Ignore, MidiInput, MidiInputConnection};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::Arc;

/// Which MIDI notes are down, for lighting the on-screen keys.
#[derive(Default)]
pub struct NoteBits([AtomicU64; 2]);

impl NoteBits {
    fn set(&self, n: u8, on: bool) {
        let (w, b) = ((n / 64) as usize, n % 64);
        if on {
            self.0[w].fetch_or(1 << b, Relaxed);
        } else {
            self.0[w].fetch_and(!(1 << b), Relaxed);
        }
    }
    pub fn get(&self, n: u8) -> bool {
        n < 128 && self.0[(n / 64) as usize].load(Relaxed) & (1 << (n % 64)) != 0
    }
    fn clear(&self) {
        self.0[0].store(0, Relaxed);
        self.0[1].store(0, Relaxed);
    }
}

pub struct MidiIn {
    _conn: MidiInputConnection<()>,
    pub port: String,
}

fn is_real_port(name: &str) -> bool {
    !name.contains("Midi Through")
}

pub fn port_names() -> Vec<String> {
    let Ok(input) = MidiInput::new("plaza-scan") else { return Vec::new() };
    input.ports().iter().filter_map(|p| input.port_name(p).ok()).collect()
}

/// Ok(None) when there is simply no device plugged in.
pub fn connect(events: Option<Events>, notes: Arc<NoteBits>) -> Result<Option<MidiIn>, String> {
    let mut input = MidiInput::new("Plaza").map_err(|e| e.to_string())?;
    input.ignore(Ignore::All);
    let ports = input.ports();
    let Some((port, name)) = ports
        .iter()
        .filter_map(|p| input.port_name(p).ok().map(|n| (p.clone(), n)))
        .find(|(_, n)| is_real_port(n))
    else {
        return Ok(None);
    };
    notes.clear();
    let conn = input
        .connect(
            &port,
            "plaza-in",
            move |_, msg, _| {
                let Some(m) = parse_midi(msg) else { return };
                let e = match m {
                    Midi::NoteOn(n, v) => {
                        notes.set(n, true);
                        Event::NoteOn { note: n, velocity: v as f32 / 127.0 }
                    }
                    Midi::NoteOff(n) => {
                        notes.set(n, false);
                        Event::NoteOff { note: n }
                    }
                    Midi::Bend(b) => Event::Bend(b * 2.0),
                    Midi::ModWheel(m) => Event::ModWheel(m),
                    Midi::Sustain(s) => Event::Sustain(s),
                    Midi::AllNotesOff => {
                        notes.clear();
                        Event::AllNotesOff
                    }
                };
                if let Some(ev) = &events {
                    send(ev, e);
                }
            },
            (),
        )
        .map_err(|e| e.to_string())?;
    Ok(Some(MidiIn { _conn: conn, port: name }))
}

/// Is the connected port still plugged in?
pub fn still_present(port: &str) -> bool {
    port_names().iter().any(|n| n == port)
}

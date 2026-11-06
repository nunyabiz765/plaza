// Windows: a GUI app, so double-clicking doesn't open a console window too. The
// command-line subcommands attach to the console they were started from.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod audio;
mod midi;

use plaza::demo::{self, Layer};
use plaza::engine::{Engine, Event};
use plaza::presets;
use std::path::PathBuf;
use std::time::{Duration, Instant};

slint::include_modules!();

const HELP: &str = "\
Plaza: an 80s polysynth for vaporwave

USAGE
  plaza [--preset NAME] [--muted]        Open the synth
  plaza render [options] PRESET=PHRASE... Render offline to a WAV (no audio device)
  plaza presets                          List presets
  plaza midi                             List MIDI input ports
  plaza probe [SECONDS]                  Run the real-time engine MUTED and report timing
  plaza bench                            Measure DSP speed offline

RENDER OPTIONS
  -o FILE       Output WAV (24-bit, default plaza-render.wav)
  --bpm N       Tempo (default 84)
  --slow        Slow + pitch down (also stretches the timing, like a slowed record)
  --rate N      Sample rate (default 48000)
  --tail S      Seconds of release tail (default 4)
  Phrases: chords keys melody stabs bass scale. Several layers are summed:
    plaza render -o mall.wav --slow 'Mall Atrium=chords' 'Glass Lead=melody'

KEYS (in the window)
  Z S X D C V G B H N J M  lower octave     Q 2 W 3 E R 5 T 6 Y 7 U I  upper octave
  [ ] or ← →  octave    ↑ ↓  preset    Tab  slow + pitch down    Esc  all notes off
";

fn find_preset(name: &str) -> Option<usize> {
    let want = name.to_lowercase().replace(['-', '_'], " ");
    let all = presets::all();
    all.iter()
        .position(|p| p.name.to_lowercase() == want)
        .or_else(|| all.iter().position(|p| p.name.to_lowercase().starts_with(&want)))
}

fn die(msg: impl std::fmt::Display) -> ! {
    eprintln!("plaza: {msg}");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(windows)]
    if !args.is_empty() {
        use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
        unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
    }
    match args.first().map(String::as_str) {
        Some("render") => render(&args[1..]),
        Some("presets") => {
            for p in presets::all() {
                println!("{:<16} {}", p.name, p.blurb);
            }
        }
        Some("midi") => {
            let ports = midi::port_names();
            if ports.is_empty() {
                println!("No MIDI input ports.");
            }
            for p in ports {
                println!("{p}");
            }
        }
        Some("probe") => probe(args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5.0)),
        Some("bench") => bench(),
        Some("--version" | "-V") => println!("plaza {}", env!("CARGO_PKG_VERSION")),
        Some("--help" | "-h" | "help") => print!("{HELP}"),
        _ => {
            let mut preset = None;
            let mut muted = false;
            let mut it = args.iter();
            while let Some(a) = it.next() {
                match a.as_str() {
                    "--preset" => {
                        let n = it.next().unwrap_or_else(|| die("--preset needs a name"));
                        preset = Some(find_preset(n).unwrap_or_else(|| die(format!("no preset '{n}'"))));
                    }
                    "--muted" => muted = true,
                    other => die(format!("unknown argument '{other}'. Try --help.")),
                }
            }
            if let Err(e) = app::run(preset, muted) {
                die(format!("couldn't open a window: {e}"));
            }
        }
    }
}

fn render(args: &[String]) {
    let mut out = PathBuf::from("plaza-render.wav");
    let (mut bpm, mut slow, mut rate, mut tail) = (84.0, false, 48_000u32, 4.0);
    let mut layers = Vec::new();
    let mut it = args.iter();
    let num = |v: Option<&String>, what: &str| -> f32 {
        v.and_then(|s| s.parse().ok()).unwrap_or_else(|| die(format!("{what} needs a number")))
    };
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" => out = PathBuf::from(it.next().unwrap_or_else(|| die("-o needs a file"))),
            "--bpm" => bpm = num(it.next(), "--bpm"),
            "--slow" => slow = true,
            "--rate" => rate = num(it.next(), "--rate") as u32,
            "--tail" => tail = num(it.next(), "--tail"),
            spec => {
                let (p, ph) = spec.split_once('=').unwrap_or((spec, "chords"));
                let pi = find_preset(p).unwrap_or_else(|| die(format!("no preset '{p}'")));
                let notes = demo::phrase(ph).unwrap_or_else(|| die(format!("no phrase '{ph}'")));
                layers.push((pi, notes));
            }
        }
    }
    if layers.is_empty() {
        die("render needs at least one PRESET=PHRASE");
    }
    let all = presets::all();
    let ls: Vec<Layer> = layers.iter().map(|(pi, n)| Layer { values: &all[*pi].values, notes: n }).collect();
    let t0 = Instant::now();
    let samples = demo::render(&ls, bpm, slow, rate as f32, tail);
    let took = t0.elapsed().as_secs_f32();
    plaza::wav::write_file(&out, rate, 2, &samples).unwrap_or_else(|e| die(e));
    let secs = samples.len() as f32 / 2.0 / rate as f32;
    let peak = samples.iter().fold(0f32, |m, s| m.max(s.abs()));
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    println!(
        "{}: {:.1} s, peak {:.1} dBFS, RMS {:.1} dBFS, rendered in {:.2} s ({:.0}x real time)",
        out.display(),
        secs,
        20.0 * peak.log10(),
        20.0 * rms.log10(),
        took,
        secs / took
    );
}

fn bench() {
    let sr = 48_000.0;
    let secs = 20.0;
    println!("Worst case: 8 voices held, every layer on (saw+detune, pulse, sub, noise, FM), all effects.");
    let mut e = Engine::new(sr);
    let mut v = plaza::params::defaults();
    use plaza::params::P::*;
    for (p, x) in [(Saw, 1.0), (Detune, 10.0), (Pulse, 1.0), (Pwm, 0.5), (Sub, 1.0), (Noise, 0.1), (FmLevel, 0.5),
        (Chorus, 3.0), (RevMix, 0.5), (Wow, 0.5), (Flutter, 0.5), (Saturation, 0.5), (Age, 0.3), (Hiss, 0.3), (Crush, 0.3),
        (Glide, 0.2), (Vibrato, 0.3)]
    {
        v[p as usize] = x;
    }
    e.load_now(&v);
    for n in [48, 52, 55, 59, 62, 64, 67, 71] {
        e.handle(Event::NoteOn { note: n, velocity: 0.8 });
    }
    let mut buf = vec![0.0f32; 512 * 2];
    let blocks = (secs * sr / 512.0) as usize;
    let mut worst = Duration::ZERO;
    let t0 = Instant::now();
    for _ in 0..blocks {
        let t = Instant::now();
        e.render(&mut buf);
        worst = worst.max(t.elapsed());
    }
    let took = t0.elapsed().as_secs_f32();
    let budget = 512.0 / sr;
    println!(
        "{secs} s in {took:.3} s: {:.0}x real time, average {:.1}% of one core, worst 512-frame block {:.2} ms of {:.2} ms",
        secs / took,
        100.0 * took / secs,
        worst.as_secs_f32() * 1000.0,
        budget * 1000.0
    );
}

/// Opens the real device, plays a chord through the full engine with the
/// output forced to silence, and reports callback timing. Safe to run with
/// speakers on: every sample written is 0.0.
fn probe(secs: f32) {
    use std::sync::atomic::Ordering::Relaxed;
    let a = audio::start(true).unwrap_or_else(|e| die(format!("audio: {e}")));
    println!("Host {} · device '{}' · {} Hz · output MUTED", a.host, a.device, a.sample_rate);
    let mut v = presets::all()[0].values;
    v[plaza::params::P::RevMix as usize] = 0.5;
    audio::send(&a.events, Event::Load(v));
    for n in [48, 55, 60, 64, 67, 71, 74, 79] {
        audio::send(&a.events, Event::NoteOn { note: n, velocity: 0.8 });
    }
    let t0 = Instant::now();
    let mut worst = 0f32;
    let mut peak = 0f32;
    while t0.elapsed().as_secs_f32() < secs {
        std::thread::sleep(Duration::from_millis(250));
        worst = worst.max(f32::from_bits(a.shared.load.swap(0, Relaxed)));
        peak = peak.max(f32::from_bits(a.shared.peak.swap(0, Relaxed)));
    }
    let cb = a.shared.callbacks.load(Relaxed);
    let frames = a.shared.frames.load(Relaxed);
    println!(
        "{cb} callbacks of {frames} frames ({:.1} ms) in {secs} s · worst callback {:.1}% of its budget · {} voices · engine peak {:.1} dBFS (not played) · xruns reported: {}",
        frames as f32 * 1000.0 / a.sample_rate as f32,
        worst * 100.0,
        a.shared.voices.load(Relaxed),
        20.0 * peak.max(1e-9).log10(),
        a.shared.xruns.load(Relaxed)
    );
}

//! Window glue: knob models, keyboard, presets, recording, status.

use crate::audio::{self, send, Audio, Events};
use crate::midi::{self, MidiIn, NoteBits};
use crate::{AppWindow, KnobData};
use plaza::engine::Event;
use plaza::keys::{self, semitone};
use plaza::params::{self, Values, P};
use plaza::presets::{self, Preset};
use plaza::scope;
use plaza::wav::WavWriter;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use std::time::{Duration, Instant};

type Section = (&'static [P], fn(&AppWindow, ModelRc<KnobData>));

const SECTIONS: &[Section] = &[
    (&[P::Saw, P::Pulse, P::PulseWidth, P::Pwm, P::Sub, P::Noise, P::Detune, P::Glide], AppWindow::set_osc),
    (&[P::FmLevel, P::FmRatio, P::FmIndex, P::FmDecay], AppWindow::set_fm),
    (&[P::Cutoff, P::Resonance, P::FilterEnv, P::FilterLfo, P::KeyTrack, P::FilterVel], AppWindow::set_filter),
    (&[P::FAttack, P::FDecay, P::FSustain, P::FRelease], AppWindow::set_fenv),
    (&[P::AAttack, P::ADecay, P::ASustain, P::ARelease, P::AmpVel], AppWindow::set_amp),
    (&[P::LfoRate, P::LfoDelay, P::Vibrato], AppWindow::set_lfo),
    (&[P::RevDecay, P::RevDamp, P::RevPre, P::RevMix], AppWindow::set_reverb),
    (&[P::Wow, P::Flutter, P::Saturation, P::Age, P::Hiss, P::Crush], AppWindow::set_tape),
    (&[P::Volume, P::SlowRate], AppWindow::set_master),
];

struct State {
    values: Values,
    presets: Vec<Preset>,
    preset: usize,
    octave: i32,
    slow: bool,
    /// Key character -> note it started, so octave changes mid-note can't strand notes.
    typed: HashMap<String, u8>,
    mouse: HashSet<u8>,
    models: Vec<(Rc<VecModel<KnobData>>, HashMap<usize, usize>)>,
    events: Option<Events>,
    recorder: Option<(WavWriter, PathBuf, Instant)>,
    last_rec: Option<PathBuf>,
    midi: Option<MidiIn>,
    midi_scan: Instant,
    midi_note: Arc<NoteBits>,
    midi_error: Option<String>,
}

impl State {
    fn send(&self, e: Event) {
        if let Some(ev) = &self.events {
            send(ev, e);
        }
    }

    fn knob(p: P, v: f32) -> KnobData {
        KnobData {
            pid: p as i32,
            label: p.def().label.into(),
            value: params::to_norm(p, v),
            display: params::display(p, v).into(),
        }
    }

    fn set_param(&mut self, p: P, v: f32) {
        self.values[p as usize] = v;
        self.send(Event::Param(p, v));
        for (model, rows) in &self.models {
            if let Some(&row) = rows.get(&(p as usize)) {
                model.set_row_data(row, Self::knob(p, v));
            }
        }
    }

    fn refresh_all(&self) {
        for (model, rows) in &self.models {
            for (&pi, &row) in rows {
                let p = P::from_index(pi).unwrap();
                model.set_row_data(row, Self::knob(p, self.values[pi]));
            }
        }
    }
}

fn music_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    home.join("Music").join("Plaza")
}

fn timestamp() -> String {
    unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            tm.tm_year + 1900,
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min,
            tm.tm_sec
        )
    }
}

pub fn run(start_preset: Option<usize>, muted: bool) -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let audio: Result<Audio, String> = audio::start(muted);
    let (events, rec_cons, sample_rate, audio_desc, shared, scope) = match audio {
        Ok(a) => {
            let desc = format!("{} · {} · {:.1} kHz", a.host, a.device, a.sample_rate as f32 / 1000.0);
            let a = Box::leak(Box::new(a));
            (Some(a.events.clone()), a.rec.take(), a.sample_rate, desc, Some(a.shared.clone()), Some(a.scope.clone()))
        }
        Err(e) => (None, None, 48_000, format!("No audio output: {e}"), None, None),
    };

    let presets = presets::all();
    let preset = start_preset.unwrap_or(0).min(presets.len() - 1);
    let mut models = Vec::new();
    for (ids, setter) in SECTIONS {
        let model = Rc::new(VecModel::from(ids.iter().map(|&p| State::knob(p, p.def().default)).collect::<Vec<_>>()));
        let rows = ids.iter().enumerate().map(|(row, &p)| (p as usize, row)).collect();
        setter(&ui, model.clone().into());
        models.push((model, rows));
    }
    let names: Vec<SharedString> = presets.iter().map(|p| p.name.into()).collect();
    ui.set_preset_names(ModelRc::new(VecModel::from(names)));

    let state = Rc::new(RefCell::new(State {
        values: params::defaults(),
        presets,
        preset,
        octave: 4,
        slow: false,
        typed: HashMap::new(),
        mouse: HashSet::new(),
        models,
        events,
        recorder: None,
        last_rec: None,
        midi: None,
        midi_scan: Instant::now() - Duration::from_secs(10),
        midi_note: Arc::new(NoteBits::default()),
        midi_error: None,
    }));
    let held_model = Rc::new(VecModel::from(vec![false; 25]));
    ui.set_held(held_model.clone().into());

    let load_preset = {
        let ui = ui.as_weak();
        let state = state.clone();
        move |i: usize| {
            let ui = ui.unwrap();
            let mut s = state.borrow_mut();
            let i = i % s.presets.len();
            s.preset = i;
            // The slow-rate knob belongs to the player, not the preset.
            let slow_rate = s.values[P::SlowRate as usize];
            let mut v = s.presets[i].values;
            v[P::SlowRate as usize] = slow_rate;
            s.values = v;
            s.send(Event::Load(v));
            s.refresh_all();
            ui.set_preset_index(i as i32);
            ui.set_preset_name(s.presets[i].name.into());
            ui.set_preset_blurb(s.presets[i].blurb.into());
            ui.set_chorus_mode(v[P::Chorus as usize] as i32);
            ui.set_slow_label(params::display(P::SlowRate, slow_rate).into());
        }
    };
    load_preset(preset);

    {
        let state = state.clone();
        let ui_w = ui.as_weak();
        ui.on_knob(move |pid, norm| {
            let Some(p) = P::from_index(pid as usize) else { return };
            let v = params::from_norm(p, norm);
            state.borrow_mut().set_param(p, v);
            if p == P::SlowRate {
                ui_w.unwrap().set_slow_label(params::display(p, v).into());
            }
        });
    }
    {
        let state = state.clone();
        let ui_w = ui.as_weak();
        ui.on_knob_reset(move |pid| {
            let Some(p) = P::from_index(pid as usize) else { return };
            let mut s = state.borrow_mut();
            let v = s.presets[s.preset].values[p as usize];
            s.set_param(p, v);
            if p == P::SlowRate {
                ui_w.unwrap().set_slow_label(params::display(p, v).into());
            }
        });
    }
    {
        let state = state.clone();
        let ui_w = ui.as_weak();
        ui.on_chorus(move |m| {
            state.borrow_mut().set_param(P::Chorus, m as f32);
            ui_w.unwrap().set_chorus_mode(m);
        });
    }
    {
        let state = state.clone();
        let lp = load_preset.clone();
        ui.on_preset_step(move |d| {
            let (cur, n) = {
                let s = state.borrow();
                (s.preset as i32, s.presets.len() as i32)
            };
            lp((cur + d).rem_euclid(n) as usize);
        });
    }
    {
        let lp = load_preset.clone();
        ui.on_preset_pick(move |i| lp(i as usize));
    }
    {
        let state = state.clone();
        let ui_w = ui.as_weak();
        ui.on_toggle_slow(move || {
            let mut s = state.borrow_mut();
            s.slow = !s.slow;
            let on = s.slow;
            s.send(Event::Slow(on));
            ui_w.unwrap().set_slow(on);
        });
    }
    {
        let state = state.clone();
        let ui_w = ui.as_weak();
        let shared = shared.clone();
        ui.on_toggle_rec(move || {
            let ui = ui_w.unwrap();
            let Some(shared) = &shared else {
                ui.set_status_note("Can't record: no audio output".into());
                return;
            };
            let mut s = state.borrow_mut();
            if s.recorder.is_some() {
                shared.recording.store(false, Relaxed);
                // The writer is finished by the timer once the queue drains.
                ui.set_recording(false);
            } else {
                let dir = music_dir();
                let path = dir.join(format!("plaza-{}.wav", timestamp()));
                match std::fs::create_dir_all(&dir).and_then(|_| WavWriter::create(&path, sample_rate, 2)) {
                    Ok(w) => {
                        s.recorder = Some((w, path, Instant::now()));
                        shared.recording.store(true, Relaxed);
                        ui.set_recording(true);
                        ui.set_rec_label("0:00".into());
                    }
                    Err(e) => ui.set_status_note(format!("Can't record: {e}").into()),
                }
            }
        });
    }
    {
        ui.on_open_source(move || {
            let _ = std::process::Command::new("xdg-open").arg("https://github.com/nunyabiz765/plaza").spawn();
        });
    }
    {
        ui.on_open_recordings(move || {
            let dir = music_dir();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
        });
    }
    {
        let state = state.clone();
        let ui_w = ui.as_weak();
        ui.on_octave_step(move |d| {
            let mut s = state.borrow_mut();
            s.octave = (s.octave + d).clamp(keys::MIN_OCTAVE, keys::MAX_OCTAVE);
            let ui = ui_w.unwrap();
            ui.set_octave(s.octave);
            ui.set_base_name(keys::note_name(keys::base_note(s.octave)).into());
        });
    }
    {
        let state = state.clone();
        ui.on_key_down(move |text, _repeat| {
            let mut chars = text.chars();
            let (Some(c), None) = (chars.next(), chars.next()) else { return false };
            let Some(semi) = semitone(c) else { return false };
            let key = c.to_ascii_lowercase().to_string();
            let mut s = state.borrow_mut();
            if s.typed.contains_key(&key) {
                return true; // auto-repeat
            }
            let note = keys::base_note(s.octave) + semi;
            s.typed.insert(key, note);
            s.send(Event::NoteOn { note, velocity: 0.8 });
            true
        });
    }
    {
        let state = state.clone();
        ui.on_key_up(move |text| {
            let key = text.to_lowercase();
            let mut s = state.borrow_mut();
            match s.typed.remove(&key) {
                Some(note) => {
                    if !s.typed.values().any(|&n| n == note) && !s.mouse.contains(&note) {
                        s.send(Event::NoteOff { note });
                    }
                    true
                }
                None => semitone(text.chars().next().unwrap_or(' ')).is_some(),
            }
        });
    }
    {
        let state = state.clone();
        ui.on_mouse_note(move |semi, on| {
            let mut s = state.borrow_mut();
            let note = keys::base_note(s.octave) + semi as u8;
            if on {
                s.mouse.insert(note);
                s.send(Event::NoteOn { note, velocity: 0.8 });
            } else {
                s.mouse.retain(|&n| n != note);
                if !s.typed.values().any(|&n| n == note) {
                    s.send(Event::NoteOff { note });
                }
            }
        });
    }
    {
        let state = state.clone();
        ui.on_panic(move || {
            let mut s = state.borrow_mut();
            s.typed.clear();
            s.mouse.clear();
            s.send(Event::AllNotesOff);
        });
    }

    // Testing aid for screenshots: PLAZA_HOLD="60,64,67" holds notes at startup.
    if let Ok(list) = std::env::var("PLAZA_HOLD") {
        let mut s = state.borrow_mut();
        for n in list.split(',').filter_map(|x| x.trim().parse::<u8>().ok()) {
            s.mouse.insert(n);
            s.send(Event::NoteOn { note: n, velocity: 0.8 });
        }
    }

    ui.set_octave(4);
    ui.set_base_name(keys::note_name(keys::base_note(4)).into());
    ui.set_status_audio(audio_desc.clone().into());

    // 30 fps: scope, meters, recording drain, held keys, MIDI hot-plug.
    let timer = slint::Timer::default();
    {
        let ui_w = ui.as_weak();
        let state = state.clone();
        let mut rec_cons = rec_cons;
        let mut level = 0f32;
        let mut load_shown = 0f32;
        let mut last_status = Instant::now() - Duration::from_secs(1);
        let mut drain = vec![0.0f32; 0];
        timer.start(slint::TimerMode::Repeated, Duration::from_millis(33), move || {
            let ui = ui_w.unwrap();
            let mut s = state.borrow_mut();

            if let (Some(scope), Some(shared)) = (&scope, &shared) {
                let raw = scope.latest(2400);
                let dec: Vec<f32> = raw.iter().step_by(2).copied().collect();
                ui.set_scope_path(scope::trace(&dec, 400, 600.0, 120.0).into());
                let peak = f32::from_bits(shared.peak.swap(0, Relaxed));
                level = peak.max(level * 0.85);
                ui.set_level(level);
                let load = f32::from_bits(shared.load.swap(0, Relaxed));
                load_shown = load.max(load_shown * 0.97);

                // Recording: drain the queue to disk; finish once stopped and empty.
                if let Some(cons) = &mut rec_cons {
                    let n = cons.slots();
                    if n > 0 {
                        drain.clear();
                        if let Ok(chunk) = cons.read_chunk(n) {
                            let (a, b) = chunk.as_slices();
                            drain.extend_from_slice(a);
                            drain.extend_from_slice(b);
                            chunk.commit_all();
                        }
                        if let Some((w, _, _)) = &mut s.recorder {
                            if let Err(e) = w.write(&drain) {
                                ui.set_status_note(format!("Recording error: {e}").into());
                            }
                        }
                    }
                    let stopped = !shared.recording.load(Relaxed);
                    if stopped && cons.slots() == 0 {
                        if let Some((w, path, _)) = s.recorder.take() {
                            let secs = w.frames() as f32 / sample_rate as f32;
                            let msg = match w.finish() {
                                Ok(()) => format!("Saved {:.1} s to {}", secs, path.display()),
                                Err(e) => format!("Recording error: {e}"),
                            };
                            ui.set_status_note(msg.into());
                            s.last_rec = Some(path);
                        }
                    }
                }
                if let Some((_, _, t0)) = &s.recorder {
                    let t = t0.elapsed().as_secs();
                    ui.set_rec_label(format!("{}:{:02}", t / 60, t % 60).into());
                }

                if last_status.elapsed() > Duration::from_millis(500) {
                    last_status = Instant::now();
                    let frames = shared.frames.load(Relaxed);
                    let xruns = shared.xruns.load(Relaxed);
                    ui.set_status_audio(
                        format!(
                            "{audio_desc} · {} frames · DSP {:.0}% · {} voices{}",
                            frames,
                            load_shown * 100.0,
                            shared.voices.load(Relaxed),
                            if xruns > 0 { format!(" · {xruns} xruns") } else { String::new() }
                        )
                        .into(),
                    );
                }
            }

            // MIDI hot-plug.
            if s.midi_scan.elapsed() > Duration::from_secs(3) {
                s.midi_scan = Instant::now();
                if let Some(m) = &s.midi {
                    if !midi::still_present(&m.port) {
                        s.midi = None;
                        s.send(Event::AllNotesOff);
                    }
                }
                if s.midi.is_none() {
                    match midi::connect(s.events.clone(), s.midi_note.clone()) {
                        Ok(m) => {
                            s.midi = m;
                            s.midi_error = None;
                        }
                        Err(e) => s.midi_error = Some(e),
                    }
                }
                let text = match (&s.midi, &s.midi_error) {
                    (Some(m), _) => format!("MIDI: {}", m.port),
                    (None, Some(e)) => format!("MIDI unavailable: {e}"),
                    (None, None) => "MIDI: no device".into(),
                };
                ui.set_status_midi(text.into());
            }

            // Light the keys: typed, clicked, or played over MIDI.
            let base = keys::base_note(s.octave);
            let typed: HashSet<u8> = s.typed.values().copied().collect();
            for i in 0..25usize {
                let n = base + i as u8;
                let on = typed.contains(&n) || s.mouse.contains(&n) || s.midi_note.get(n);
                if held_model.row_data(i) != Some(on) {
                    held_model.set_row_data(i, on);
                }
            }
        });
    }

    ui.run()
}

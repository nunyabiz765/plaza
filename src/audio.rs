//! Real-time output through cpal (native PipeWire, falling back to PulseAudio
//! or ALSA). The callback owns the engine; nothing in it locks or allocates.

use plaza::engine::{Engine, Event};
use plaza::scope::Scope;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub type Events = Arc<Mutex<Producer<Event>>>;

#[derive(Default)]
pub struct Shared {
    pub recording: AtomicBool,
    pub muted: AtomicBool,
    /// f32 bits: output peak since the UI last looked.
    pub peak: AtomicU32,
    /// f32 bits: worst callback time / buffer time since the UI last looked.
    pub load: AtomicU32,
    pub callbacks: AtomicU64,
    pub frames: AtomicU32,
    pub voices: AtomicU32,
    pub xruns: AtomicU64,
    pub rec_dropped: AtomicU64,
}

pub struct Audio {
    pub _stream: cpal::Stream,
    pub sample_rate: u32,
    pub host: String,
    pub device: String,
    pub shared: Arc<Shared>,
    pub scope: Arc<Scope>,
    pub events: Events,
    pub rec: Option<Consumer<f32>>,
}

const SCRATCH_FRAMES: usize = 4096;

pub fn start(muted: bool) -> Result<Audio, String> {
    let host = cpal::default_host();
    let host_name = format!("{:?}", host.id());
    let device = host.default_output_device().ok_or("no output device")?;
    let device_name = device
        .description()
        .map(|d| d.to_string())
        .unwrap_or_else(|_| "default output".into());
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let format = supported.sample_format();
    let mut config: cpal::StreamConfig = supported.into();
    // Ask for a short buffer so the keys feel immediate (256 frames = 5.3 ms at
    // 48 kHz); PLAZA_BUFFER overrides, 0 means the server's default.
    let want: u32 = std::env::var("PLAZA_BUFFER").ok().and_then(|v| v.parse().ok()).unwrap_or(256);
    if want > 0 {
        config.buffer_size = cpal::BufferSize::Fixed(want);
    }
    let sr = config.sample_rate;

    let shared = Arc::new(Shared::default());
    shared.muted.store(muted, Relaxed);
    let scope = Arc::new(Scope::new(8192));
    let attempt = |config: &cpal::StreamConfig| {
        let (prod, cons) = RingBuffer::<Event>::new(2048);
        let (rec_prod, rec_cons) = RingBuffer::<f32>::new(sr as usize * 2 * 6);
        let stream = open(&device, config, format, cons, rec_prod, shared.clone(), scope.clone())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok::<_, String>((stream, prod, rec_cons))
    };
    let (stream, prod, rec_cons) = match attempt(&config) {
        Ok(s) => s,
        Err(e) if config.buffer_size != cpal::BufferSize::Default => {
            eprintln!("plaza: {e}; retrying with the default buffer size");
            config.buffer_size = cpal::BufferSize::Default;
            attempt(&config)?
        }
        Err(e) => return Err(e),
    };
    Ok(Audio {
        _stream: stream,
        sample_rate: sr,
        host: host_name,
        device: device_name,
        shared,
        scope,
        events: Arc::new(Mutex::new(prod)),
        rec: Some(rec_cons),
    })
}

fn open(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    format: SampleFormat,
    cons: Consumer<Event>,
    rec_prod: Producer<f32>,
    shared: Arc<Shared>,
    scope: Arc<Scope>,
) -> Result<cpal::Stream, String> {
    match format {
        SampleFormat::F32 => build::<f32>(device, config, cons, rec_prod, shared, scope),
        SampleFormat::I16 => build::<i16>(device, config, cons, rec_prod, shared, scope),
        SampleFormat::I32 => build::<i32>(device, config, cons, rec_prod, shared, scope),
        other => Err(format!("unsupported sample format {other}")),
    }
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut events: Consumer<Event>,
    mut rec: Producer<f32>,
    shared: Arc<Shared>,
    scope: Arc<Scope>,
) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let sr = config.sample_rate as f32;
    let channels = config.channels as usize;
    let mut engine = Engine::new(sr);
    let mut scratch = vec![0.0f32; SCRATCH_FRAMES * 2];
    let mut denormals_off = false;
    let err_shared = shared.clone();

    let callback = move |out: &mut [T], _: &cpal::OutputCallbackInfo| {
        if !denormals_off {
            flush_denormals();
            denormals_off = true;
        }
        let t0 = Instant::now();
        while let Ok(e) = events.pop() {
            engine.handle(e);
        }
        let frames = out.len() / channels.max(1);
        let muted = shared.muted.load(Relaxed);
        let recording = shared.recording.load(Relaxed);
        let mut peak = 0f32;
        let mut done = 0;
        while done < frames {
            let n = (frames - done).min(SCRATCH_FRAMES);
            let buf = &mut scratch[..n * 2];
            engine.render(buf);
            for f in 0..n {
                let (l, r) = (buf[2 * f], buf[2 * f + 1]);
                peak = peak.max(l.abs()).max(r.abs());
                scope.push(0.5 * (l + r));
                let (l, r) = if muted { (0.0, 0.0) } else { (l, r) };
                let frame = &mut out[(done + f) * channels..(done + f + 1) * channels];
                match channels {
                    1 => frame[0] = T::from_sample(0.5 * (l + r)),
                    _ => {
                        frame[0] = T::from_sample(l);
                        frame[1] = T::from_sample(r);
                        for s in &mut frame[2..] {
                            *s = T::from_sample(0.0);
                        }
                    }
                }
            }
            if recording {
                if rec.slots() >= buf.len() {
                    for &s in buf.iter() {
                        let _ = rec.push(s);
                    }
                } else {
                    shared.rec_dropped.fetch_add(n as u64, Relaxed);
                }
            }
            done += n;
        }
        let budget = frames as f32 / sr;
        let load = t0.elapsed().as_secs_f32() / budget.max(1e-6);
        if load > f32::from_bits(shared.load.load(Relaxed)) {
            shared.load.store(load.to_bits(), Relaxed);
        }
        if peak > f32::from_bits(shared.peak.load(Relaxed)) {
            shared.peak.store(peak.to_bits(), Relaxed);
        }
        shared.frames.store(frames as u32, Relaxed);
        shared.voices.store(engine.active_voices() as u32, Relaxed);
        shared.callbacks.fetch_add(1, Relaxed);
    };
    let on_error = move |e: cpal::Error| {
        if e.kind() == cpal::ErrorKind::Xrun {
            err_shared.xruns.fetch_add(1, Relaxed);
        } else {
            eprintln!("plaza: audio: {e}");
        }
    };
    device.build_output_stream(config.clone(), callback, on_error, None).map_err(|e| e.to_string())
}

/// Sets flush-to-zero / denormals-are-zero for this thread: decaying reverb
/// tails otherwise wander into denormal territory and spike the CPU.
#[allow(deprecated)]
fn flush_denormals() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use std::arch::x86_64::{_mm_getcsr, _mm_setcsr};
        _mm_setcsr(_mm_getcsr() | 0x8040);
    }
}

pub fn send(events: &Events, e: Event) {
    if let Ok(mut p) = events.lock() {
        let _ = p.push(e);
    }
}

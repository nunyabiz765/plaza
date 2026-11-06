//! Plaza: an 80s polysynth for vaporwave.
//!
//! The library is the whole sound engine and is UI-free, so it can be tested
//! and rendered offline. `main.rs` adds the window, audio device and MIDI.

pub mod demo;
pub mod dsp;
pub mod engine;
pub mod fx;
pub mod keys;
pub mod params;
pub mod presets;
pub mod scope;
pub mod voice;
pub mod wav;

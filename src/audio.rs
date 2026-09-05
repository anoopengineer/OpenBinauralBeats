//! Real-time binaural tone generator.
//!
//! The UI thread only writes atomics; the audio callback reads them once per
//! buffer and does all smoothing itself, so there are no locks or allocations
//! on the real-time path.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};

/// Peak amplitude of each tone at full volume.
const TONE_GAIN: f32 = 0.5;
/// Time constant for volume changes, in seconds.
const PARAM_SMOOTHING_SECS: f32 = 0.03;

struct AtomicF32(AtomicU32);

impl AtomicF32 {
    fn new(v: f32) -> Self {
        Self(AtomicU32::new(v.to_bits()))
    }
    fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
    fn set(&self, v: f32) {
        self.0.store(v.to_bits(), Ordering::Relaxed);
    }
}

/// State shared between the UI and the audio callback.
struct Shared {
    carrier: AtomicF32,
    beat: AtomicF32,
    glide_secs: AtomicF32,
    /// Bumped after carrier/beat/glide are written so the callback picks up a new target.
    tone_generation: AtomicU32,
    volume: AtomicF32,
    playing: AtomicBool,
    /// Fade-out time used when `playing` goes false.
    release_secs: AtomicF32,
    /// Set by the callback once output has fully faded to silence.
    silent: AtomicBool,
}

pub struct Engine {
    shared: Arc<Shared>,
    stream: Option<cpal::Stream>,
    stream_running: bool,
    pub error: Option<String>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared {
                carrier: AtomicF32::new(200.0),
                beat: AtomicF32::new(10.0),
                glide_secs: AtomicF32::new(0.0),
                tone_generation: AtomicU32::new(0),
                volume: AtomicF32::new(0.5),
                playing: AtomicBool::new(false),
                release_secs: AtomicF32::new(0.08),
                silent: AtomicBool::new(true),
            }),
            stream: None,
            stream_running: false,
            error: None,
        }
    }

    pub fn is_playing(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed)
    }

    /// Set the target tone. When audible, the callback glides there over `glide_secs`.
    pub fn set_tone(&self, carrier: f32, beat: f32, glide_secs: f32) {
        self.shared.carrier.set(carrier);
        self.shared.beat.set(beat);
        self.shared.glide_secs.set(glide_secs.max(0.0));
        self.shared.tone_generation.fetch_add(1, Ordering::Release);
    }

    pub fn set_volume(&self, volume: f32) {
        self.shared.volume.set(volume.clamp(0.0, 1.0));
    }

    pub fn play(&mut self) {
        if let Err(e) = self.ensure_stream() {
            self.error = Some(e);
            return;
        }
        self.shared.silent.store(false, Ordering::Relaxed);
        self.shared.playing.store(true, Ordering::Relaxed);
        if !self.stream_running
            && let Some(stream) = &self.stream
        {
            match stream.play() {
                Ok(()) => self.stream_running = true,
                Err(e) => self.error = Some(format!("Could not start audio: {e}")),
            }
        }
    }

    /// Fade out over `release_secs`; the stream is paused once silent (see [`Self::tick`]).
    pub fn stop(&mut self, release_secs: f32) {
        self.shared.release_secs.set(release_secs.max(0.01));
        self.shared.playing.store(false, Ordering::Relaxed);
    }

    /// Housekeeping from the UI thread. Returns true while it needs to be called again soon.
    pub fn tick(&mut self) -> bool {
        if self.stream_running && !self.is_playing() {
            if self.shared.silent.load(Ordering::Relaxed) {
                // Pausing releases the audio thread entirely: zero CPU while idle.
                if let Some(stream) = &self.stream {
                    let _ = stream.pause();
                }
                self.stream_running = false;
                return false;
            }
            return true;
        }
        false
    }

    fn ensure_stream(&mut self) -> Result<(), String> {
        if self.stream.is_some() {
            return Ok(());
        }
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "No audio output device found".to_string())?;
        let supported = device
            .default_output_config()
            .map_err(|e| format!("Audio device unavailable: {e}"))?;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();

        let stream = match format {
            SampleFormat::F32 => self.build::<f32>(&device, config),
            SampleFormat::F64 => self.build::<f64>(&device, config),
            SampleFormat::I16 => self.build::<i16>(&device, config),
            SampleFormat::I32 => self.build::<i32>(&device, config),
            SampleFormat::U16 => self.build::<u16>(&device, config),
            SampleFormat::U8 => self.build::<u8>(&device, config),
            other => return Err(format!("Unsupported sample format {other}")),
        }
        .map_err(|e| format!("Could not open audio stream: {e}"))?;

        self.stream = Some(stream);
        self.stream_running = false;
        self.error = None;
        Ok(())
    }

    fn build<T>(
        &self,
        device: &cpal::Device,
        config: cpal::StreamConfig,
    ) -> Result<cpal::Stream, cpal::Error>
    where
        T: SizedSample + FromSample<f32>,
    {
        let channels = config.channels as usize;
        let mut voice = Voice::new(config.sample_rate as f32, Arc::clone(&self.shared));

        device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| voice.render(data, channels),
            |err: cpal::Error| eprintln!("audio stream error: {err}"),
            None,
        )
    }
}

/// Audio-thread state. Owned by the callback closure.
struct Voice {
    shared: Arc<Shared>,
    sample_rate: f32,
    phase_l: f32,
    phase_r: f32,
    carrier: f32,
    beat: f32,
    carrier_step: f32,
    beat_step: f32,
    glide_remaining: u32,
    seen_generation: u32,
    initialized: bool,
    /// Smoothed master gain (volume x play envelope).
    gain: f32,
}

impl Voice {
    fn new(sample_rate: f32, shared: Arc<Shared>) -> Self {
        Self {
            shared,
            sample_rate,
            phase_l: 0.0,
            phase_r: 0.0,
            carrier: 0.0,
            beat: 0.0,
            carrier_step: 0.0,
            beat_step: 0.0,
            glide_remaining: 0,
            seen_generation: u32::MAX,
            initialized: false,
            gain: 0.0,
        }
    }

    fn smoothing(&self, secs: f32) -> f32 {
        1.0 - (-1.0 / (secs * self.sample_rate)).exp()
    }

    fn update_targets(&mut self) {
        let generation = self.shared.tone_generation.load(Ordering::Acquire);
        if generation == self.seen_generation {
            return;
        }
        self.seen_generation = generation;
        let carrier = self.shared.carrier.get();
        let beat = self.shared.beat.get();
        let glide = self.shared.glide_secs.get();
        let samples = (glide * self.sample_rate) as u32;

        // Jump straight there if nothing is audible yet; otherwise glide.
        if !self.initialized || self.gain < 1e-4 || samples == 0 {
            self.carrier = carrier;
            self.beat = beat;
            self.glide_remaining = 0;
            self.initialized = true;
        } else {
            self.carrier_step = (carrier - self.carrier) / samples as f32;
            self.beat_step = (beat - self.beat) / samples as f32;
            self.glide_remaining = samples;
        }
    }

    fn render<T: SizedSample + FromSample<f32>>(&mut self, out: &mut [T], channels: usize) {
        self.update_targets();

        let playing = self.shared.playing.load(Ordering::Relaxed);
        let volume = self.shared.volume.get();

        let (gain_target, gain_k) = if playing {
            (volume, self.smoothing(PARAM_SMOOTHING_SECS))
        } else {
            (0.0, self.smoothing(self.shared.release_secs.get() / 5.0))
        };

        if !playing && self.gain < 1e-5 {
            self.gain = 0.0;
            self.shared.silent.store(true, Ordering::Relaxed);
            out.fill(T::EQUILIBRIUM);
            return;
        }

        let inv_sr = 1.0 / self.sample_rate;
        let tau = std::f32::consts::TAU;

        for frame in out.chunks_mut(channels) {
            if self.glide_remaining > 0 {
                self.carrier += self.carrier_step;
                self.beat += self.beat_step;
                self.glide_remaining -= 1;
            }
            self.gain += (gain_target - self.gain) * gain_k;

            self.phase_l = (self.phase_l + self.carrier * inv_sr).fract();
            self.phase_r = (self.phase_r + (self.carrier + self.beat) * inv_sr).fract();

            let tone = self.gain * TONE_GAIN;
            let l = (self.phase_l * tau).sin() * tone;
            let r = (self.phase_r * tau).sin() * tone;

            let l = l.clamp(-1.0, 1.0);
            let r = r.clamp(-1.0, 1.0);
            match frame {
                [only] => *only = T::from_sample((l + r) * 0.5),
                [fl, fr, rest @ ..] => {
                    *fl = T::from_sample(l);
                    *fr = T::from_sample(r);
                    rest.fill(T::EQUILIBRIUM);
                }
                [] => {}
            }
        }
    }
}

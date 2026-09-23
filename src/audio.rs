//! Real-time binaural tone generator.
//!
//! The UI thread only writes atomics; the audio callback reads them once per
//! buffer and does all smoothing itself, so there are no locks or allocations
//! on the real-time path.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use eframe::egui;
use serde::{Deserialize, Serialize};

/// Peak amplitude of each tone at full volume. Leaves headroom for noise.
const TONE_GAIN: f32 = 0.5;
/// Peak amplitude of the noise bed at full volume and full noise level.
const NOISE_GAIN: f32 = 0.35;
/// Time constant for volume / noise-level changes, in seconds.
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Noise {
    Off,
    White,
    #[default]
    Pink,
    Brown,
}

impl Noise {
    pub const ALL: [Noise; 4] = [Noise::Off, Noise::White, Noise::Pink, Noise::Brown];

    pub fn label(self) -> &'static str {
        match self {
            Noise::Off => "Off",
            Noise::White => "White",
            Noise::Pink => "Pink",
            Noise::Brown => "Brown",
        }
    }

    fn from_u32(v: u32) -> Self {
        match v {
            1 => Noise::White,
            2 => Noise::Pink,
            3 => Noise::Brown,
            _ => Noise::Off,
        }
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
    noise_level: AtomicF32,
    noise: AtomicU32,
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
    needs_rebuild: Arc<AtomicBool>,
    repaint: egui::Context,
    pub error: Option<String>,
    pub device_name: Option<String>,
}

impl Engine {
    pub fn new(repaint: egui::Context) -> Self {
        Self {
            shared: Arc::new(Shared {
                carrier: AtomicF32::new(200.0),
                beat: AtomicF32::new(10.0),
                glide_secs: AtomicF32::new(0.0),
                tone_generation: AtomicU32::new(0),
                volume: AtomicF32::new(0.5),
                noise_level: AtomicF32::new(0.0),
                noise: AtomicU32::new(Noise::Off as u32),
                playing: AtomicBool::new(false),
                release_secs: AtomicF32::new(0.08),
                silent: AtomicBool::new(true),
            }),
            stream: None,
            stream_running: false,
            needs_rebuild: Arc::new(AtomicBool::new(false)),
            repaint,
            error: None,
            device_name: None,
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

    pub fn set_noise(&self, noise: Noise, level: f32) {
        self.shared.noise.store(noise as u32, Ordering::Relaxed);
        self.shared.noise_level.set(level.clamp(0.0, 1.0));
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
        if self.needs_rebuild.swap(false, Ordering::Relaxed) {
            let was_playing = self.is_playing();
            self.stream = None;
            self.stream_running = false;
            if was_playing {
                self.play();
            }
        }
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

        self.device_name = device.description().ok().map(|d| d.name().to_string());
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
        let needs_rebuild = Arc::clone(&self.needs_rebuild);
        let repaint = self.repaint.clone();

        device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| voice.render(data, channels),
            move |err: cpal::Error| {
                use cpal::ErrorKind::*;
                match err.kind() {
                    Xrun | RealtimeDenied => {}
                    _ => {
                        // Device unplugged / default output changed: reopen on the UI thread.
                        needs_rebuild.store(true, Ordering::Relaxed);
                        repaint.request_repaint();
                    }
                }
            },
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
    noise_gain: f32,
    noise: NoiseGen,
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
            noise_gain: 0.0,
            noise: NoiseGen::new(),
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
        let noise_kind = Noise::from_u32(self.shared.noise.load(Ordering::Relaxed));
        let noise_level = if noise_kind == Noise::Off {
            0.0
        } else {
            self.shared.noise_level.get()
        };

        let (gain_target, gain_k) = if playing {
            (volume, self.smoothing(PARAM_SMOOTHING_SECS))
        } else {
            (0.0, self.smoothing(self.shared.release_secs.get() / 5.0))
        };
        let noise_target = noise_level * if playing { 1.0 } else { 0.0 };
        let noise_k = gain_k;

        if !playing && self.gain < 1e-5 && self.noise_gain < 1e-5 {
            self.gain = 0.0;
            self.noise_gain = 0.0;
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
            self.noise_gain += (noise_target - self.noise_gain) * noise_k;

            self.phase_l = (self.phase_l + self.carrier * inv_sr).fract();
            self.phase_r = (self.phase_r + (self.carrier + self.beat) * inv_sr).fract();

            let tone = self.gain * TONE_GAIN;
            let mut l = (self.phase_l * tau).sin() * tone;
            let mut r = (self.phase_r * tau).sin() * tone;

            if self.noise_gain > 1e-6 {
                let n = self.noise_gain * self.gain * NOISE_GAIN;
                let (nl, nr) = self.noise.next(noise_kind);
                l += nl * n;
                r += nr * n;
            }

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

/// Independent per-channel noise sources, normalized to roughly unit peak.
struct NoiseGen {
    rng: u32,
    pink: [[f32; 3]; 2],
    brown: [f32; 2],
}

impl NoiseGen {
    fn new() -> Self {
        Self {
            rng: 0x9E37_79B9,
            pink: [[0.0; 3]; 2],
            brown: [0.0; 2],
        }
    }

    fn white(&mut self) -> f32 {
        // xorshift32
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn channel(&mut self, kind: Noise, ch: usize) -> f32 {
        let w = self.white();
        match kind {
            Noise::Off => 0.0,
            Noise::White => w * 0.5,
            Noise::Pink => {
                // Paul Kellet's economy pink filter.
                let b = &mut self.pink[ch];
                b[0] = 0.99765 * b[0] + w * 0.099_046;
                b[1] = 0.963 * b[1] + w * 0.296_516_4;
                b[2] = 0.57 * b[2] + w * 1.052_691_3;
                (b[0] + b[1] + b[2] + w * 0.1848) * 0.2
            }
            Noise::Brown => {
                let b = &mut self.brown[ch];
                *b = (*b + 0.02 * w) / 1.02;
                *b * 3.5
            }
        }
    }

    fn next(&mut self, kind: Noise) -> (f32, f32) {
        (self.channel(kind, 0), self.channel(kind, 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn setup(carrier: f32, beat: f32) -> (Engine, Voice) {
        let engine = Engine::new(egui::Context::default());
        engine.set_tone(carrier, beat, 0.0);
        engine.set_volume(1.0);
        engine.set_noise(Noise::Off, 0.0);
        engine.shared.playing.store(true, Ordering::Relaxed);
        let voice = Voice::new(SR, Arc::clone(&engine.shared));
        (engine, voice)
    }

    fn render(voice: &mut Voice, seconds: f32) -> Vec<f32> {
        let mut out = vec![0.0f32; (seconds * SR) as usize * 2];
        for block in out.chunks_mut(512 * 2) {
            voice.render(block, 2);
        }
        out
    }

    /// Frequency estimate from rising zero crossings over the given interleaved channel.
    fn freq(buf: &[f32], ch: usize) -> f32 {
        let s: Vec<f32> = buf.iter().skip(ch).step_by(2).copied().collect();
        let crossings: Vec<usize> = (1..s.len())
            .filter(|&i| s[i - 1] < 0.0 && s[i] >= 0.0)
            .collect();
        let span = (crossings[crossings.len() - 1] - crossings[0]) as f32 / SR;
        (crossings.len() - 1) as f32 / span
    }

    #[test]
    fn left_is_carrier_right_is_carrier_plus_beat() {
        let (_e, mut v) = setup(200.0, 40.0);
        let out = render(&mut v, 1.0);
        assert!(
            (freq(&out, 0) - 200.0).abs() < 0.5,
            "left {}",
            freq(&out, 0)
        );
        assert!(
            (freq(&out, 1) - 240.0).abs() < 0.5,
            "right {}",
            freq(&out, 1)
        );
        assert!(out.iter().all(|x| x.abs() <= TONE_GAIN + 1e-3));
    }

    #[test]
    fn glide_is_continuous_and_reaches_target() {
        let (e, mut v) = setup(400.0, 10.0);
        render(&mut v, 0.5);
        e.set_tone(200.0, 2.0, 1.0);
        let out = render(&mut v, 1.5);
        // No sample-to-sample jump bigger than a 640 Hz sine at full scale could make.
        let max_step = std::f32::consts::TAU * 640.0 / SR * TONE_GAIN * 1.05;
        for ch in 0..2 {
            let s: Vec<f32> = out.iter().skip(ch).step_by(2).copied().collect();
            assert!(s.windows(2).all(|w| (w[1] - w[0]).abs() <= max_step));
        }
        let tail = &out[out.len() - (0.4 * SR) as usize * 2..];
        assert!((freq(tail, 0) - 200.0).abs() < 1.0);
        assert!((freq(tail, 1) - 202.0).abs() < 1.0);
    }

    #[test]
    fn stop_fades_to_silence_and_flags_it() {
        let (mut e, mut v) = setup(300.0, 6.0);
        render(&mut v, 0.2);
        e.stop(0.08);
        render(&mut v, 0.5);
        assert!(e.shared.silent.load(Ordering::Relaxed));
        assert!(render(&mut v, 0.05).iter().all(|&x| x == 0.0));
    }

    #[test]
    fn noise_stays_in_range() {
        for kind in [Noise::White, Noise::Pink, Noise::Brown] {
            let (e, mut v) = setup(200.0, 40.0);
            e.set_noise(kind, 1.0);
            let out = render(&mut v, 2.0);
            let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak <= 1.0 && peak > TONE_GAIN, "{kind:?} peak {peak}");
        }
    }
}

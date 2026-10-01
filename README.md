# OpenBinauralBeats

A small native desktop app that generates binaural beats in real time. Written in Rust with
[egui](https://github.com/emilk/egui) for the UI and [cpal](https://github.com/RustAudio/cpal) for audio.

- One self-contained binary (~6 MB). Links only OS frameworks; no Python, runtimes, or audio libraries to install.
- About 40 MB of memory and ~0% CPU when idle. The UI redraws only when you interact, and the audio
  stream pauses when you stop playback.
- Phase-continuous oscillators with smoothed parameter changes, so there are no clicks when you switch
  presets, change volume, or stop.

Use stereo headphones. Binaural beats only work when each ear gets its own tone.

## Features

- **Presets:** 40 Hz Focus (gamma), Focus (beta, 20 Hz), Relax (alpha, 10 Hz), Meditate (theta, 6 Hz),
  Deep Sleep (delta, 2 Hz).
- **Custom frequencies:** type any carrier (40-1500 Hz) and beat (0.5-100 Hz). Left ear gets the carrier,
  right ear gets carrier + beat.
- **Saved presets:** name and save any setting. Right-click a saved preset to delete it.
- **Smooth transitions:** switching presets while playing glides over the transition time (0-30 s).
- **Background noise:** white, pink, or brown, with adjustable level.
- **Session timer:** 15-90 minutes, ending with an 8-second fade-out.
- Settings and saved presets persist between launches.

### Keyboard

| Key   | Action             |
|-------|--------------------|
| Space | Play / stop        |
| 1-5   | Select a preset    |
| Enter | Apply custom input |

## Build and run

Requires a Rust toolchain ([rustup.rs](https://rustup.rs)).

```bash
cargo run --release
```

On Linux you also need ALSA dev headers to build (`sudo apt install libasound2-dev`).

## Package

### macOS

```bash
./scripts/package-macos.sh              # native architecture
./scripts/package-macos.sh --universal  # Apple Silicon + Intel
```

This writes `dist/OpenBinauralBeats.app` and `dist/OpenBinauralBeats-<version>.dmg`. The app has an ad-hoc
signature, which is enough to run it on your own machine. To distribute it to other people, sign it with a
Developer ID and notarize it.

### Windows / Linux

`cargo build --release` produces a single executable in `target/release/`. Release builds on Windows do not
open a console window.

## Development

```bash
cargo test                       # audio engine + input parsing tests
cargo clippy --all-targets
cargo run --example make_icon    # regenerate assets/icon-*.png
```

Layout:

- `src/audio.rs`: real-time engine. The UI writes atomics; the audio callback reads them once per buffer.
  There are no locks or allocations on the audio thread.
- `src/presets.rs`: built-in presets, frequency parsing and validation.
- `src/app.rs`: UI and persisted settings.

## About binaural beats

When each ear hears a slightly different frequency (say 200 Hz left, 240 Hz right), the brain perceives
a beat at the difference (40 Hz). Beat frequencies are commonly grouped into bands:

| Band  | Range      | Associated with                 |
|-------|------------|---------------------------------|
| Delta | 0.5-4 Hz   | Deep sleep                      |
| Theta | 4-8 Hz     | Meditation, creativity          |
| Alpha | 8-13 Hz    | Relaxation                      |
| Beta  | 13-30 Hz   | Focus, alertness                |
| Gamma | 30+ Hz     | Attention; 40 Hz is most studied |

The evidence for most of the claimed effects is limited and varies between people. This is a relaxation
and focus aid, not a medical treatment. Start at a low volume. Do not use it while driving. If you have
epilepsy or another neurological condition, talk to a doctor first.

## License

MIT

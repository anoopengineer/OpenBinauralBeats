# OpenBinauralBeats

**Website and downloads: [openbinauralbeats.com](https://openbinauralbeats.com/)**

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

## Releases and downloads

Downloads are built only by CI, never by hand:

- **`.github/workflows/ci.yml`** runs on every push and pull request. It runs `cargo fmt`, `clippy` and the tests
  on macOS, Windows and Linux, launches the app headless on Linux, and checks the website for broken links.
- **`.github/workflows/release.yml`** runs when a `v*` tag is pushed. It checks that the tag matches the
  `Cargo.toml` version, then builds, smoke-tests and publishes:

  | File | Platform |
  |---|---|
  | `OpenBinauralBeats-macos.dmg` | macOS 11+, universal (Apple Silicon + Intel), ad-hoc signed |
  | `OpenBinauralBeats-windows-x64.exe` | Windows 10/11 x64, with embedded icon and version info |
  | `OpenBinauralBeats-linux-x86_64.AppImage` | Most x86_64 distros |
  | `OpenBinauralBeats-linux-amd64.deb` | Ubuntu 22.04+, Debian 12+ |
  | `OpenBinauralBeats-linux-x86_64.tar.gz` | Plain binary, `.desktop` file and icon |
  | `SHA256SUMS.txt` | Checksums for everything above |

  Each file also gets a GitHub build attestation. The release is created as a draft and published only after
  every file is attached, so `releases/latest` never points at a partial release.

To publish a new version, follow [Creating a new release](#creating-a-new-release).

To build packages locally:

```bash
./scripts/package-macos.sh --universal   # dist/OpenBinauralBeats.app and .dmg
./scripts/package-linux.sh               # after cargo build --release; needs cargo-deb and appimagetool
scripts/smoke-test.sh target/release/open-binaural-beats
```

## Creating a new release

1. **Pick the version number.** Releases use `major.minor.patch`:
   - patch (`0.1.0` → `0.1.1`) for bug fixes,
   - minor (`0.1.1` → `0.2.0`) for new features,
   - major (`1.0.0`) for breaking changes or a big milestone.

   The last release is on the [releases page](https://github.com/anoopengineer/OpenBinauralBeats/releases), or run
   `git describe --tags --abbrev=0`.

2. **Make sure `main` is ready.** Commit and push everything you want in the release, and check that the latest
   [CI run](https://github.com/anoopengineer/OpenBinauralBeats/actions/workflows/ci.yml) on `main` is green.

   ```bash
   git checkout main && git pull
   ```

3. **Tag it.** This bumps the version in `Cargo.toml`, runs fmt, clippy, tests and the website checks, commits
   "Release vX.Y.Z" and creates the tag. It stops if the working tree isn't clean or the tag already exists.

   ```bash
   scripts/release.sh 0.2.0
   ```

4. **Push the commit and the tag.** Pushing the tag starts the release workflow.

   ```bash
   git push origin main v0.2.0
   ```

5. **Watch the build** on the [Release workflow](https://github.com/anoopengineer/OpenBinauralBeats/actions/workflows/release.yml)
   page. It takes about 15 to 25 minutes. Nothing is published unless every platform builds and passes its
   smoke test.

6. **Check the result.** The new version should be the latest on the releases page, and the site's download
   buttons pick it up automatically. Nothing on the website needs changing.

   ```bash
   curl -sIL https://openbinauralbeats.com/download/macos | grep -i -E "^HTTP|^location" | tail -2
   ```

**If the workflow fails before publishing**, nothing went public. Fix the problem on `main`, then delete the tag
and tag again:

```bash
git tag -d v0.2.0 && git push origin :refs/tags/v0.2.0   # remove the failed tag locally and on GitHub
# fix, commit and push to main, then:
scripts/release.sh 0.2.0 && git push origin main v0.2.0
```

**If a published release turns out to be broken**, don't rewrite it. Release the fix as the next patch version
(for example `0.2.1`). People may already have downloaded the broken one, and the checksums and attestations
must keep matching.

**Dry run:** on the Release workflow page, click **Run workflow** on `main`. It builds and smoke-tests every
package without publishing anything. Useful before a release with big packaging changes.

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

## Website

The site at [openbinauralbeats.com](https://openbinauralbeats.com) is static HTML in `website/`, deployed by
Netlify from `main` using `netlify.toml`. Preview it with `python3 -m http.server -d website` and check it with
`python3 scripts/check-site.py`. Download buttons link to `/download/*`, which Netlify redirects to the latest
GitHub release files, so the site never needs editing for a new release.

## License

MIT. See [LICENSE](LICENSE).

mod audio;
mod presets;

use std::time::Duration;

/// Temporary CLI driver: plays the first preset for ten seconds.
fn main() {
    let p = &presets::BUILTINS[0];
    let mut engine = audio::Engine::new();
    engine.set_tone(p.carrier, p.beat, 0.0);
    engine.play();
    if let Some(err) = &engine.error {
        eprintln!("{err}");
        return;
    }
    println!(
        "Playing {} ({} Hz beat on a {} Hz carrier)",
        p.name, p.beat, p.carrier
    );
    std::thread::sleep(Duration::from_secs(10));
    engine.stop(0.08);
    std::thread::sleep(Duration::from_millis(300));
}

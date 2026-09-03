mod presets;

fn main() {
    for p in presets::BUILTINS {
        println!(
            "{:<12} {:<6} carrier {:>4} Hz, beat {:>2} Hz  {}",
            p.name, p.band, p.carrier, p.beat, p.blurb
        );
    }
}

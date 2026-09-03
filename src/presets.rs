pub struct Builtin {
    pub name: &'static str,
    pub band: &'static str,
    pub blurb: &'static str,
    pub carrier: f32,
    pub beat: f32,
}

pub const BUILTINS: &[Builtin] = &[
    Builtin {
        name: "40 Hz Focus",
        band: "Gamma",
        blurb: "Sustained attention, deep work",
        carrier: 200.0,
        beat: 40.0,
    },
    Builtin {
        name: "Focus",
        band: "Beta",
        blurb: "Alert, active concentration",
        carrier: 440.0,
        beat: 20.0,
    },
    Builtin {
        name: "Relax",
        band: "Alpha",
        blurb: "Calm, light meditation",
        carrier: 400.0,
        beat: 10.0,
    },
    Builtin {
        name: "Meditate",
        band: "Theta",
        blurb: "Meditation, creativity",
        carrier: 300.0,
        beat: 6.0,
    },
    Builtin {
        name: "Deep Sleep",
        band: "Delta",
        blurb: "Sleep, deep rest",
        carrier: 200.0,
        beat: 2.0,
    },
];

pub fn band_for(beat: f32) -> &'static str {
    match beat {
        b if b < 4.0 => "Delta",
        b if b < 8.0 => "Theta",
        b if b < 13.0 => "Alpha",
        b if b < 30.0 => "Beta",
        _ => "Gamma",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands() {
        assert_eq!(band_for(2.0), "Delta");
        assert_eq!(band_for(6.0), "Theta");
        assert_eq!(band_for(10.0), "Alpha");
        assert_eq!(band_for(20.0), "Beta");
        assert_eq!(band_for(40.0), "Gamma");
    }
}

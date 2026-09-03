pub const CARRIER_RANGE: std::ops::RangeInclusive<f32> = 40.0..=1500.0;
pub const BEAT_RANGE: std::ops::RangeInclusive<f32> = 0.5..=100.0;

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

/// Parse a user-typed frequency, accepting an optional "Hz" suffix.
pub fn parse_hz(text: &str, range: &std::ops::RangeInclusive<f32>) -> Result<f32, String> {
    let trimmed = text.trim().trim_end_matches(['h', 'H', 'z', 'Z']).trim();
    let value: f32 = trimmed
        .parse()
        .map_err(|_| format!("\"{}\" is not a number", text.trim()))?;
    if !value.is_finite() || !range.contains(&value) {
        return Err(format!(
            "must be between {} and {} Hz",
            range.start(),
            range.end()
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_and_suffixed() {
        assert_eq!(parse_hz("40", &BEAT_RANGE), Ok(40.0));
        assert_eq!(parse_hz(" 7.83 Hz ", &BEAT_RANGE), Ok(7.83));
        assert_eq!(parse_hz("200hz", &CARRIER_RANGE), Ok(200.0));
    }

    #[test]
    fn rejects_out_of_range_and_garbage() {
        assert!(parse_hz("0", &BEAT_RANGE).is_err());
        assert!(parse_hz("5000", &CARRIER_RANGE).is_err());
        assert!(parse_hz("abc", &BEAT_RANGE).is_err());
        assert!(parse_hz("NaN", &BEAT_RANGE).is_err());
    }

    #[test]
    fn bands() {
        assert_eq!(band_for(2.0), "Delta");
        assert_eq!(band_for(6.0), "Theta");
        assert_eq!(band_for(10.0), "Alpha");
        assert_eq!(band_for(20.0), "Beta");
        assert_eq!(band_for(40.0), "Gamma");
    }
}

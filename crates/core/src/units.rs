//! Length units. All model coordinates are `i64` nanometres.

use serde::{Deserialize, Serialize};

pub const NM_PER_MM: i64 = 1_000_000;
pub const NM_PER_MIL: i64 = 25_400;
pub const NM_PER_INCH: i64 = 25_400_000;

/// Convert millimetres to nanometres, rounding to the nearest nanometre.
pub fn mm(v: f64) -> i64 {
    (v * NM_PER_MM as f64).round() as i64
}

/// Convert mils (thousandths of an inch) to nanometres.
pub fn mil(v: f64) -> i64 {
    (v * NM_PER_MIL as f64).round() as i64
}

pub fn to_mm(nm: i64) -> f64 {
    nm as f64 / NM_PER_MM as f64
}

pub fn to_mil(nm: i64) -> f64 {
    nm as f64 / NM_PER_MIL as f64
}

/// Display unit chosen by the user. Only affects presentation and input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unit {
    #[default]
    Mm,
    Mil,
    Inch,
}

impl Unit {
    pub fn label(self) -> &'static str {
        match self {
            Unit::Mm => "mm",
            Unit::Mil => "mil",
            Unit::Inch => "inch",
        }
    }

    pub fn from_nm(self, nm: i64) -> f64 {
        match self {
            Unit::Mm => to_mm(nm),
            Unit::Mil => to_mil(nm),
            Unit::Inch => nm as f64 / NM_PER_INCH as f64,
        }
    }

    pub fn to_nm(self, v: f64) -> i64 {
        match self {
            Unit::Mm => mm(v),
            Unit::Mil => mil(v),
            Unit::Inch => (v * NM_PER_INCH as f64).round() as i64,
        }
    }

    /// Format a length with a sensible number of decimals for the unit.
    pub fn format(self, nm: i64) -> String {
        let v = self.from_nm(nm);
        let s = match self {
            Unit::Mm => format!("{v:.3}"),
            Unit::Mil => format!("{v:.1}"),
            Unit::Inch => format!("{v:.4}"),
        };
        trim_zeros(&s)
    }
}

/// Parse a user-entered length such as `"0.2"`, `"8mil"`, `"0.1 in"`.
/// A bare number is interpreted in `default` units.
pub fn parse_length(input: &str, default: Unit) -> Option<i64> {
    let s = input.trim().to_ascii_lowercase();
    let (num, unit) = if let Some(n) = s.strip_suffix("mm") {
        (n, Unit::Mm)
    } else if let Some(n) = s.strip_suffix("mil") {
        (n, Unit::Mil)
    } else if let Some(n) = s.strip_suffix("inch") {
        (n, Unit::Inch)
    } else if let Some(n) = s.strip_suffix("in") {
        (n, Unit::Inch)
    } else if let Some(n) = s.strip_suffix('"') {
        (n, Unit::Inch)
    } else {
        (s.as_str(), default)
    };
    let v: f64 = num.trim().parse().ok()?;
    v.is_finite().then(|| unit.to_nm(v))
}

fn trim_zeros(s: &str) -> String {
    if s.contains('.') {
        let t = s.trim_end_matches('0').trim_end_matches('.');
        if t == "-0" { "0".into() } else { t.into() }
    } else {
        s.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_round_trip() {
        assert_eq!(mm(1.0), 1_000_000);
        assert_eq!(mil(100.0), 2_540_000);
        assert_eq!(Unit::Mil.from_nm(mil(8.0)), 8.0);
    }

    #[test]
    fn parses_suffixes() {
        assert_eq!(parse_length("8mil", Unit::Mm), Some(mil(8.0)));
        assert_eq!(parse_length(" 0.2 ", Unit::Mm), Some(mm(0.2)));
        assert_eq!(parse_length("0.1in", Unit::Mm), Some(mil(100.0)));
        assert_eq!(parse_length("abc", Unit::Mm), None);
    }

    #[test]
    fn formats_without_trailing_zeros() {
        assert_eq!(Unit::Mm.format(mm(0.25)), "0.25");
        assert_eq!(Unit::Mm.format(mm(2.0)), "2");
        assert_eq!(Unit::Mil.format(mil(10.0)), "10");
    }
}

//! Helpers behind the IC Symbol Wizard: turn a pin table pasted from a
//! datasheet into a symbol.

use crate::library::{PinKind, Symbol};

fn parse_kind(s: &str) -> Option<PinKind> {
    Some(match s.to_lowercase().as_str() {
        "in" | "input" | "i" => PinKind::Input,
        "out" | "output" | "o" => PinKind::Output,
        "io" | "i/o" | "bidir" | "bidirectional" | "b" => PinKind::Bidirectional,
        "pwr" | "power" | "power_in" | "p" | "supply" => PinKind::PowerIn,
        "power_out" | "pout" => PinKind::PowerOut,
        "oc" | "od" | "open_collector" | "open_drain" => PinKind::OpenCollector,
        "nc" | "n/c" => PinKind::NotConnected,
        "passive" | "pas" | "a" | "analog" => PinKind::Passive,
        _ => return None,
    })
}

/// Guess a pin type from its name when none is given.
pub fn guess_kind(name: &str) -> PinKind {
    let n = name.to_uppercase();
    if n == "NC" || n == "N/C" || n == "DNC" {
        PinKind::NotConnected
    } else if n.contains("GND")
        || n.starts_with("VCC")
        || n.starts_with("VDD")
        || n.starts_with("VSS")
        || n.starts_with("AVCC")
        || n.starts_with("VBAT")
        || n == "VIN"
    {
        PinKind::PowerIn
    } else if n.starts_with("VOUT") || n == "VO" {
        PinKind::PowerOut
    } else {
        PinKind::Bidirectional
    }
}

/// Parse "number name [type]" lines. Columns may be separated by tabs,
/// commas, semicolons or spaces. Lines starting with `#` are comments.
pub fn parse_pin_list(text: &str) -> Vec<(String, String, PinKind)> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let parts: Vec<&str> = l
                .split(|c: char| c == '\t' || c == ',' || c == ';' || c.is_whitespace())
                .filter(|s| !s.is_empty())
                .collect();
            match parts.as_slice() {
                [] => None,
                [num] => Some((num.to_string(), num.to_string(), PinKind::Passive)),
                [num, name] => Some((num.to_string(), name.to_string(), guess_kind(name))),
                [num, name, kind, ..] => Some((
                    num.to_string(),
                    name.to_string(),
                    parse_kind(kind).unwrap_or_else(|| guess_kind(name)),
                )),
            }
        })
        .collect()
}

/// Build a box symbol with the first half of the pins on the left.
pub fn ic_symbol(id: &str, name: &str, pins: &[(String, String, PinKind)]) -> Symbol {
    let half = pins.len().div_ceil(2);
    let refs: Vec<(&str, &str, PinKind)> = pins
        .iter()
        .map(|(a, b, k)| (a.as_str(), b.as_str(), *k))
        .collect();
    crate::builtin::symbols::ic_box(id, name, &refs[..half], &refs[half..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mixed_separators() {
        let pins =
            parse_pin_list("# pinout\n1 VCC\n2\tSDA\tio\n3,SCL,in\n4;GND\n5 NC\n6 EN bogus\n\n");
        assert_eq!(pins.len(), 6);
        assert_eq!(pins[0], ("1".into(), "VCC".into(), PinKind::PowerIn));
        assert_eq!(pins[1].2, PinKind::Bidirectional);
        assert_eq!(pins[2].2, PinKind::Input);
        assert_eq!(pins[3].2, PinKind::PowerIn);
        assert_eq!(pins[4].2, PinKind::NotConnected);
        assert_eq!(
            pins[5].2,
            PinKind::Bidirectional,
            "unknown type falls back to a guess"
        );
    }

    #[test]
    fn builds_balanced_symbol() {
        let pins = parse_pin_list("1 A\n2 B\n3 C\n4 D\n5 E");
        let s = ic_symbol("sym:x", "X", &pins);
        assert_eq!(s.pins.len(), 5);
        let left = s.pins.iter().filter(|p| p.pos.x < 0).count();
        assert_eq!(left, 3);
    }
}

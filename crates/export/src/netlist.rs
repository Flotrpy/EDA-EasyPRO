//! Netlist interchange formats.

use easypro_core::library::Library;
use easypro_core::netlist::{connectivity, natural_cmp};
use easypro_core::schematic::Schematic;
use std::fmt::Write;

fn q(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// KiCad `.net` (S-expression, version E) for use with KiCad's PCB editor
/// or any tool that reads it.
pub fn kicad_netlist(sch: &Schematic, lib: &Library, source: &str) -> String {
    let conn = connectivity(sch, lib);
    let mut out = String::new();
    out.push_str("(export (version \"E\")\n");
    let _ = writeln!(out, "  (design (source {}) (tool \"EasyPRO\"))", q(source));
    out.push_str("  (components\n");
    let mut comps: Vec<_> = sch.components().map(|(_, c)| c).collect();
    comps.sort_by(|a, b| natural_cmp(&a.designator, &b.designator));
    for c in comps {
        let fp = lib
            .devices
            .get(&c.device)
            .and_then(|d| d.footprint.as_ref())
            .and_then(|f| lib.footprints.get(f))
            .map(|f| f.name.clone())
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "    (comp (ref {}) (value {}) (footprint {}))",
            q(&c.designator),
            q(&c.value),
            q(&fp)
        );
    }
    out.push_str("  )\n  (nets\n");
    for (i, net) in conn.nets.iter().enumerate() {
        let _ = write!(out, "    (net (code \"{}\") (name {})", i + 1, q(&net.name));
        for node in &net.nodes {
            let pad = lib
                .devices
                .get(
                    &sch.find_component(node.comp)
                        .map(|c| c.device.clone())
                        .unwrap_or_default(),
                )
                .map(|d| d.pad_for_pin(&node.pin).to_string())
                .unwrap_or_else(|| node.pin.clone());
            let _ = write!(
                out,
                "\n      (node (ref {}) (pin {}) (pinfunction {}))",
                q(&node.designator),
                q(&pad),
                q(&node.pin_name)
            );
        }
        out.push_str(")\n");
    }
    out.push_str("  )\n)\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use easypro_core::examples::blinker;
    use easypro_core::sexpr::parse;

    #[test]
    fn kicad_netlist_parses_back() {
        let p = blinker();
        let text = kicad_netlist(&p.boards[0].schematic, &p.library, "blinker");
        let s = parse(&text).unwrap();
        assert_eq!(s.head(), Some("export"));
        let nets = s.get("nets").unwrap();
        let gnd = nets
            .all("net")
            .find(|n| n.get("name").and_then(|x| x.arg(0)) == Some("GND"))
            .unwrap();
        assert!(gnd.all("node").count() >= 5);
    }
}

//! Bill of materials and component placement (pick-and-place) files.

use easypro_core::layer::Side;
use easypro_core::library::Library;
use easypro_core::netlist::natural_cmp;
use easypro_core::pcb::Pcb;
use easypro_core::schematic::Schematic;
use easypro_core::units::to_mm;
use std::collections::BTreeMap;

/// Quote a CSV field when needed.
pub fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn row(fields: &[&str]) -> String {
    fields
        .iter()
        .map(|f| csv_field(f))
        .collect::<Vec<_>>()
        .join(",")
        + "\n"
}

#[derive(Clone, Debug, PartialEq)]
pub struct BomLine {
    pub designators: Vec<String>,
    pub value: String,
    pub footprint: String,
    pub device: String,
    pub manufacturer: String,
    pub mpn: String,
    pub supplier_part: String,
}

/// Group identical parts (same device, value and footprint).
pub fn lines(sch: &Schematic, lib: &Library) -> Vec<BomLine> {
    let mut groups: BTreeMap<(String, String, String), BomLine> = BTreeMap::new();
    for (_, c) in sch.components() {
        if c.exclude_from_bom {
            continue;
        }
        let dev = lib.devices.get(&c.device);
        let Some(fp) = dev.and_then(|d| d.footprint.clone()) else {
            continue;
        };
        let fp_name = lib
            .footprints
            .get(&fp)
            .map(|f| f.name.clone())
            .unwrap_or(fp);
        let key = (c.device.clone(), c.value.clone(), fp_name.clone());
        let attr = |k: &str| c.attr(k, lib).unwrap_or("").to_string();
        let e = groups.entry(key).or_insert_with(|| BomLine {
            designators: vec![],
            value: c.value.clone(),
            footprint: fp_name,
            device: dev.map(|d| d.name.clone()).unwrap_or_default(),
            manufacturer: attr("Manufacturer"),
            mpn: attr("MPN"),
            supplier_part: attr("LCSC"),
        });
        e.designators.push(c.designator.clone());
    }
    let mut out: Vec<BomLine> = groups.into_values().collect();
    for l in &mut out {
        l.designators.sort_by(|a, b| natural_cmp(a, b));
    }
    out.sort_by(|a, b| natural_cmp(&a.designators[0], &b.designators[0]));
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BomStyle {
    /// Item, Qty, Designators, Value, Footprint, Manufacturer, MPN, Supplier part.
    Generic,
    /// Comment, Designator, Footprint, LCSC Part # (JLCPCB assembly).
    Jlc,
}

pub fn bom_csv(lines: &[BomLine], style: BomStyle) -> String {
    let mut out = String::new();
    match style {
        BomStyle::Generic => {
            out += &row(&[
                "Item",
                "Qty",
                "Designators",
                "Value",
                "Footprint",
                "Description",
                "Manufacturer",
                "MPN",
                "Supplier Part",
            ]);
            for (i, l) in lines.iter().enumerate() {
                out += &row(&[
                    &(i + 1).to_string(),
                    &l.designators.len().to_string(),
                    &l.designators.join(","),
                    &l.value,
                    &l.footprint,
                    &l.device,
                    &l.manufacturer,
                    &l.mpn,
                    &l.supplier_part,
                ]);
            }
        }
        BomStyle::Jlc => {
            out += &row(&["Comment", "Designator", "Footprint", "LCSC Part #"]);
            for l in lines {
                out += &row(&[
                    &l.value,
                    &l.designators.join(","),
                    &l.footprint,
                    &l.supplier_part,
                ]);
            }
        }
    }
    out
}

/// Pick-and-place / CPL: one line per placed SMD or THT part.
pub fn cpl_csv(pcb: &Pcb, lib: &Library, style: BomStyle) -> String {
    let mut parts: Vec<_> = pcb
        .components
        .iter()
        .filter(|c| {
            lib.footprints
                .get(&c.footprint)
                .is_some_and(|f| !f.pads.is_empty())
        })
        .collect();
    parts.sort_by(|a, b| natural_cmp(&a.designator, &b.designator));
    let mut out = String::new();
    let f = |nm: i64| format!("{:.4}", to_mm(nm));
    match style {
        BomStyle::Jlc => {
            out += &row(&["Designator", "Mid X", "Mid Y", "Layer", "Rotation"]);
            for c in parts {
                let layer = if c.side == Side::Top { "Top" } else { "Bottom" };
                out += &row(&[
                    &c.designator,
                    &format!("{}mm", f(c.pos.x)),
                    &format!("{}mm", f(c.pos.y)),
                    layer,
                    &format!("{}", c.rotation),
                ]);
            }
        }
        BomStyle::Generic => {
            out += &row(&[
                "Designator",
                "Value",
                "Package",
                "Mid X (mm)",
                "Mid Y (mm)",
                "Rotation",
                "Side",
            ]);
            for c in parts {
                let pkg = lib
                    .footprints
                    .get(&c.footprint)
                    .map(|f| f.name.as_str())
                    .unwrap_or("");
                let side = if c.side == Side::Top { "top" } else { "bottom" };
                out += &row(&[
                    &c.designator,
                    &c.value,
                    pkg,
                    &f(c.pos.x),
                    &f(c.pos.y),
                    &format!("{}", c.rotation),
                    side,
                ]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use easypro_core::examples::blinker;

    #[test]
    fn groups_identical_parts() {
        let p = blinker();
        let l = lines(&p.boards[0].schematic, &p.library);
        let c100n = l.iter().find(|l| l.value == "100nF").unwrap();
        assert_eq!(c100n.designators.len(), 1);
        assert_eq!(l.iter().map(|l| l.designators.len()).sum::<usize>(), 9);
        let csv = bom_csv(&l, BomStyle::Generic);
        assert!(csv.lines().next().unwrap().starts_with("Item,Qty"));
        let jlc = bom_csv(&l, BomStyle::Jlc);
        assert!(jlc.starts_with("Comment,Designator,Footprint,LCSC Part #"));
    }

    #[test]
    fn cpl_lists_every_part() {
        let p = blinker();
        let csv = cpl_csv(&p.boards[0].pcb, &p.library, BomStyle::Jlc);
        assert_eq!(csv.lines().count(), 10);
        assert!(csv.contains("U1,25.0000mm,22.0000mm,Top,0"));
    }

    #[test]
    fn csv_quoting() {
        assert_eq!(csv_field("R1,R2"), "\"R1,R2\"");
        assert_eq!(csv_field("a\"b"), "\"a\"\"b\"");
        assert_eq!(csv_field("plain"), "plain");
    }
}

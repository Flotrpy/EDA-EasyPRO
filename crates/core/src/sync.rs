//! Annotation and "Update PCB from schematic".

use crate::geom::Point;
use crate::id::IdGen;
use crate::layer::Side;
use crate::library::Library;
use crate::netlist::{connectivity, natural_cmp};
use crate::pcb::{Pcb, PcbComponent};
use crate::project::Board;
use crate::units::mm;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Assign designators. With `reset`, every part is renumbered; otherwise
/// only parts ending in `?` or clashing with another part are.
/// Order: by sheet, then top-to-bottom, left-to-right.
pub fn annotate(board: &mut Board, lib: &Library, reset: bool) -> usize {
    let mut order: Vec<(usize, usize)> = Vec::new();
    for (si, s) in board.schematic.sheets.iter().enumerate() {
        for ci in 0..s.components.len() {
            order.push((si, ci));
        }
    }
    order.sort_by(|a, b| {
        let ca = &board.schematic.sheets[a.0].components[a.1];
        let cb = &board.schematic.sheets[b.0].components[b.1];
        a.0.cmp(&b.0)
            .then(cb.pos.y.cmp(&ca.pos.y))
            .then(ca.pos.x.cmp(&cb.pos.x))
    });

    let prefix_of = |device: &str, current: &str| -> String {
        lib.devices
            .get(device)
            .map(|d| d.prefix.clone())
            .unwrap_or_else(|| {
                current
                    .trim_end_matches(|c: char| c.is_ascii_digit() || c == '?')
                    .to_string()
            })
    };

    let mut used: HashSet<String> = HashSet::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut needs = Vec::new();
    for &(si, ci) in &order {
        let c = &board.schematic.sheets[si].components[ci];
        let fresh = reset
            || c.designator.ends_with('?')
            || c.designator.is_empty()
            || !seen.insert(c.designator.clone());
        if fresh {
            needs.push((si, ci));
        } else {
            used.insert(c.designator.clone());
        }
    }
    let mut next: HashMap<String, u32> = HashMap::new();
    for &(si, ci) in &needs {
        let c = &board.schematic.sheets[si].components[ci];
        let prefix = prefix_of(&c.device, &c.designator);
        let n = next.entry(prefix.clone()).or_insert(1);
        while used.contains(&format!("{prefix}{n}")) {
            *n += 1;
        }
        let des = format!("{prefix}{n}");
        used.insert(des.clone());
        board.schematic.sheets[si].components[ci].designator = des;
    }
    needs.len()
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SyncReport {
    pub added: Vec<String>,
    pub updated: Vec<String>,
    pub removed: Vec<String>,
    pub warnings: Vec<String>,
}

impl SyncReport {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.updated.is_empty() && self.removed.is_empty()
    }

    pub fn summary(&self) -> String {
        format!(
            "{} added, {} updated, {} removed",
            self.added.len(),
            self.updated.len(),
            self.removed.len()
        )
    }
}

/// Bring the PCB in line with the schematic: add new footprints, update
/// values, footprints and pad nets, and delete footprints whose symbol is
/// gone. Placement of existing parts is never touched.
pub fn update_pcb(board: &mut Board, lib: &Library, ids: &mut IdGen) -> SyncReport {
    let conn = connectivity(&board.schematic, lib);
    let mut report = SyncReport::default();
    let pcb: &mut Pcb = &mut board.pcb;

    // Staging area for new parts: to the right of the board.
    let area = pcb
        .outline()
        .map(crate::geom::Rect::from_points)
        .unwrap_or(crate::geom::Rect::new(Point::ZERO, Point::ZERO));
    let mut cursor = Point::new(area.max.x + mm(10.0), area.max.y);
    let mut row_height = 0;
    let row_limit = area.max.x + mm(80.0);

    let mut alive = HashSet::new();
    let mut seen_designators: BTreeMap<String, usize> = BTreeMap::new();
    let mut parts: Vec<_> = board
        .schematic
        .components()
        .map(|(_, c)| c.clone())
        .collect();
    parts.sort_by(|a, b| natural_cmp(&a.designator, &b.designator));

    for c in &parts {
        *seen_designators.entry(c.designator.clone()).or_default() += 1;
        let Some(dev) = lib.devices.get(&c.device) else {
            report.warnings.push(format!(
                "{}: device {} not found in project library",
                c.designator, c.device
            ));
            continue;
        };
        let Some(fp_id) = dev.footprint.clone() else {
            continue;
        };
        let Some(fp) = lib.footprints.get(&fp_id) else {
            report
                .warnings
                .push(format!("{}: footprint {fp_id} not found", c.designator));
            continue;
        };
        alive.insert(c.id);

        let mut pad_nets = BTreeMap::new();
        if let Some(sym) = lib.symbols.get(&dev.symbol) {
            for pin in &sym.pins {
                if let Some(net) = conn.net_of_pin(c.id, &pin.number) {
                    pad_nets.insert(dev.pad_for_pin(&pin.number).to_string(), net.name.clone());
                }
            }
        }

        if let Some(existing) = pcb.components.iter_mut().find(|p| p.link == Some(c.id)) {
            let mut changes = Vec::new();
            if existing.designator != c.designator {
                changes.push(format!(
                    "designator {} -> {}",
                    existing.designator, c.designator
                ));
                existing.designator = c.designator.clone();
            }
            if existing.value != c.value {
                changes.push(format!("value {} -> {}", existing.value, c.value));
                existing.value = c.value.clone();
            }
            if existing.footprint != fp_id || existing.device != c.device {
                changes.push(format!("footprint {} -> {}", existing.footprint, fp_id));
                existing.footprint = fp_id.clone();
                existing.device = c.device.clone();
            }
            if existing.pad_nets != pad_nets {
                changes.push("nets".to_string());
                existing.pad_nets = pad_nets;
            }
            if !changes.is_empty() {
                report
                    .updated
                    .push(format!("{}: {}", c.designator, changes.join(", ")));
            }
        } else {
            let b = fp.bbox();
            let (w, h) = (b.width().max(mm(1.0)), b.height().max(mm(1.0)));
            if cursor.x + w > row_limit && cursor.x > area.max.x + mm(10.0) {
                cursor = Point::new(area.max.x + mm(10.0), cursor.y - row_height - mm(3.0));
                row_height = 0;
            }
            let pos = Point::new(
                cursor.x + w / 2 - b.center().x,
                cursor.y - h / 2 - b.center().y,
            );
            cursor.x += w + mm(3.0);
            row_height = row_height.max(h);
            pcb.components.push(PcbComponent {
                id: ids.next(),
                link: Some(c.id),
                designator: c.designator.clone(),
                value: c.value.clone(),
                device: c.device.clone(),
                footprint: fp_id,
                pos: pos.snap(mm(0.5)),
                rotation: 0.0,
                side: Side::Top,
                locked: false,
                pad_nets,
                designator_offset: None,
                show_designator: true,
            });
            report.added.push(c.designator.clone());
        }
    }

    pcb.components.retain(|p| {
        let keep = p.link.is_none_or(|l| alive.contains(&l));
        if !keep {
            report.removed.push(p.designator.clone());
        }
        keep
    });

    for (des, n) in seen_designators {
        if n > 1 {
            report
                .warnings
                .push(format!("designator {des} is used {n} times; run Annotate"));
        } else if des.ends_with('?') {
            report.warnings.push(format!("{des} is not annotated"));
        }
    }
    let nets: HashSet<String> = conn.nets.iter().map(|n| n.name.clone()).collect();
    let orphan_tracks = pcb
        .tracks
        .iter()
        .filter(|t| !t.net.is_empty() && !nets.contains(&t.net))
        .count();
    if orphan_tracks > 0 {
        report.warnings.push(format!(
            "{orphan_tracks} track(s) belong to nets that no longer exist"
        ));
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin;
    use crate::geom::Point;
    use crate::id::Id;
    use crate::project::Project;
    use crate::schematic::{Component, Wire};

    fn part(id: u64, device: &str, des: &str, x: f64, y: f64) -> Component {
        Component {
            id: Id(id),
            device: device.into(),
            designator: des.into(),
            value: "10k".into(),
            pos: Point::new(crate::units::mil(x), crate::units::mil(y)),
            rotation: 0.0,
            mirror: false,
            attributes: Default::default(),
            exclude_from_bom: false,
            designator_offset: None,
            value_offset: None,
        }
    }

    fn project() -> Project {
        let mut p = Project::new("t");
        p.library.import_device(builtin::library(), "dev:R-0603");
        p.ids.bump(Id(100));
        let s = &mut p.boards[0].schematic.sheets[0];
        s.components.push(part(10, "dev:R-0603", "R?", 0.0, 0.0));
        s.components.push(part(11, "dev:R-0603", "R?", 1000.0, 0.0));
        s.wires.push(Wire {
            id: Id(12),
            points: vec![
                Point::new(crate::units::mil(200.0), 0),
                Point::new(crate::units::mil(800.0), 0),
            ],
        });
        p
    }

    #[test]
    fn annotate_numbers_left_to_right() {
        let mut p = project();
        let lib = p.library.clone();
        assert_eq!(annotate(&mut p.boards[0], &lib, false), 2);
        let des: Vec<_> = p.boards[0].schematic.sheets[0]
            .components
            .iter()
            .map(|c| c.designator.clone())
            .collect();
        assert_eq!(des, vec!["R1", "R2"]);
        assert_eq!(annotate(&mut p.boards[0], &lib, false), 0);
    }

    #[test]
    fn update_pcb_adds_updates_and_removes() {
        let mut p = project();
        let lib = p.library.clone();
        annotate(&mut p.boards[0], &lib, false);
        let r = update_pcb(&mut p.boards[0], &lib, &mut p.ids);
        assert_eq!(r.added, vec!["R1", "R2"]);
        let pcb = &p.boards[0].pcb;
        let r1 = pcb
            .components
            .iter()
            .find(|c| c.designator == "R1")
            .unwrap();
        assert_eq!(r1.net_of("2"), "Net-(R1-2)");
        assert_eq!(r1.net_of("1"), "");

        // Running again is a no-op.
        let r = update_pcb(&mut p.boards[0], &lib, &mut p.ids);
        assert!(r.is_empty(), "{r:?}");

        // Change a value and delete R2.
        p.boards[0].schematic.sheets[0].components[0].value = "4k7".into();
        p.boards[0].schematic.sheets[0].components.pop();
        let r = update_pcb(&mut p.boards[0], &lib, &mut p.ids);
        assert_eq!(r.removed, vec!["R2"]);
        assert_eq!(r.updated.len(), 1);
        assert_eq!(p.boards[0].pcb.components[0].value, "4k7");
    }
}

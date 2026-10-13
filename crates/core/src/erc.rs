//! Electrical rule check for schematics.

use crate::check::{sort, Issue, Severity};
use crate::geom::{point_on_segment, Point};
use crate::library::{Library, PinKind};
use crate::netlist::{connectivity, Connectivity};
use crate::schematic::Schematic;
use std::collections::{HashMap, HashSet};

pub fn run(sch: &Schematic, lib: &Library) -> Vec<Issue> {
    let conn = connectivity(sch, lib);
    run_with(sch, lib, &conn)
}

pub fn run_with(sch: &Schematic, lib: &Library, conn: &Connectivity) -> Vec<Issue> {
    let mut out = Vec::new();

    // Designators.
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for (si, c) in sch.components() {
        if c.designator.ends_with('?') || c.designator.is_empty() {
            out.push(
                Issue::new(
                    Severity::Error,
                    format!("{} is not annotated", c.device_label(lib)),
                )
                .on_sheet(si)
                .at(c.pos)
                .with(c.id),
            );
        }
        *seen.entry(c.designator.as_str()).or_default() += 1;
        if !lib.devices.contains_key(&c.device) {
            out.push(
                Issue::new(
                    Severity::Error,
                    format!(
                        "{}: device {} is missing from the project library",
                        c.designator, c.device
                    ),
                )
                .on_sheet(si)
                .at(c.pos)
                .with(c.id),
            );
        }
    }
    for (d, n) in seen {
        if n > 1 && !d.ends_with('?') {
            out.push(Issue::new(
                Severity::Error,
                format!("Designator {d} is used {n} times"),
            ));
        }
    }

    // Unconnected pins (unless marked no-connect or of kind NC).
    let nc: Vec<HashSet<Point>> = sch
        .sheets
        .iter()
        .map(|s| s.no_connects.iter().map(|n| n.pos).collect())
        .collect();
    for (si, comp, pin, pos) in &conn.unconnected {
        if nc[*si].contains(pos) {
            continue;
        }
        let Some(c) = sch.sheets[*si].component(*comp) else {
            continue;
        };
        let kind = lib
            .devices
            .get(&c.device)
            .and_then(|d| lib.symbols.get(&d.symbol))
            .and_then(|s| s.pin(pin))
            .map(|p| p.kind);
        if kind == Some(PinKind::NotConnected) {
            continue;
        }
        out.push(
            Issue::new(
                Severity::Warning,
                format!("Pin {}.{} is not connected", c.designator, pin),
            )
            .on_sheet(*si)
            .at(*pos)
            .with(c.id),
        );
    }

    // No-connect markers on connected pins.
    for (si, s) in sch.sheets.iter().enumerate() {
        for n in &s.no_connects {
            let connected = s.components.iter().any(|c| {
                lib.devices
                    .get(&c.device)
                    .and_then(|d| lib.symbols.get(&d.symbol))
                    .map(|sym| {
                        c.pin_positions(sym).any(|(num, p)| {
                            p == n.pos
                                && conn
                                    .net_of_pin(c.id, num)
                                    .is_some_and(|net| net.nodes.len() > 1)
                        })
                    })
                    .unwrap_or(false)
            });
            if connected {
                out.push(
                    Issue::new(
                        Severity::Warning,
                        "No-connect flag placed on a connected pin",
                    )
                    .on_sheet(si)
                    .at(n.pos)
                    .with(n.id),
                );
            }
        }
    }

    // Net level checks.
    for net in &conn.nets {
        if net.nodes.len() == 1 {
            let n = &net.nodes[0];
            out.push(
                Issue::new(
                    Severity::Warning,
                    format!(
                        "Net {} has only one pin ({}.{})",
                        net.name, n.designator, n.pin
                    ),
                )
                .with(n.comp),
            );
        }
        if net.aliases.len() > 1 {
            out.push(Issue::new(
                Severity::Warning,
                format!("Net has several names: {}", net.aliases.join(", ")),
            ));
        }
        let drivers = net
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, PinKind::Output | PinKind::PowerOut))
            .count();
        if drivers > 1 {
            out.push(Issue::new(
                Severity::Error,
                format!("Net {} is driven by {drivers} outputs", net.name),
            ));
        }
        let has_power_in = net.nodes.iter().any(|n| n.kind == PinKind::PowerIn);
        let has_source = net.named || net.nodes.iter().any(|n| n.kind == PinKind::PowerOut);
        if has_power_in && !has_source {
            out.push(Issue::new(
                Severity::Info,
                format!(
                    "Power input on net {} has no power port or source",
                    net.name
                ),
            ));
        }
    }

    // Dangling wire ends and floating labels.
    for (si, s) in sch.sheets.iter().enumerate() {
        let mut anchors: HashSet<Point> = HashSet::new();
        for c in &s.components {
            if let Some(sym) = lib
                .devices
                .get(&c.device)
                .and_then(|d| lib.symbols.get(&d.symbol))
            {
                anchors.extend(c.pin_positions(sym).map(|(_, p)| p));
            }
        }
        anchors.extend(s.labels.iter().map(|l| l.pos));
        anchors.extend(s.ports.iter().map(|l| l.pos));
        anchors.extend(s.no_connects.iter().map(|l| l.pos));
        for w in &s.wires {
            for end in [w.points.first(), w.points.last()].into_iter().flatten() {
                let touches_wire = s.wires.iter().any(|o| {
                    o.id != w.id && o.segments().any(|(a, b)| point_on_segment(*end, a, b))
                });
                if !touches_wire && !anchors.contains(end) {
                    out.push(
                        Issue::new(Severity::Warning, "Wire end is not connected to anything")
                            .on_sheet(si)
                            .at(*end)
                            .with(w.id),
                    );
                }
            }
        }
        let wire_or_pin = |p: Point| {
            s.wires
                .iter()
                .any(|w| w.segments().any(|(a, b)| point_on_segment(p, a, b)))
                || s.components.iter().any(|c| {
                    lib.devices
                        .get(&c.device)
                        .and_then(|d| lib.symbols.get(&d.symbol))
                        .is_some_and(|sym| c.pin_positions(sym).any(|(_, q)| q == p))
                })
        };
        for l in &s.labels {
            if !wire_or_pin(l.pos) {
                out.push(
                    Issue::new(
                        Severity::Warning,
                        format!("Net label {} is not attached to a wire", l.name),
                    )
                    .on_sheet(si)
                    .at(l.pos)
                    .with(l.id),
                );
            }
        }
        for l in &s.ports {
            if !wire_or_pin(l.pos) {
                out.push(
                    Issue::new(
                        Severity::Warning,
                        format!("Port {} is not attached to a wire", l.name),
                    )
                    .on_sheet(si)
                    .at(l.pos)
                    .with(l.id),
                );
            }
        }
    }

    sort(&mut out);
    out
}

impl crate::schematic::Component {
    fn device_label(&self, lib: &Library) -> String {
        let name = lib
            .devices
            .get(&self.device)
            .map(|d| d.name.as_str())
            .unwrap_or(&self.device);
        format!("{} ({name})", self.designator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin;
    use crate::id::Id;
    use crate::project::Project;
    use crate::schematic::{Component, NetLabel, NoConnect};
    use crate::units::mil;

    fn comp(id: u64, des: &str) -> Component {
        Component {
            id: Id(id),
            device: "dev:R-0603".into(),
            designator: des.into(),
            value: "1k".into(),
            pos: Point::ZERO,
            rotation: 0.0,
            mirror: false,
            attributes: Default::default(),
            exclude_from_bom: false,
            designator_offset: None,
            value_offset: None,
        }
    }

    #[test]
    fn reports_common_problems() {
        let mut p = Project::new("t");
        p.library.import_device(builtin::library(), "dev:R-0603");
        let s = &mut p.boards[0].schematic.sheets[0];
        s.components.push(comp(10, "R?"));
        s.no_connects.push(NoConnect {
            id: Id(11),
            pos: Point::new(mil(-200.0), 0),
        });
        s.labels.push(NetLabel {
            id: Id(12),
            name: "X".into(),
            pos: Point::new(mil(5000.0), 0),
            rotation: 0.0,
        });
        let issues = run(&p.boards[0].schematic, &p.library);
        let msgs: Vec<_> = issues.iter().map(|i| i.message.clone()).collect();
        assert!(msgs.iter().any(|m| m.contains("not annotated")), "{msgs:?}");
        assert!(
            msgs.iter().any(|m| m.contains("Pin R?.2 is not connected")),
            "{msgs:?}"
        );
        assert!(
            !msgs.iter().any(|m| m.contains("R?.1")),
            "no-connect suppresses pin 1: {msgs:?}"
        );
        assert!(msgs.iter().any(|m| m.contains("Net label X")), "{msgs:?}");
        assert_eq!(issues[0].severity, Severity::Error);
    }
}

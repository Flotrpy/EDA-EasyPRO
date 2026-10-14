//! A generated design with hundreds of parts: connectivity, sync and DRC
//! must stay correct (and fast enough to run on every edit).

use easypro_core::builtin;
use easypro_core::geom::Point;
use easypro_core::id::Id;
use easypro_core::project::Project;
use easypro_core::schematic::{Component, Wire};
use easypro_core::units::{mil, mm, Unit};
use std::time::Instant;

const ROWS: i64 = 20;
const COLS: i64 = 30;

fn chain() -> Project {
    let mut p = Project::new("stress");
    p.library.import_device(builtin::library(), "dev:R-0603");
    let mut next = 1000u64;
    let mut id = || {
        next += 1;
        Id(next)
    };
    let sheet = &mut p.boards[0].schematic.sheets[0];
    for r in 0..ROWS {
        for c in 0..COLS {
            let pos = Point::new(mil(600.0 * c as f64), mil(400.0 * r as f64));
            sheet.components.push(Component {
                id: id(),
                device: "dev:R-0603".into(),
                designator: "R?".into(),
                value: "1k".into(),
                pos,
                rotation: 0.0,
                mirror: false,
                attributes: Default::default(),
                exclude_from_bom: false,
                designator_offset: None,
                value_offset: None,
            });
            // Pin 2 of this resistor to pin 1 of the next one in the row.
            if c + 1 < COLS {
                sheet.wires.push(Wire {
                    id: id(),
                    points: vec![
                        pos + Point::new(mil(200.0), 0),
                        pos + Point::new(mil(400.0), 0),
                    ],
                });
            }
        }
    }
    p.ids.bump(Id(next));
    p
}

#[test]
fn hundreds_of_parts() {
    let mut p = chain();
    let lib = p.library.clone();

    let t = Instant::now();
    let conn = easypro_core::netlist::connectivity(&p.boards[0].schematic, &lib);
    let conn_time = t.elapsed();
    assert_eq!(
        conn.nets.len() as i64,
        ROWS * (COLS - 1),
        "one net per joint"
    );

    assert_eq!(
        easypro_core::sync::annotate(&mut p.boards[0], &lib, false) as i64,
        ROWS * COLS
    );
    let report = easypro_core::sync::update_pcb(&mut p.boards[0], &lib, &mut p.ids);
    assert_eq!(report.added.len() as i64, ROWS * COLS);

    // Spread footprints out on a big board so DRC has real work to do.
    let pcb = &mut p.boards[0].pcb;
    for (i, c) in pcb.components.iter_mut().enumerate() {
        let (r, col) = (i as i64 / COLS, i as i64 % COLS);
        c.pos = Point::new(mm(5.0) + col * mm(4.0), mm(5.0) + r * mm(3.0));
    }
    let t = Instant::now();
    let issues = easypro_core::drc::run(pcb, &lib, Unit::Mm);
    let drc_time = t.elapsed();
    assert!(issues.iter().any(|i| i.message.contains("unrouted")));
    eprintln!(
        "connectivity {conn_time:?}, DRC {drc_time:?} for {} parts",
        ROWS * COLS
    );
}

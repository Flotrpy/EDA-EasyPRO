//! One-click fabrication packages with per-fab file naming.
//!
//! No ordering, no accounts: the user gets a zip they can upload to
//! whichever fab they like.

use crate::bom::{bom_csv, cpl_csv, lines, BomStyle};
use crate::drill::{excellon, holes};
use crate::gerber::{artwork, Artwork};
use crate::zip::ZipWriter;
use easypro_core::layer::{Layer, Side};
use easypro_core::library::Library;
use easypro_core::project::Board;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fab {
    Generic,
    Jlcpcb,
    Pcbway,
    Oshpark,
    Eurocircuits,
}

impl Fab {
    pub const ALL: [Fab; 5] = [
        Fab::Generic,
        Fab::Jlcpcb,
        Fab::Pcbway,
        Fab::Oshpark,
        Fab::Eurocircuits,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Fab::Generic => "Generic (Gerber X2)",
            Fab::Jlcpcb => "JLCPCB",
            Fab::Pcbway => "PCBWay",
            Fab::Oshpark => "OSH Park",
            Fab::Eurocircuits => "Eurocircuits",
        }
    }

    pub fn notes(self) -> &'static str {
        match self {
            Fab::Generic => "Gerber X2 with .gbr extensions; accepted by most fabs.",
            Fab::Jlcpcb => "Protel extensions plus JLC-format BOM and CPL for assembly.",
            Fab::Pcbway => "Protel extensions; BOM and centroid in generic format.",
            Fab::Oshpark => "Protel extensions, drill as .XLN. OSH Park ignores BOM/CPL.",
            Fab::Eurocircuits => "Gerber X2 plus generic BOM and pick-and-place.",
        }
    }

    fn protel(self) -> bool {
        matches!(self, Fab::Jlcpcb | Fab::Pcbway | Fab::Oshpark)
    }

    fn bom_style(self) -> BomStyle {
        if self == Fab::Jlcpcb {
            BomStyle::Jlc
        } else {
            BomStyle::Generic
        }
    }
}

fn layer_file(art: Artwork, copper_layers: u8, protel: bool, base: &str) -> String {
    if protel {
        let ext = match art {
            Artwork::Copper(Layer::TopCopper) => "GTL".to_string(),
            Artwork::Copper(Layer::BottomCopper) => "GBL".to_string(),
            Artwork::Copper(Layer::Inner(n)) => format!("G{n}"),
            Artwork::Copper(_) => "GTL".to_string(),
            Artwork::Silk(Side::Top) => "GTO".into(),
            Artwork::Silk(Side::Bottom) => "GBO".into(),
            Artwork::Mask(Side::Top) => "GTS".into(),
            Artwork::Mask(Side::Bottom) => "GBS".into(),
            Artwork::Paste(Side::Top) => "GTP".into(),
            Artwork::Paste(Side::Bottom) => "GBP".into(),
            Artwork::Outline => "GKO".into(),
        };
        format!("{base}.{ext}")
    } else {
        let name = match art {
            Artwork::Copper(l) => {
                let idx = Layer::copper_stack(copper_layers)
                    .iter()
                    .position(|x| *x == l)
                    .unwrap_or(0)
                    + 1;
                let side = match l {
                    Layer::TopCopper => "F",
                    Layer::BottomCopper => "B",
                    _ => "In",
                };
                format!("{side}_Cu_L{idx}")
            }
            Artwork::Silk(s) => format!("{}_Silkscreen", if s == Side::Top { "F" } else { "B" }),
            Artwork::Mask(s) => format!("{}_Mask", if s == Side::Top { "F" } else { "B" }),
            Artwork::Paste(s) => format!("{}_Paste", if s == Side::Top { "F" } else { "B" }),
            Artwork::Outline => "Edge_Cuts".into(),
        };
        format!("{base}-{name}.gbr")
    }
}

/// Every file in the package: (name, contents).
pub fn package_files(board: &Board, lib: &Library, fab: Fab, base: &str) -> Vec<(String, Vec<u8>)> {
    let pcb = &board.pcb;
    let protel = fab.protel();
    let mut arts: Vec<Artwork> = pcb
        .copper_stack()
        .into_iter()
        .map(Artwork::Copper)
        .collect();
    arts.extend([
        Artwork::Silk(Side::Top),
        Artwork::Silk(Side::Bottom),
        Artwork::Mask(Side::Top),
        Artwork::Mask(Side::Bottom),
        Artwork::Paste(Side::Top),
        Artwork::Paste(Side::Bottom),
        Artwork::Outline,
    ]);
    let mut files: Vec<(String, Vec<u8>)> = arts
        .into_iter()
        .map(|a| {
            (
                layer_file(a, pcb.copper_layers, protel, base),
                artwork(pcb, lib, a).into_bytes(),
            )
        })
        .collect();
    let drill_ext = if fab == Fab::Oshpark { "XLN" } else { "DRL" };
    let pth = holes(pcb, lib, true);
    files.push((
        format!("{base}-PTH.{drill_ext}"),
        excellon(&pth, true).into_bytes(),
    ));
    let npth = holes(pcb, lib, false);
    if npth.hits() > 0 {
        files.push((
            format!("{base}-NPTH.{drill_ext}"),
            excellon(&npth, false).into_bytes(),
        ));
    }
    if fab != Fab::Oshpark {
        let style = fab.bom_style();
        files.push((
            format!("{base}-BOM.csv"),
            bom_csv(&lines(&board.schematic, lib), style).into_bytes(),
        ));
        files.push((
            format!("{base}-CPL.csv"),
            cpl_csv(pcb, lib, style).into_bytes(),
        ));
    }
    files.push((
        "README.txt".into(),
        format!(
            "Fabrication files for {base}\nGenerated by EasyPRO for {}.\n{}\n\nBoard: {} copper layers, {:.2} mm thick.\n",
            fab.label(),
            fab.notes(),
            pcb.copper_layers,
            pcb.thickness as f64 / 1e6
        )
        .into_bytes(),
    ));
    files
}

pub fn package_zip(board: &Board, lib: &Library, fab: Fab, base: &str) -> Vec<u8> {
    let mut z = ZipWriter::default();
    for (name, data) in package_files(board, lib, fab, base) {
        z.add(&name, &data);
    }
    z.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use easypro_core::examples::blinker;

    #[test]
    fn jlc_package_contents() {
        let p = blinker();
        let files = package_files(&p.boards[0], &p.library, Fab::Jlcpcb, "blinker");
        let names: Vec<_> = files.iter().map(|f| f.0.as_str()).collect();
        for want in [
            "blinker.GTL",
            "blinker.GBL",
            "blinker.GTO",
            "blinker.GKO",
            "blinker-PTH.DRL",
            "blinker-BOM.csv",
            "blinker-CPL.csv",
        ] {
            assert!(names.contains(&want), "missing {want} in {names:?}");
        }
    }

    #[test]
    fn generic_and_oshpark_naming() {
        let p = blinker();
        let g = package_files(&p.boards[0], &p.library, Fab::Generic, "b");
        assert!(g.iter().any(|f| f.0 == "b-F_Cu_L1.gbr"));
        assert!(g.iter().any(|f| f.0 == "b-B_Cu_L2.gbr"));
        let o = package_files(&p.boards[0], &p.library, Fab::Oshpark, "b");
        assert!(o.iter().any(|f| f.0 == "b-PTH.XLN"));
        assert!(!o.iter().any(|f| f.0.ends_with(".csv")));
        let zip = package_zip(&p.boards[0], &p.library, Fab::Pcbway, "b");
        assert!(zip.len() > 1000);
    }
}

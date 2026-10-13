//! Projects and the on-disk file format.
//!
//! Files are pretty-printed JSON with sorted maps, so they diff and merge
//! well in git. Every file records its format version and is migrated on
//! load.

use crate::geom::{Point, Rect};
use crate::id::{Id, IdGen};
use crate::layer::Layer;
use crate::library::Library;
use crate::pcb::{Pcb, PcbGraphic};
use crate::schematic::{Schematic, Sheet};
use crate::shape::Shape;
use crate::units::mm;
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "easypro-project";
pub const FORMAT_VERSION: u32 = 1;
pub const FILE_EXTENSION: &str = "epro.json";

/// A schematic and the PCB built from it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Board {
    pub id: Id,
    pub name: String,
    pub schematic: Schematic,
    pub pcb: Pcb,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub format: String,
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// ISO-8601 timestamps, maintained by the app.
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub modified: String,
    #[serde(default)]
    pub library: Library,
    pub boards: Vec<Board>,
    #[serde(default)]
    pub ids: IdGen,
}

#[derive(Debug)]
pub enum LoadError {
    Json(serde_json::Error),
    NotAProject,
    TooNew(u32),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Json(e) => write!(f, "invalid project file: {e}"),
            LoadError::NotAProject => write!(f, "this file is not an EasyPRO project"),
            LoadError::TooNew(v) => write!(
                f,
                "project was saved by a newer version (format {v}); please update"
            ),
        }
    }
}

impl std::error::Error for LoadError {}

impl Project {
    /// A new project with one board, one empty sheet and a 50 x 40 mm
    /// board outline so the PCB editor never opens to a blank void.
    pub fn new(name: &str) -> Project {
        let mut ids = IdGen::default();
        let board_id = ids.next();
        let sheet_id = ids.next();
        let mut pcb = Pcb::default();
        let outline = Rect::new(Point::ZERO, Point::new(mm(50.0), mm(40.0)));
        pcb.graphics.push(PcbGraphic {
            id: ids.next(),
            layer: Layer::Outline,
            shape: Shape::Polygon {
                points: outline.corners().to_vec(),
                width: mm(0.1),
                fill: false,
            },
        });
        Project {
            format: FORMAT.into(),
            version: FORMAT_VERSION,
            name: name.into(),
            description: String::new(),
            created: String::new(),
            modified: String::new(),
            library: Library::default(),
            boards: vec![Board {
                id: board_id,
                name: "Board1".into(),
                schematic: Schematic {
                    sheets: vec![Sheet {
                        id: sheet_id,
                        name: "Sheet1".into(),
                        ..Default::default()
                    }],
                },
                pcb,
            }],
            ids,
        }
    }

    pub fn next_id(&mut self) -> Id {
        self.ids.next()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("project serializes")
    }

    pub fn from_json(text: &str) -> Result<Project, LoadError> {
        let value: serde_json::Value = serde_json::from_str(text).map_err(LoadError::Json)?;
        if value.get("format").and_then(|f| f.as_str()) != Some(FORMAT) {
            return Err(LoadError::NotAProject);
        }
        let version = value.get("version").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        if version > FORMAT_VERSION {
            return Err(LoadError::TooNew(version));
        }
        let value = migrate(value, version);
        let mut p: Project = serde_json::from_value(value).map_err(LoadError::Json)?;
        p.version = FORMAT_VERSION;
        p.repair_ids();
        Ok(p)
    }

    /// Make sure the id generator is ahead of every id in the file, so
    /// hand-edited or merged files never produce duplicates.
    pub fn repair_ids(&mut self) {
        for b in &self.boards {
            self.ids.bump(b.id);
            for s in &b.schematic.sheets {
                self.ids.bump(s.max_id());
            }
            self.ids.bump(b.pcb.max_id());
        }
    }

    pub fn board(&self, id: Id) -> Option<&Board> {
        self.boards.iter().find(|b| b.id == id)
    }

    pub fn board_mut(&mut self, id: Id) -> Option<&mut Board> {
        self.boards.iter_mut().find(|b| b.id == id)
    }

    pub fn add_board(&mut self, name: &str) -> Id {
        let template = Project::new(name);
        let id = self.next_id();
        let sheet_id = self.next_id();
        let outline_id = self.next_id();
        let mut b = template.boards.into_iter().next().expect("template board");
        b.id = id;
        b.name = name.into();
        b.schematic.sheets[0].id = sheet_id;
        b.pcb.graphics[0].id = outline_id;
        self.boards.push(b);
        id
    }

    pub fn add_sheet(&mut self, board: Id) -> Option<Id> {
        let id = self.next_id();
        let b = self.board_mut(board)?;
        let n = b.schematic.sheets.len() + 1;
        b.schematic.sheets.push(Sheet {
            id,
            name: format!("Sheet{n}"),
            ..Default::default()
        });
        Some(id)
    }
}

/// Upgrade older file layouts to the current one.
fn migrate(value: serde_json::Value, from: u32) -> serde_json::Value {
    // Version 1 is the first released format; future upgrades chain here:
    // if from < 2 { value = v1_to_v2(value) }
    let _ = from;
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_json() {
        let p = Project::new("Demo");
        let text = p.to_json();
        let q = Project::from_json(&text).unwrap();
        assert_eq!(p, q);
        assert_eq!(q.to_json(), text, "serialization is stable");
    }

    #[test]
    fn rejects_foreign_and_future_files() {
        assert!(matches!(
            Project::from_json("{}"),
            Err(LoadError::NotAProject)
        ));
        let future = format!(r#"{{"format":"{FORMAT}","version":99}}"#);
        assert!(matches!(
            Project::from_json(&future),
            Err(LoadError::TooNew(99))
        ));
        assert!(matches!(
            Project::from_json("not json"),
            Err(LoadError::Json(_))
        ));
    }

    #[test]
    fn ids_are_repaired_after_load() {
        let mut p = Project::new("Demo");
        p.ids = IdGen::default();
        let q = Project::from_json(&p.to_json()).unwrap();
        let mut q2 = q.clone();
        let id = q2.next_id();
        assert!(id > q.boards[0].pcb.max_id());
    }

    #[test]
    fn new_project_has_board_outline() {
        let p = Project::new("x");
        assert!(p.boards[0].pcb.outline().is_some());
        let mut p = p;
        let b = p.add_board("Second");
        assert!(p.board(b).unwrap().pcb.outline().is_some());
        assert!(p.add_sheet(b).is_some());
    }
}

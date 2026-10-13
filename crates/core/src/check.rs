//! Shared result type for electrical and design rule checks.

use crate::geom::Point;
use crate::id::Id;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "Error",
            Severity::Warning => "Warning",
            Severity::Info => "Info",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Issue {
    pub severity: Severity,
    pub message: String,
    /// Sheet index for schematic issues.
    pub sheet: Option<usize>,
    /// Where to zoom when the issue is clicked.
    pub pos: Option<Point>,
    pub objects: Vec<Id>,
}

impl Issue {
    pub fn new(severity: Severity, message: impl Into<String>) -> Issue {
        Issue {
            severity,
            message: message.into(),
            sheet: None,
            pos: None,
            objects: vec![],
        }
    }

    pub fn at(mut self, pos: Point) -> Issue {
        self.pos = Some(pos);
        self
    }

    pub fn on_sheet(mut self, sheet: usize) -> Issue {
        self.sheet = Some(sheet);
        self
    }

    pub fn with(mut self, id: Id) -> Issue {
        self.objects.push(id);
        self
    }
}

/// Sort issues: errors first, then by message for stable output.
pub fn sort(issues: &mut [Issue]) {
    issues.sort_by(|a, b| a.severity.cmp(&b.severity).then(a.message.cmp(&b.message)));
}

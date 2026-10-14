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

/// Plain-text report of a check run, one issue per line with its location,
/// for saving next to a design or pasting into a review.
pub fn report(title: &str, issues: &[Issue], unit: crate::units::Unit) -> String {
    use std::fmt::Write;
    let count = |s: Severity| issues.iter().filter(|i| i.severity == s).count();
    let mut out = format!(
        "{title}\n{} error(s), {} warning(s), {} info\n\n",
        count(Severity::Error),
        count(Severity::Warning),
        count(Severity::Info)
    );
    if issues.is_empty() {
        out.push_str("No problems found.\n");
    }
    for i in issues {
        let _ = write!(out, "{:<8} {}", i.severity.label(), i.message);
        if let Some(s) = i.sheet {
            let _ = write!(out, "  [sheet {}]", s + 1);
        }
        if let Some(p) = i.pos {
            let _ = write!(
                out,
                "  @ ({}, {}) {}",
                unit.format(p.x),
                unit.format(p.y),
                unit.label()
            );
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{mm, Unit};

    #[test]
    fn report_lists_issues_with_locations() {
        let issues = vec![
            Issue::new(Severity::Error, "Short").at(Point::new(mm(1.5), mm(-2.0))),
            Issue::new(Severity::Warning, "Pin U1.3 is not connected").on_sheet(0),
        ];
        let r = report("DRC: Board1", &issues, Unit::Mm);
        let lines: Vec<_> = r.lines().collect();
        assert_eq!(lines[0], "DRC: Board1");
        assert_eq!(lines[1], "1 error(s), 1 warning(s), 0 info");
        assert_eq!(lines[3], "Error    Short  @ (1.5, -2) mm");
        assert_eq!(lines[4], "Warning  Pin U1.3 is not connected  [sheet 1]");
        assert!(report("ERC", &[], Unit::Mm).contains("No problems found."));
    }
}

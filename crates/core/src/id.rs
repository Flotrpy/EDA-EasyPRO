//! Stable identifiers for document objects.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Id(pub u64);

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Hands out ids that are unique within one project.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IdGen {
    next: u64,
}

impl IdGen {
    pub fn next(&mut self) -> Id {
        self.next += 1;
        Id(self.next)
    }

    /// Make sure future ids are above `id` (used after loading files).
    pub fn bump(&mut self, id: Id) {
        self.next = self.next.max(id.0);
    }
}

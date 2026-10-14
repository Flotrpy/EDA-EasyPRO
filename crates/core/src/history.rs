//! Undo/redo by snapshots.
//!
//! Projects are small enough that cloning per edit is cheap, and snapshots
//! can never get out of sync with the document the way inverse commands can.

#[derive(Clone, Debug)]
pub struct History<T: Clone> {
    undo: Vec<(String, T)>,
    redo: Vec<(String, T)>,
    limit: usize,
}

impl<T: Clone> Default for History<T> {
    fn default() -> Self {
        History::new(200)
    }
}

impl<T: Clone> History<T> {
    pub fn new(limit: usize) -> Self {
        History {
            undo: Vec::new(),
            redo: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// Record `before` (the state prior to an edit described by `label`).
    pub fn push(&mut self, label: impl Into<String>, before: T) {
        self.undo.push((label.into(), before));
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Swap `current` with the last snapshot. Returns the undone label.
    pub fn undo(&mut self, current: &mut T) -> Option<String> {
        let (label, prev) = self.undo.pop()?;
        let now = std::mem::replace(current, prev);
        self.redo.push((label.clone(), now));
        Some(label)
    }

    pub fn redo(&mut self, current: &mut T) -> Option<String> {
        let (label, next) = self.redo.pop()?;
        let now = std::mem::replace(current, next);
        self.undo.push((label.clone(), now));
        Some(label)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|(l, _)| l.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|(l, _)| l.as_str())
    }

    /// Undo step labels, most recent first.
    pub fn undo_labels(&self) -> Vec<String> {
        self.undo.iter().rev().map(|(l, _)| l.clone()).collect()
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_redo_cycle() {
        let mut h = History::new(10);
        let mut doc = 1;
        h.push("set 2", doc);
        doc = 2;
        h.push("set 3", doc);
        doc = 3;
        assert_eq!(h.undo(&mut doc).as_deref(), Some("set 3"));
        assert_eq!(doc, 2);
        assert_eq!(h.undo(&mut doc).as_deref(), Some("set 2"));
        assert_eq!(doc, 1);
        assert!(h.undo(&mut doc).is_none());
        assert_eq!(h.redo(&mut doc).as_deref(), Some("set 2"));
        assert_eq!(doc, 2);
    }

    #[test]
    fn labels_are_newest_first() {
        let mut h = History::new(5);
        let mut doc = 0;
        h.push("a", doc);
        doc = 1;
        h.push("b", doc);
        assert_eq!(h.undo_labels(), vec!["b", "a"]);
        h.undo(&mut doc);
        assert_eq!(h.undo_labels(), vec!["a"]);
    }

    #[test]
    fn new_edit_clears_redo_and_limit_applies() {
        let mut h = History::new(2);
        let mut doc = 0;
        for i in 1..=5 {
            h.push(format!("{i}"), doc);
            doc = i;
        }
        assert_eq!(h.undo(&mut doc).as_deref(), Some("5"));
        h.push("x", doc);
        assert!(!h.can_redo());
        assert_eq!(h.undo(&mut doc).as_deref(), Some("x"));
        assert_eq!(h.undo(&mut doc).as_deref(), Some("4"));
        assert!(!h.can_undo());
    }
}

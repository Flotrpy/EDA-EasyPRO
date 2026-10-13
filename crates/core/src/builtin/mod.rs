//! The library that ships with EasyPRO. Everything here is generated in
//! code, so it is versioned and reviewed like the rest of the program.

mod devices;
pub mod footprints;
pub mod symbols;

use crate::library::Library;
use std::sync::OnceLock;

/// The built-in library (built once, then shared).
pub fn library() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let mut lib = Library::default();
        for s in devices::symbols() {
            lib.add_symbol(s);
        }
        for f in footprints::all() {
            lib.add_footprint(f);
        }
        for d in devices::all() {
            lib.add_device(d);
        }
        lib
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_is_consistent() {
        let lib = library();
        let problems = lib.validate();
        assert!(problems.is_empty(), "{problems:#?}");
        assert!(
            lib.devices.len() >= 40,
            "only {} devices",
            lib.devices.len()
        );
    }

    #[test]
    fn search_finds_parts() {
        let lib = library();
        assert!(lib
            .search("0603 resistor")
            .iter()
            .any(|d| d.footprint.as_deref() == Some("fp:0603")));
        assert!(!lib.search("esp32").is_empty());
        assert!(lib.search("definitely-not-a-part").is_empty());
    }
}

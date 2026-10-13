//! Write an example project's fabrication package to disk:
//! `cargo run -p easypro-export --example fab_package -- out.zip [jlcpcb|pcbway|oshpark|eurocircuits|generic] [example]`
//!
//! `example` is a case-insensitive prefix of a bundled example name
//! ("555", "3.3v", ...); the first example is used when it is omitted.

use easypro_export::fab::{package_zip, Fab};

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| "blinker-fab.zip".into());
    let fab = match args.next().as_deref() {
        Some("pcbway") => Fab::Pcbway,
        Some("oshpark") => Fab::Oshpark,
        Some("generic") => Fab::Generic,
        Some("eurocircuits") => Fab::Eurocircuits,
        _ => Fab::Jlcpcb,
    };
    let want = args.next().unwrap_or_default().to_lowercase();
    let examples = easypro_core::examples::all();
    let Some((name, make)) = examples
        .iter()
        .find(|(n, _)| n.to_lowercase().starts_with(&want))
    else {
        let names: Vec<_> = examples.iter().map(|(n, _)| *n).collect();
        eprintln!("unknown example {want:?}; choose one of {names:?}");
        std::process::exit(2);
    };
    let p = make();
    let stem: String = p
        .name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let bytes = package_zip(&p.boards[0], &p.library, fab, &stem);
    std::fs::write(&out, bytes).expect("write zip");
    println!("wrote {out}: {name} for {}", fab.label());
}

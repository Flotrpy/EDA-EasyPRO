//! Write the example project's fabrication package to disk:
//! `cargo run -p easypro-export --example fab_package -- out.zip [jlcpcb|pcbway|oshpark|generic]`

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
    let p = easypro_core::examples::blinker();
    let bytes = package_zip(&p.boards[0], &p.library, fab, "blinker");
    std::fs::write(&out, bytes).expect("write zip");
    println!("wrote {out} for {}", fab.label());
}

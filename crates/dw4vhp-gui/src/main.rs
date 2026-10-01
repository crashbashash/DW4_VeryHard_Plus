//! The `dw4-veryhard-plus` binary.
//!
//! Handles `--version` and `--help` itself — `eframe` provides no CLI — and
//! then starts the window from [`dw4vhp_gui::app::App`].

use eframe::egui;

fn main() -> eframe::Result {
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("dw4-veryhard-plus {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            _ => {}
        }
    }

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([760.0, 720.0])
        .with_title("DW4 Very Hard Plus");

    // The icon is the committed, generated `icons/icon.png`; the decode cannot
    // fail in practice, but a bad image must not stop the window from opening.
    let viewport = match eframe::icon_data::from_png_bytes(include_bytes!("../icons/icon.png")) {
        Ok(icon) => viewport.with_icon(icon),
        Err(_) => viewport,
    };

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "dw4-veryhard-plus",
        options,
        Box::new(|_cc| Ok(Box::new(dw4vhp_gui::app::App::new()))),
    )
}

fn print_help() {
    println!(
        "dw4-veryhard-plus {} — patch a Digimon World 4 (USA) ISO to Very Hard Plus",
        env!("CARGO_PKG_VERSION")
    );
    println!("Usage: dw4-veryhard-plus [OPTIONS]");
    println!("  -V, --version  print the version and exit");
    println!("  -h, --help     print this help and exit");
}

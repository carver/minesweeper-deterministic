use eframe::egui;
use minesweeper_deterministic::ui::{App, open_journal};

const ICON: &[u8] = include_bytes!("../assets/icon.png");

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Deterministic Minesweeper")
        .with_app_id("minesweeper-deterministic")
        .with_inner_size([780.0, 480.0])
        .with_min_inner_size([420.0, 260.0]);
    match eframe::icon_data::from_png_bytes(ICON) {
        Ok(icon) => viewport = viewport.with_icon(icon),
        Err(e) => eprintln!("window icon unavailable: {e}"),
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Deterministic Minesweeper",
        options,
        Box::new(|cc| Ok(Box::new(App::restore(cc.storage, open_journal())))),
    )
}

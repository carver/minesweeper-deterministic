use eframe::egui;
use minesweeper_deterministic::ui::App;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Deterministic Minesweeper")
            .with_inner_size([780.0, 480.0])
            .with_min_inner_size([420.0, 260.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Deterministic Minesweeper",
        options,
        Box::new(|_| Ok(Box::<App>::default())),
    )
}

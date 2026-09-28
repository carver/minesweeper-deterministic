use eframe::egui;
use std::time::Instant;

use minesweeper_deterministic::ui::{App, Startup, open_journal};

const ICON: &[u8] = include_bytes!("../assets/icon.png");
/// Set to `wgpu` to draw with wgpu instead of OpenGL.
const RENDERER_VAR: &str = "MINESWEEPER_RENDERER";

fn renderer() -> eframe::Renderer {
    match std::env::var(RENDERER_VAR).as_deref() {
        Ok("wgpu") => eframe::Renderer::Wgpu,
        Ok("glow") | Err(_) => eframe::Renderer::Glow,
        Ok(other) => {
            eprintln!("{RENDERER_VAR}={other} not recognised, using glow");
            eframe::Renderer::Glow
        }
    }
}

fn main() -> eframe::Result {
    let launched = Instant::now();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Deterministic Minesweeper")
        .with_app_id("minesweeper-deterministic")
        .with_inner_size([780.0, 480.0])
        .with_min_inner_size([420.0, 260.0]);
    match eframe::icon_data::from_png_bytes(ICON) {
        Ok(icon) => viewport = viewport.with_icon(icon),
        Err(e) => eprintln!("window icon unavailable: {e}"),
    }
    let renderer = renderer();
    eprintln!("renderer: {renderer:?}");
    let options = eframe::NativeOptions {
        viewport,
        renderer,
        ..Default::default()
    };
    eframe::run_native(
        "Deterministic Minesweeper",
        options,
        Box::new(move |cc| {
            let startup = Startup {
                launched,
                renderer_ready: Instant::now(),
            };
            let app = App::restore(cc.storage, open_journal()).with_startup(startup);
            Ok(Box::new(app))
        }),
    )
}

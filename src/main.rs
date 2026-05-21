#![windows_subsystem = "windows"]

mod utils;

use utils::app::App;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Insomni Player")
            .with_inner_size([680.0, 520.0])
            .with_min_inner_size([480.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "insomni player",
        options,
        Box::new(|_cc| Box::new(App::new())),
    )
}

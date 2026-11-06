mod app;
mod canvas;
mod command_palette;
mod panels;
pub mod screenshot;
mod theme;
mod workspace;

fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--screenshot") {
        if let Some(path) = args.get(pos + 1) {
            match screenshot::render_ui_screenshot(std::path::Path::new(path)) {
                Ok(_) => {
                    println!("Screenshot saved to {}", path);
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("Screenshot error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("VDraw")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([800.0, 600.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "VDraw",
        native_options,
        Box::new(|cc| Ok(Box::new(app::VDrawApp::new(cc)))),
    )
}

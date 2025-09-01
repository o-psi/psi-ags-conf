mod config;
mod data;
mod drawing;
mod ui;

use gtk4::prelude::*;
use gtk4::Application;
use config::GraphConfig;

fn main() {
    eprintln!("Starting graph window...");
    let args: Vec<String> = std::env::args().collect();
    eprintln!("Args: {:?}", args);

    let config = if args.len() > 1 {
        serde_json::from_str(&args[1]).unwrap_or_else(|e| {
            eprintln!("Failed to parse JSON config: {}", e);
            GraphConfig::default()
        })
    } else {
        GraphConfig::default()
    };

    let app_id = format!("com.example.graphwindow.{}", std::process::id());
    let app = Application::builder()
        .application_id(&app_id)
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_activate(move |app| {
        ui::build_ui(app, config.clone());
    });

    let empty_args: Vec<String> = vec![];
    app.run_with_args(&empty_args);
}
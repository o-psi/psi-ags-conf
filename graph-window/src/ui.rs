use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, DrawingArea, Label, Box, Orientation};
use gtk4_layer_shell::{LayerShell, Layer, Edge};
use gtk4::gdk::{Key};
use glib::{timeout_add_local, ControlFlow};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::config::GraphConfig;
use crate::data::{self, GraphData, AdvancedMemoryData};
use crate::drawing;

pub fn build_ui(app: &Application, config: GraphConfig) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title(&config.title)
        .default_width(config.width)
        .default_height(config.height + 50)
        .build();
    
    window.set_decorated(false);
    window.set_resizable(false);
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);

    let key_controller = gtk4::EventControllerKey::new();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == Key::Escape {
            std::process::exit(0);
        }
        glib::Propagation::Proceed
    });
    window.add_controller(key_controller);

    let click_controller = gtk4::GestureClick::new();
    click_controller.set_button(3);
    click_controller.connect_pressed(move |_, _, _, _| {
        std::process::exit(0);
    });
    window.add_controller(click_controller);

    if config.position_x > 0 || config.position_y > 0 {
        let offset_x = config.position_x + 10;
        let offset_y = config.position_y + 10;
        
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Bottom, false);
        window.set_anchor(Edge::Right, false);
        
        let display = gtk4::prelude::WidgetExt::display(&window);
        if let Some(surface) = window.surface() {
            if let Some(monitor) = display.monitor_at_surface(&surface) {
                let geometry = monitor.geometry();
                let max_x = geometry.x() + geometry.width() - config.width;
                let max_y = geometry.y() + geometry.height() - config.height - 50;
                
                let final_x = offset_x.min(max_x).max(0);
                let final_y = offset_y.min(max_y).max(0);
                
                window.set_margin(Edge::Top, final_y);
                window.set_margin(Edge::Left, final_x);
            }
        }
    }
    
    let vbox = Box::new(Orientation::Vertical, 8);
    vbox.set_margin_top(8);
    vbox.set_margin_bottom(8);
    vbox.set_margin_start(8);
    vbox.set_margin_end(8);
    
    let title_box = Box::new(Orientation::Horizontal, 8);
    
    let title_label = Label::new(Some(&config.title));
    title_label.set_css_classes(&["title-label"]);
    title_label.set_hexpand(true);
    title_label.set_halign(gtk4::Align::Start);
    
    let close_button = gtk4::Button::new();
    close_button.set_label("✕");
    close_button.set_css_classes(&["close-button"]);
    close_button.connect_clicked(|_| {
        std::process::exit(0);
    });
    
    title_box.append(&title_label);
    title_box.append(&close_button);
    
    let drawing_area = DrawingArea::builder()
        .width_request(config.width)
        .height_request(config.height)
        .build();
    
    let stats_label = Label::new(Some("Initializing..."));
    stats_label.set_css_classes(&["stats-label"]);
    
    let history = data::load_history();
    
    let advanced_mem_data = Arc::new(Mutex::new(AdvancedMemoryData::new(60)));
    let graph_data = Arc::new(Mutex::new(GraphData::new_with_zeros(60)));
    let graph_data2 = Arc::new(Mutex::new(GraphData::new_with_zeros(60)));
    let cpu_core_data = Arc::new(Mutex::new(vec![]));
    let iowait_data = Arc::new(Mutex::new(GraphData::new_with_zeros(60)));

    if config.data_source == "memory" && config.advanced {
        let mut mem_data = advanced_mem_data.lock().unwrap();
        if let Some(total) = history["memory"]["total"].as_f64() {
            mem_data.total = total;
        }
        if let Some(apps) = history["memory_apps"].as_array() {
            mem_data.apps.values = apps.iter().filter_map(|v| v.as_f64()).collect();
        }
        if let Some(cached) = history["memory_cached"].as_array() {
            mem_data.cached.values = cached.iter().filter_map(|v| v.as_f64()).collect();
        }
        if let Some(buffers) = history["memory_buffers"].as_array() {
            mem_data.buffers.values = buffers.iter().filter_map(|v| v.as_f64()).collect();
        }
        if let Some(slab) = history["memory_slab"].as_array() {
            mem_data.slab.values = slab.iter().filter_map(|v| v.as_f64()).collect();
        }
        if let Some(shmem) = history["memory_shmem"].as_array() {
            mem_data.shmem.values = shmem.iter().filter_map(|v| v.as_f64()).collect();
        }
    } else {
        // Load data for other charts
    }

    let config_draw = config.clone();
    let advanced_mem_data_draw = advanced_mem_data.clone();
    let graph_data_draw = graph_data.clone();
    let graph_data2_draw = graph_data2.clone();
    let cpu_cores_draw = cpu_core_data.clone();
    let iowait_draw = iowait_data.clone();

    drawing_area.set_draw_func(move |_, cr, width, height| {
        if config_draw.data_source == "memory" && config_draw.advanced {
            let mem_data = advanced_mem_data_draw.lock().unwrap();
            drawing::draw_advanced_memory_chart(cr, &mem_data, width as f64, height as f64);
        } else if config_draw.data_source == "cpu" && config_draw.multi_chart {
            let cores = cpu_cores_draw.lock().unwrap();
            let iowait = iowait_draw.lock().unwrap();
            drawing::draw_multi_cpu_charts(cr, &cores, &iowait, &config_draw, width as f64, height as f64);
        } else {
            let data = graph_data_draw.lock().unwrap();
            let data2 = graph_data2_draw.lock().unwrap();
            drawing::draw_graph(cr, &data, Some(&data2), &config_draw, width as f64, height as f64);
        }
    });

    let config_update = config.clone();
    let advanced_mem_data_update = advanced_mem_data.clone();
    let stats_label_update = stats_label.clone();
    let drawing_area_update = drawing_area.clone();

    timeout_add_local(Duration::from_millis(1000), move || {
        let history = data::load_history();
        if config_update.data_source == "memory" && config_update.advanced {
            let mut mem_data = advanced_mem_data_update.lock().unwrap();
            if let Some(total) = history["memory"]["total"].as_f64() {
                mem_data.total = total;
            }
            if let Some(apps) = history["memory_apps"].as_array() {
                mem_data.apps.values = apps.iter().filter_map(|v| v.as_f64()).collect();
            }
            if let Some(cached) = history["memory_cached"].as_array() {
                mem_data.cached.values = cached.iter().filter_map(|v| v.as_f64()).collect();
            }
            if let Some(buffers) = history["memory_buffers"].as_array() {
                mem_data.buffers.values = buffers.iter().filter_map(|v| v.as_f64()).collect();
            }
            if let Some(slab) = history["memory_slab"].as_array() {
                mem_data.slab.values = slab.iter().filter_map(|v| v.as_f64()).collect();
            }
            if let Some(shmem) = history["memory_shmem"].as_array() {
                mem_data.shmem.values = shmem.iter().filter_map(|v| v.as_f64()).collect();
            }
            let apps = mem_data.apps.values.last().unwrap_or(&0.0) / 1024.0;
            let cached = mem_data.cached.values.last().unwrap_or(&0.0) / 1024.0;
            let buffers = mem_data.buffers.values.last().unwrap_or(&0.0) / 1024.0;
            let slab = mem_data.slab.values.last().unwrap_or(&0.0) / 1024.0;
            let shmem = mem_data.shmem.values.last().unwrap_or(&0.0) / 1024.0;
            stats_label_update.set_text(&format!(
                "Apps: {:.1}MB | Cached: {:.1}MB | Buffers: {:.1}MB | Slab: {:.1}MB | Shmem: {:.1}MB",
                apps, cached, buffers, slab, shmem
            ));
        } else {
            // Update other charts
        }
        drawing_area_update.queue_draw();
        ControlFlow::Continue
    });
    
    vbox.append(&title_box);
    vbox.append(&drawing_area);
    vbox.append(&stats_label);
    
    window.set_child(Some(&vbox));
    
    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_string(r#"
        window {
            background-color: #1e1e2e;
            border-radius: 12px;
            border: 1px solid #313244;
            box-shadow: 0 8px 16px rgba(0, 0, 0, 0.8);
        }
        
        .title-label {
            color: #cdd6f4;
            font-size: 14px;
            font-weight: bold;
            font-family: sans-serif;
        }
        
        .stats-label {
            color: #a6adc8;
            font-size: 11px;
            font-family: monospace;
        }
        
        .legend-label {
            color: #bac2de;
            font-size: 10px;
            font-family: sans-serif;
        }
        
        .close-button {
            background: none;
            border: none;
            color: #f38ba8;
            font-size: 16px;
            font-weight: bold;
            padding: 0;
            min-width: 20px;
            min-height: 20px;
        }
        
        .close-button:hover {
            color: #f5c2e7;
            background-color: rgba(243, 139, 168, 0.2);
            border-radius: 4px;
        }
    "#);
    
    gtk4::style_context_add_provider_for_display(
        &gtk4::prelude::WidgetExt::display(&window),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    
    window.present();
}
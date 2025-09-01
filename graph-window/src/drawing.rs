use cairo::Context;
use crate::data::{GraphData, AdvancedMemoryData};
use crate::config::GraphConfig;

pub fn parse_color(color: &str) -> (f64, f64, f64) {
    if color.starts_with('#') && color.len() == 7 {
        let r = u8::from_str_radix(&color[1..3], 16).unwrap_or(128) as f64 / 255.0;
        let g = u8::from_str_radix(&color[3..5], 16).unwrap_or(128) as f64 / 255.0;
        let b = u8::from_str_radix(&color[5..7], 16).unwrap_or(128) as f64 / 255.0;
        (r, g, b)
    } else {
        (0.5, 0.5, 1.0)
    }
}

pub fn draw_advanced_memory_chart(cr: &Context, mem_data: &AdvancedMemoryData, width: f64, height: f64) {
    cr.set_source_rgba(0.118, 0.118, 0.180, 0.9);
    cr.rectangle(0.0, 0.0, width, height);
    cr.fill().unwrap();

    let data_points = mem_data.apps.values.len();
    if data_points == 0 { return; }

    let categories = [
        ("Apps", &mem_data.apps, "#f38ba8"),
        ("Cached", &mem_data.cached, "#a6e3a1"),
        ("Buffers", &mem_data.buffers, "#89b4fa"),
        ("Slab", &mem_data.slab, "#f9e2af"),
        ("Shmem", &mem_data.shmem, "#cba6f7"),
    ];

    let max_value = mem_data.total;
    if max_value == 0.0 { return; }

    let mut cumulative_values = vec![0.0; data_points];

    for (_name, data, color_str) in categories.iter() {
        let (r, g, b) = parse_color(color_str);
        cr.set_source_rgba(r, g, b, 0.7);

        cr.move_to(0.0, height);
        for i in 0..data_points {
            let x = (i as f64 / (data_points - 1).max(1) as f64) * width;
            let y = height - (cumulative_values[i] / max_value).min(1.0) * height;
            cr.line_to(x, y);
        }

        for i in (0..data_points).rev() {
            let new_cumulative = cumulative_values[i] + data.values[i];
            let x = (i as f64 / (data_points - 1).max(1) as f64) * width;
            let y = height - (new_cumulative / max_value).min(1.0) * height;
            cr.line_to(x, y);
            cumulative_values[i] = new_cumulative;
        }
        cr.close_path();
        cr.fill().unwrap();
    }
}

pub fn draw_multi_cpu_charts(cr: &Context, cpu_data: &[GraphData], iowait_data: &GraphData, config: &GraphConfig, width: f64, height: f64) {
    cr.set_source_rgba(0.118, 0.118, 0.180, 0.9);
    cr.rectangle(0.0, 0.0, width, height);
    cr.fill().unwrap();
    
    let num_cores = cpu_data.len().min(16);
    let cols = 4;
    let rows = (num_cores + cols - 1) / cols;
    
    let chart_width = width / cols as f64;
    let chart_height = (height - 40.0) / (rows + 1) as f64;
    
    let core_colors = [
        "#89b4fa", "#94e2d5", "#89dceb", "#74c7ec",
        "#f9e2af", "#fab387", "#f38ba8", "#cba6f7",
        "#a6e3a1", "#f5c2e7", "#eba0ac", "#f2cdcd",
        "#b4befe", "#89b4fa", "#94e2d5", "#89dceb"
    ];
    
    for (i, core_data) in cpu_data.iter().enumerate().take(num_cores) {
        if core_data.values.is_empty() { continue; }
        
        let col = i % cols;
        let row = i / cols;
        let x_offset = col as f64 * chart_width;
        let y_offset = row as f64 * chart_height;
        
        let (r, g, b) = parse_color(core_colors[i % core_colors.len()]);
        
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.2);
        cr.rectangle(x_offset + 2.0, y_offset + 2.0, chart_width - 4.0, chart_height - 4.0);
        cr.fill().unwrap();
        
        cr.set_source_rgba(0.8, 0.8, 0.9, 1.0);
        cr.move_to(x_offset + 4.0, y_offset + 15.0);
        cr.show_text(&format!("C{}", i)).unwrap();
        
        cr.set_source_rgba(r, g, b, 0.3);
        let mini_width = chart_width - 8.0;
        let mini_height = chart_height - 20.0;
        
        cr.move_to(x_offset + 4.0, y_offset + chart_height - 4.0);
        
        for (j, value) in core_data.values.iter().enumerate() {
            let x = x_offset + 4.0 + (j as f64 / (core_data.values.len() - 1).max(1) as f64) * mini_width;
            let y = y_offset + chart_height - 4.0 - (value / config.max_value).min(1.0) * mini_height;
            cr.line_to(x, y);
        }
        
        cr.line_to(x_offset + 4.0 + mini_width, y_offset + chart_height - 4.0);
        cr.close_path();
        cr.fill().unwrap();
        
        cr.set_source_rgba(r, g, b, 1.0);
        cr.set_line_width(1.0);
        
        for (j, value) in core_data.values.iter().enumerate() {
            let x = x_offset + 4.0 + (j as f64 / (core_data.values.len() - 1).max(1) as f64) * mini_width;
            let y = y_offset + chart_height - 4.0 - (value / config.max_value).min(1.0) * mini_height;
            
            if j == 0 {
                cr.move_to(x, y);
            } else {
                cr.line_to(x, y);
            }
        }
        cr.stroke().unwrap();
        
        let current = core_data.values.last().copied().unwrap_or(0.0);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
        cr.move_to(x_offset + chart_width - 30.0, y_offset + chart_height - 8.0);
        cr.show_text(&format!("{:.0}%", current)).unwrap();
    }
    
    if !iowait_data.values.is_empty() {
        let iowait_y = rows as f64 * chart_height + 10.0;
        let iowait_height = chart_height - 20.0;
        
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.2);
        cr.rectangle(10.0, iowait_y, width - 20.0, iowait_height);
        cr.fill().unwrap();
        
        cr.set_source_rgba(0.8, 0.8, 0.9, 1.0);
        cr.move_to(15.0, iowait_y + 15.0);
        cr.show_text("IO Wait").unwrap();
        
        let (r, g, b) = parse_color("#f38ba8");
        cr.set_source_rgba(r, g, b, 0.3);
        
        cr.move_to(10.0, iowait_y + iowait_height);
        
        for (i, value) in iowait_data.values.iter().enumerate() {
            let x = 10.0 + (i as f64 / (iowait_data.values.len() - 1).max(1) as f64) * (width - 20.0);
            let y = iowait_y + iowait_height - (value / 10.0).min(1.0) * (iowait_height - 20.0);
            cr.line_to(x, y);
        }
        
        cr.line_to(width - 10.0, iowait_y + iowait_height);
        cr.close_path();
        cr.fill().unwrap();
        
        cr.set_source_rgba(r, g, b, 1.0);
        cr.set_line_width(1.5);
        
        for (i, value) in iowait_data.values.iter().enumerate() {
            let x = 10.0 + (i as f64 / (iowait_data.values.len() - 1).max(1) as f64) * (width - 20.0);
            let y = iowait_y + iowait_height - (value / 10.0).min(1.0) * (iowait_height - 20.0);
            
            if i == 0 {
                cr.move_to(x, y);
            } else {
                cr.line_to(x, y);
            }
        }
        cr.stroke().unwrap();
        
        let current = iowait_data.values.last().copied().unwrap_or(0.0);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
        cr.move_to(width - 50.0, iowait_y + iowait_height - 5.0);
        cr.show_text(&format!("{:.1}%", current)).unwrap();
    }
}

pub fn draw_graph(cr: &Context, data: &GraphData, data2: Option<&GraphData>, config: &GraphConfig, width: f64, height: f64) {
    cr.set_source_rgba(0.118, 0.118, 0.180, 0.9);
    cr.rectangle(0.0, 0.0, width, height);
    cr.fill().unwrap();
    
    cr.set_source_rgba(0.27, 0.28, 0.35, 0.3);
    cr.set_line_width(0.5);
    
    for i in 1..=4 {
        let y = (height / 4.0) * i as f64;
        cr.move_to(0.0, y);
        cr.line_to(width, y);
        cr.stroke().unwrap();
    }
    
    if data.values.is_empty() {
        return;
    }
    
    let (r, g, b) = parse_color(&config.color);
    
    cr.set_source_rgba(r, g, b, 0.2);
    cr.move_to(0.0, height);
    
    for (i, value) in data.values.iter().enumerate() {
        let x = (i as f64 / (data.values.len() - 1).max(1) as f64) * width;
        let y = height - (value / config.max_value).min(1.0) * height;
        cr.line_to(x, y);
    }
    
    cr.line_to(width, height);
    cr.close_path();
    cr.fill().unwrap();
    
    cr.set_source_rgba(r, g, b, 1.0);
    cr.set_line_width(2.0);
    
    for (i, value) in data.values.iter().enumerate() {
        let x = (i as f64 / (data.values.len() - 1).max(1) as f64) * width;
        let y = height - (value / config.max_value).min(1.0) * height;
        
        if i == 0 {
            cr.move_to(x, y);
        } else {
            cr.line_to(x, y);
        }
    }
    cr.stroke().unwrap();
    
    if let Some(data2) = data2 {
        if !data2.values.is_empty() {
            let (r2, g2, b2) = if !config.color2.is_empty() {
                parse_color(&config.color2)
            } else {
                (1.0, 0.5, 0.5)
            };
            
            cr.set_source_rgba(r2, g2, b2, 0.2);
            cr.move_to(0.0, height);
            
            for (i, value) in data2.values.iter().enumerate() {
                let x = (i as f64 / (data2.values.len() - 1).max(1) as f64) * width;
                let y = height - (value / config.max_value).min(1.0) * height;
                cr.line_to(x, y);
            }
            
            cr.line_to(width, height);
            cr.close_path();
            cr.fill().unwrap();
            
            cr.set_source_rgba(r2, g2, b2, 1.0);
            cr.set_line_width(2.0);
            
            for (i, value) in data2.values.iter().enumerate() {
                let x = (i as f64 / (data2.values.len() - 1).max(1) as f64) * width;
                let y = height - (value / config.max_value).min(1.0) * height;
                
                if i == 0 {
                    cr.move_to(x, y);
                } else {
                    cr.line_to(x, y);
                }
            }
            cr.stroke().unwrap();
        }
    }
}
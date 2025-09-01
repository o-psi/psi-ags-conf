use serde::Deserialize;
use std::env;
use std::fs;
use std::io::{self, Read};

#[derive(Debug, Deserialize)]
struct GraphRequest {
    data: Vec<f64>,
    max_value: f64,
    color: String,
    width: u32,
    height: u32,
    output_path: String,
}

fn generate_svg(req: &GraphRequest) -> String {
    let width = req.width;
    let height = req.height;
    let data = &req.data;
    let max_value = req.max_value;
    let color = &req.color;
    
    if data.len() < 2 {
        return format!(
            r##"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">
                <rect width="{}" height="{}" fill="#1e1e2e" opacity="0.3" rx="4"/>
            </svg>"##,
            width, height, width, height
        );
    }
    
    // Generate points for the line
    let points: Vec<String> = data
        .iter()
        .enumerate()
        .map(|(i, &value)| {
            let x = (i as f64 / (data.len() - 1) as f64) * width as f64;
            let y = height as f64 - (value.min(max_value) / max_value) * height as f64;
            format!("{:.2},{:.2}", x, y)
        })
        .collect();
    
    let line_points = points.join(" ");
    
    // Create fill polygon points
    let mut fill_points = format!("0,{} ", height);
    fill_points.push_str(&line_points);
    fill_points.push_str(&format!(" {},{}",width, height));
    
    // Generate grid lines
    let mut grid_lines = String::new();
    for i in 1..=4 {
        let y = (height as f64 / 4.0) * i as f64;
        grid_lines.push_str(&format!(
            r##"<line x1="0" y1="{:.0}" x2="{}" y2="{:.0}" stroke="#45475a" stroke-width="0.5" opacity="0.3"/>"##,
            y, width, y
        ));
    }
    
    format!(
        r##"<svg width="{}" height="{}" xmlns="http://www.w3.org/2000/svg">
            <rect width="{}" height="{}" fill="#1e1e2e" opacity="0.3" rx="4"/>
            {}
            <polygon points="{}" fill="{}" opacity="0.2"/>
            <polyline points="{}" fill="none" stroke="{}" stroke-width="2" stroke-linejoin="round"/>
        </svg>"##,
        width, height, width, height, grid_lines, fill_points, color, line_points, color
    )
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    
    let json_input = if args.len() > 1 {
        // Read from command line argument
        args[1].clone()
    } else {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    };
    
    let request: GraphRequest = serde_json::from_str(&json_input)
        .expect("Failed to parse JSON input");
    
    let svg = generate_svg(&request);
    
    fs::write(&request.output_path, svg)
        .expect("Failed to write SVG file");
    
    println!("{}", request.output_path);
    
    Ok(())
}

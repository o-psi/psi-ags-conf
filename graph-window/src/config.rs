use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GraphConfig {
    pub title: String,
    pub color: String,
    #[serde(default)]
    pub color2: String,
    pub max_value: f64,
    pub width: i32,
    pub height: i32,
    pub data_source: String,
    #[serde(default)]
    pub initial_data: Vec<f64>,
    #[serde(default)]
    pub initial_data2: Vec<f64>,
    #[serde(default)]
    pub position_x: i32,
    #[serde(default)]
    pub position_y: i32,
    #[serde(default)]
    pub multi_chart: bool,
    #[serde(default)]
    pub advanced: bool,
}

impl Default for GraphConfig {
    fn default() -> Self {
        GraphConfig {
            title: "System Graph".to_string(),
            color: "#89b4fa".to_string(),
            color2: String::new(),
            max_value: 100.0,
            width: 300,
            height: 100,
            data_source: "cpu".to_string(),
            initial_data: vec![],
            initial_data2: vec![],
            position_x: 0,
            position_y: 0,
            multi_chart: false,
            advanced: false,
        }
    }
}

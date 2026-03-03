use macroquad::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use crate::model::*;

const IMAGE_FILES: [&str; Metal::COUNT] = [
    "quicksilver_symbol.png",
    "lead_symbol.png",
    "tin_symbol.png",
    "iron_symbol.png",
    "copper_symbol.png",
    "silver_symbol.png",
    "gold_symbol.png",
];

const METAL_LABELS: [&str; Metal::COUNT] = ["QS", "Pb", "Sn", "Fe", "Cu", "Ag", "Au"];

const DEFAULT_WINDOW_WIDTH: i32 = 1920;
const DEFAULT_WINDOW_HEIGHT: i32 = 1080;
const DEFAULT_SAMPLE_COUNT: i32 = 4;

const OUTER_MARGIN_X: f32 = 30.0;
const OUTER_MARGIN_TOP: f32 = 30.0;
const OUTER_MARGIN_BOTTOM: f32 = 120.0;
const LABEL_COLUMN_WIDTH: f32 = 190.0;

const OUTPUT_RATIO_GAP_Y: f32 = 18.0;
const OUTPUT_RATIO_HEIGHT: f32 = 80.0;

const GRID_LABEL_BORDER_THICKNESS: f32 = 1.5;
const GRID_CELL_BORDER_THICKNESS: f32 = 1.0;
const OUTPUT_RATIO_BORDER_THICKNESS: f32 = 2.0;

const ICON_ROW_HEIGHT_FACTOR: f32 = 0.9;
const ICON_COL_WIDTH_FACTOR: f32 = 0.9;

const TEXT_MARGIN_X: f32 = 0.15;
const TEXT_MARGIN_Y: f32 = 0.05;

const COLOR_BACKGROUND: Color = Color::new(0.08, 0.09, 0.12, 1.0);
const COLOR_GRID_BG: Color = Color::new(0.11, 0.12, 0.15, 1.0);
const COLOR_OUTPUT_RATIO_BG: Color = Color::new(0.13, 0.16, 0.19, 1.0);
const COLOR_OUTPUT_RATIO_BORDER: Color = Color::new(0.45, 0.55, 0.66, 1.0);
const COLOR_OUTPUT_TEXT: Color = Color::new(0.78, 0.91, 0.92, 1.0);
const COLOR_TRANSITION_TEXT: Color = Color::new(0.95, 0.91, 0.8, 1.0);

const ROW_LABELS: [&str; 8] = [
    "Metal",
    "Input",
    "Requested",
    "Output",
    "Projection",
    "Rejection",
    "Purification",
    "Deposition",
];

pub fn window_conf() -> Conf {
    Conf {
        window_title: "Metal Solver".to_string(),
        window_width: DEFAULT_WINDOW_WIDTH,
        window_height: DEFAULT_WINDOW_HEIGHT,
        high_dpi: true,
        sample_count: DEFAULT_SAMPLE_COUNT,
        ..Default::default()
    }
}

impl UI {
    pub async fn load_textures(&mut self) {
        let mut textures: [Option<Texture2D>; Metal::COUNT] = std::array::from_fn(|_| None);

            for idx in 0..Metal::COUNT {
                let path = format!("src/sprites/{}", IMAGE_FILES[idx]);
                if let Ok(texture) = load_texture(&path).await {
                    texture.set_filter(FilterMode::Linear);
                    textures[idx] = Some(texture);
                }
            }
            self.textures = textures.iter().filter_map(|t| t.clone()).collect();
    }

    fn format_integer(&self, value: f64) -> String {
        if value.abs() < 1e-9 {
            "0".to_string()
        } else {
            format!("{}", value.round() as i64)
        }
    }

    fn format_fraction(&self, value: f64) -> String {
        if value.abs() < 1e-9 {
            "0".to_string()
        } else {
            decimal_to_fraction(value)
        }
    }

    fn transition_value(&self, solution: &Option<OptimalSolution>, row_idx: usize, metal_idx: usize) -> String {
        let Some(solution) = solution else {
            return "-".to_string();
        };

        let value = match row_idx {
            4 if (1..=5).contains(&metal_idx) => Some(solution.projection[metal_idx - 1]),
            5 if (2..=6).contains(&metal_idx) => Some(solution.rejection[metal_idx - 2]),
            6 if (1..=5).contains(&metal_idx) => Some(solution.purification[metal_idx - 1]),
            7 if (2..=6).contains(&metal_idx) => Some(solution.deposition[metal_idx - 2]),
            _ => None,
        };

        value.map(|v| self.format_fraction(v)).unwrap_or_else(|| "".to_string())
    }

    fn draw_text_in_rect(&self, text: &str, x: f32, y: f32, w: f32, h: f32, color: Color) {
        if text.is_empty() {
            return;
        }
        let w = (1.0 - (2.0 * TEXT_MARGIN_X)) * w;
        let h = (1.0 - (2.0 * TEXT_MARGIN_Y)) * h;
        let x = x + w * TEXT_MARGIN_X;
        let y = y + h * TEXT_MARGIN_Y;

        let (optimal_size, offset_x, offset_y) = self.text_renderer.get_text_max_size(text, w, h);
        draw_text(text, x + offset_x, y + offset_y, optimal_size, color);
    }

    pub fn draw(&self) {
        let total_grid_width = screen_width() - OUTER_MARGIN_X * 2.0;
        let total_grid_height = screen_height() - OUTER_MARGIN_TOP - OUTER_MARGIN_BOTTOM;

        let cell_w = (total_grid_width - LABEL_COLUMN_WIDTH) / Metal::COUNT as f32;
        let rows = ROW_LABELS.len();
        let row_h = total_grid_height / rows as f32;

        draw_rectangle(
            OUTER_MARGIN_X,
            OUTER_MARGIN_TOP,
            total_grid_width,
            total_grid_height,
            COLOR_GRID_BG,
        );

        for (row_idx, row_label) in ROW_LABELS.iter().enumerate() {
            let y = OUTER_MARGIN_TOP + row_idx as f32 * row_h;

            draw_rectangle_lines(OUTER_MARGIN_X, y, LABEL_COLUMN_WIDTH, row_h, GRID_LABEL_BORDER_THICKNESS, GRAY);
            self.draw_text_in_rect(row_label, OUTER_MARGIN_X, y, LABEL_COLUMN_WIDTH, row_h, WHITE);

            for metal_idx in 0..Metal::COUNT {
                let x = OUTER_MARGIN_X + LABEL_COLUMN_WIDTH + metal_idx as f32 * cell_w;
                draw_rectangle_lines(x, y, cell_w, row_h, GRID_CELL_BORDER_THICKNESS, DARKGRAY);

                match row_idx {
                    0 => {
                        let side = (row_h * ICON_ROW_HEIGHT_FACTOR).min(cell_w * ICON_COL_WIDTH_FACTOR);
                        draw_texture_ex(
                            &self.textures[metal_idx],
                            x + (cell_w - side) * 0.5,
                            y + (row_h - side) * 0.5,
                            WHITE,
                            DrawTextureParams {
                                dest_size: Some(vec2(side, side)),
                                ..Default::default()
                            },
                        );
                    }
                    1 => {
                        let value = self.inputs.get(Metal::from(metal_idx));
                        self.draw_text_in_rect(&self.format_integer(value), x, y, cell_w, row_h, WHITE);
                    }
                    2 => {
                        let value = self.target.get(Metal::from(metal_idx));
                        self.draw_text_in_rect(&self.format_integer(value), x, y, cell_w, row_h, WHITE);
                    }
                    3 => {
                        let text = if let Some(solution) = &self.solution {
                            self.format_fraction(solution.outputs[metal_idx])
                        } else {
                            "-".to_string()
                        };
                        self.draw_text_in_rect(&text, x, y, cell_w, row_h, COLOR_OUTPUT_TEXT);
                    }
                    4..=7 => {
                        let text = self.transition_value(&self.solution, row_idx, metal_idx);
                        self.draw_text_in_rect(&text, x, y, cell_w, row_h, COLOR_TRANSITION_TEXT);
                    }
                    _ => {}
                }
            }
        }

        let ratio_box_y = OUTER_MARGIN_TOP + total_grid_height + OUTPUT_RATIO_GAP_Y;
        draw_rectangle(
            OUTER_MARGIN_X,
            ratio_box_y,
            total_grid_width,
            OUTPUT_RATIO_HEIGHT,
            COLOR_OUTPUT_RATIO_BG,
        );
        draw_rectangle_lines(OUTER_MARGIN_X, ratio_box_y, total_grid_width, OUTPUT_RATIO_HEIGHT, OUTPUT_RATIO_BORDER_THICKNESS, COLOR_OUTPUT_RATIO_BORDER);

        let ratio_text = if let Some(solution) = &self.solution {
            format!("Total Ratio Achieved: {}", format_rounded(solution.ratio, 8))
        } else {
            "Total Ratio Achieved: N/A".to_string()
        };
        self.draw_text_in_rect(&ratio_text, OUTER_MARGIN_X, ratio_box_y, total_grid_width, OUTPUT_RATIO_HEIGHT, WHITE);
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct TextCacheKey {
    text: String,
    w_px: u16,
    h_px: u16,
}
type TextMaxSize = (f32, f32, f32);
pub struct CachedTextSizer {
    final_size_cache: Mutex<HashMap<TextCacheKey, TextMaxSize>>,
    unscaled_size_cache: Mutex<HashMap<String, (f32, f32)>>,
}

impl CachedTextSizer {
    pub fn new() -> Self {
        Self {
            final_size_cache: Mutex::new(HashMap::new()),
            unscaled_size_cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_text_max_size(&self, text: &str, rect_width: f32, rect_height: f32) -> TextMaxSize {
        let w_px = rect_width.round().clamp(0.0, u16::MAX as f32) as u16;
        let h_px = rect_height.round().clamp(0.0, u16::MAX as f32) as u16;

        let key = TextCacheKey {
            text: text.to_string(),
            w_px,
            h_px,
        };

        if let Ok(cache) = self.final_size_cache.lock()
            && let Some(cached_size) = cache.get(&key) 
        {
            return *cached_size;
        }

        let text_size = self.measure(text, rect_width, rect_height);
        if let Ok(mut cache) = self.final_size_cache.lock() {
            cache.insert(key, text_size);
        }
        text_size
    }

    fn measure(&self, text: &str, rect_width: f32, rect_height: f32) -> TextMaxSize {
        let reference_size = 100u16;

        let (size_x, size_y) = if let Ok(cache) = self.unscaled_size_cache.lock()
            && let Some(dimensions) = cache.get(text)
        {
            (dimensions.0, dimensions.1)
        }
        else {
            let dimensions = measure_text(text, None, reference_size, 1.0);
            let size_x = dimensions.width;
            let size_y = dimensions.height;

            if let Ok(mut cache) = self.unscaled_size_cache.lock() {
                cache.insert(text.to_string(), (size_x, size_y));
            }
            (size_x, size_y)
        };
        let scale_x = rect_width / size_x;
        let scale_y = rect_height / size_y;
        let optimal_size = reference_size as f32 * scale_x.min(scale_y);
        let final_width = size_x * (optimal_size / reference_size as f32);
        let final_height = size_y * (optimal_size / reference_size as f32);
        let offset_x = (rect_width - final_width) / 2.0;
        let offset_y = (rect_height + final_height) / 2.0;
        (optimal_size, offset_x, offset_y)
    }
}

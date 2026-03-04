use macroquad::miniquad::conf::Icon;
use macroquad::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::model::*;
use crate::solver::solve_lp;

const IMAGE_BYTES: [&[u8]; Metal::COUNT] = [
    include_bytes!("assets/quicksilver_symbol.png"),
    include_bytes!("assets/lead_symbol.png"),
    include_bytes!("assets/tin_symbol.png"),
    include_bytes!("assets/iron_symbol.png"),
    include_bytes!("assets/copper_symbol.png"),
    include_bytes!("assets/silver_symbol.png"),
    include_bytes!("assets/gold_symbol.png"),
];

const LETTER_FONT_BYTES: &[u8] = include_bytes!("assets/old_english.ttf");
const NUMBER_FONT_BYTES: &[u8] = include_bytes!("assets/french_script.ttf");

const ICON_BIG: &[u8] = include_bytes!("assets/icon_big.tif");
const ICON_MEDIUM: &[u8] = include_bytes!("assets/icon_medium.tif");
const ICON_SMALL: &[u8] = include_bytes!("assets/icon_small.tif");

// const METAL_LABELS: [&str; Metal::COUNT] = ["QS", "Pb", "Sn", "Fe", "Cu", "Ag", "Au"];

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
const TEXT_MARGIN_Y: f32 = 0.2;

const COLOR_BACKGROUND: Color = Color::new(0.08, 0.09, 0.12, 1.0);
const COLOR_GRID_BG: Color = Color::new(0.11, 0.12, 0.15, 1.0);
const COLOR_OUTPUT_RATIO_BG: Color = Color::new(0.13, 0.16, 0.19, 1.0);
const COLOR_OUTPUT_RATIO_BORDER: Color = Color::new(0.45, 0.55, 0.66, 1.0);
const COLOR_OUTPUT_TEXT: Color = Color::new(0.78, 0.91, 0.92, 1.0);
const COLOR_TRANSITION_TEXT: Color = Color::new(0.95, 0.91, 0.8, 1.0);
const COLOR_DISABLED_ROW_OVERLAY: Color = Color::new(0.03, 0.03, 0.04, 0.6);
const COLOR_CLICKABLE_OVERLAY: Color = Color::new(0.55, 0.74, 0.96, 0.22);
const COLOR_CLICK_HINT_GLYPH: Color = Color::new(0.88, 0.95, 1.0, 0.75);

pub async fn display_loading_screen() {
    clear_background(COLOR_BACKGROUND);
    draw_text("Loading...", 100.0, 100.0, 30.0, COLOR_OUTPUT_TEXT);
    next_frame().await;
}

const ROW_COUNT: usize = 4+Transition::COUNT;
const fn generate_row_labels() -> [&'static str; ROW_COUNT] {
    let mut labels = [""; ROW_COUNT];
    labels[0] = "Metal";
    labels[1] = "Input";
    labels[2] = "Target";
    labels[3] = "Output";

    let mut idx = 0;
    while idx < Transition::COUNT {
        labels[4 + idx] = Transition::all()[idx].name();
        idx += 1;
    }
    labels
}
const ROW_LABELS: [&str; ROW_COUNT] = generate_row_labels();

fn tiff_rgba_to_array<const N: usize>(bytes: &[u8]) -> Option<[u8; N]> {
    if bytes.len() < N + 8 {
        return None;
    }

    let mut out = [0_u8; N];
    out.copy_from_slice(&bytes[8..8 + N]);
    Some(out)
}

fn embedded_icon() -> Icon {
    Icon {
        big: tiff_rgba_to_array::<16384>(ICON_BIG).unwrap(),
        medium: tiff_rgba_to_array::<4096>(ICON_MEDIUM).unwrap(),
        small: tiff_rgba_to_array::<1024>(ICON_SMALL).unwrap(),
    }
}

pub fn window_conf() -> Conf {    
    Conf {
        window_title: "Metal Ratio Solver".to_string(),
        window_width: DEFAULT_WINDOW_WIDTH,
        window_height: DEFAULT_WINDOW_HEIGHT,
        high_dpi: true,
        sample_count: DEFAULT_SAMPLE_COUNT,
        icon: Some(embedded_icon()),
        ..Default::default()
    }
}

pub struct UI {
    text_renderer: CachedTextSizer,
    letter_font: Option<Font>,
    number_font: Option<Font>,
    textures: Vec<Texture2D>,
    inputs: SolveState,
    target: SolveState,
    available_transitions: AvailableTransitions,
    solution: Result<OptimalSolution, String>,
}

impl UI {
    pub fn new(
        inputs: SolveState,
        target: SolveState,
        available_transitions: AvailableTransitions,
    ) -> Self {
        Self {
            text_renderer: CachedTextSizer::new(),
            letter_font: None,
            number_font: None,
            textures: vec![],
            inputs,
            target,
            available_transitions,
            solution: Err("No solution yet".to_string()),
        }
    }

    pub fn handle_click(&mut self, x: f32, y: f32, shift_state: bool, ctrl_state: bool) {
        let total_grid_width = screen_width() - OUTER_MARGIN_X * 2.0;
        let total_grid_height = screen_height() - OUTER_MARGIN_TOP - OUTER_MARGIN_BOTTOM;

        if x < OUTER_MARGIN_X || x > OUTER_MARGIN_X + total_grid_width {
            return;
        }

        if y < OUTER_MARGIN_TOP || y > OUTER_MARGIN_TOP + total_grid_height {
            return;
        }

        let row_h = total_grid_height / ROW_LABELS.len() as f32;
        let row_idx = ((y - OUTER_MARGIN_TOP) / row_h).floor() as usize;
        let cell_w = (total_grid_width - LABEL_COLUMN_WIDTH) / Metal::COUNT as f32;

        if (1..=2).contains(&row_idx) && x >= OUTER_MARGIN_X + LABEL_COLUMN_WIDTH {
            let cell_x = x - (OUTER_MARGIN_X + LABEL_COLUMN_WIDTH);
            let metal_idx = (cell_x / cell_w).floor() as usize;

            if metal_idx < Metal::COUNT {
                let cell_left_x = OUTER_MARGIN_X + LABEL_COLUMN_WIDTH + metal_idx as f32 * cell_w;
                let is_right_half = x >= cell_left_x + cell_w * 0.5;
                let step = if shift_state && ctrl_state {
                    1000.0
                } else if ctrl_state {
                    100.0
                } else if shift_state {
                    10.0
                } else {
                    1.0
                };
                let delta = if is_right_half { step } else { -step };

                if row_idx == 1 {
                    let current = self.inputs.metals[metal_idx];
                    self.inputs.metals[metal_idx] = (current + delta).max(0.0);
                } else {
                    let current = self.target.metals[metal_idx];
                    self.target.metals[metal_idx] = (current + delta).max(0.0);
                }

                self.solve();
                return;
            }
        }

        if (4..ROW_COUNT).contains(&row_idx) {
            let transition = Transition::from(row_idx-4);
            let is_enabled = self.available_transitions.get(transition);
            self.available_transitions.set(transition, !is_enabled);
            self.solve();
        }
    }

    fn transition_enabled(&self, row_idx: usize) -> bool {
        if (4..ROW_COUNT).contains(&row_idx) {
            let transition = Transition::from(row_idx-4);
            let is_enabled = self.available_transitions.get(transition);
            return is_enabled;
        }
        false
    }

    pub async fn load_assets(&mut self) {
        if let Ok(font) = load_ttf_font_from_bytes(LETTER_FONT_BYTES) {
            self.letter_font = Some(font);
        }
        if let Ok(font) = load_ttf_font_from_bytes(NUMBER_FONT_BYTES) {
            self.number_font = Some(font);
        }
        if self.letter_font.is_none() {
            println!("Failed to load embedded letter font; using Macroquad default font for labels.");
        }
        if self.number_font.is_none() {
            println!("Failed to load embedded number font; using Macroquad default font for numbers.");
        }

        let mut textures: [Option<Texture2D>; Metal::COUNT] = std::array::from_fn(|_| None);
        for idx in 0..Metal::COUNT {
            let texture = Texture2D::from_file_with_format(IMAGE_BYTES[idx], None);
            texture.set_filter(FilterMode::Linear);
            textures[idx] = Some(texture);
        }
        self.textures = textures.iter().filter_map(|t| t.clone()).collect();
    }

    fn transition_value(&self, row_idx: usize, col_idx: usize) -> String {
        let transition = Transition::from(row_idx - 4);
        let metal = Metal::from(col_idx);
        if !transition.is_valid_target(metal) {
            return "".to_string();
        }
        let Ok(solution) = &self.solution else {
            return "/".to_string();
        };
        let value = solution.get_transition_value(transition, metal);
        if let Some(value) = value {
            decimal_to_fraction(value)
        } else {
            "".to_string()
        }
    }

    fn draw_add_sub_hints(&self, rect: Rect, shift_state: bool, ctrl_state: bool) {
        let plus_text_params = TextParams {
            font: self.get_font(FontKind::Letters),
            font_size: (rect.h * 0.7).round() as u16,
            color: COLOR_CLICK_HINT_GLYPH,
            ..Default::default()
        };
        let plus_x = rect.x + rect.w * 0.7;
        let plus_y = rect.y + rect.h * 0.75;
        draw_text_ex("+", plus_x, plus_y, plus_text_params);
        let minus_text_params = TextParams {
            font: self.get_font(FontKind::Letters),
            font_size: (rect.h * 1.5).round() as u16,
            color: COLOR_CLICK_HINT_GLYPH,
            ..Default::default()
        };
        let minus_x = rect.x + rect.w * 0.0;
        let minus_y = rect.y + rect.h * 0.9108;
        draw_text_ex("-", minus_x, minus_y, minus_text_params);
        // make left slightly darker and right slightly lighter to hint at which is which
        let overlay_multiplier = if shift_state && ctrl_state {
            3.0
        } else if ctrl_state {
            2.1
        } else if shift_state {
            1.4
        } else {
            0.9
        };
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w * 0.5,
            rect.h,
            Color::new(0.0, 0.0, 0.0, 0.09 * overlay_multiplier)
        );
        draw_rectangle(
            rect.x + rect.w * 0.5,
            rect.y,
            rect.w * 0.5,
            rect.h,
            Color::new(1.0, 1.0, 1.0, 0.05 * overlay_multiplier)
        );
    }

    fn draw_text_in_rect(&self, text: &str, rect: Rect, color: Color, font_kind: FontKind) {
        if text.is_empty() {
            return;
        }
        let x = rect.x + (rect.w * TEXT_MARGIN_X);
        let y = rect.y + (rect.h * TEXT_MARGIN_Y);
        let w = (1.0 - (2.0 * TEXT_MARGIN_X)) * rect.w;
        let h = (1.0 - (2.0 * TEXT_MARGIN_Y)) * rect.h;

        let (optimal_size, offset_x, offset_y) = self
            .text_renderer
            .get_text_max_size(text, w, h, self.get_font(font_kind), font_kind);
        draw_text_ex(
            text,
            x + offset_x,
            y + offset_y,
            TextParams {
                font: self.get_font(font_kind),
                font_size: optimal_size.max(1.0).round() as u16,
                color,
                ..Default::default()
            },
        );
    }

    fn draw_label_text_in_rect(&self, text: &str, rect: Rect, color: Color) {
        self.draw_text_in_rect(text, rect, color, FontKind::Letters);
    }

    fn draw_number_text_in_rect(&self, text: &str, rect: Rect, color: Color) {
        self.draw_text_in_rect(text, rect, color, FontKind::Numbers);
    }

    fn get_font(&self, font_kind: FontKind) -> Option<&Font> {
        match font_kind {
            FontKind::Letters => self.letter_font.as_ref(),
            FontKind::Numbers => self.number_font.as_ref(),
        }
    }

    pub fn draw(&self, mouse_x: f32, mouse_y: f32, shift_state: bool, ctrl_state: bool) {
        clear_background(COLOR_BACKGROUND);

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
            let label_rect = Rect::new(OUTER_MARGIN_X, y, LABEL_COLUMN_WIDTH, row_h);

            draw_rectangle_lines(OUTER_MARGIN_X, y, LABEL_COLUMN_WIDTH, row_h, GRID_LABEL_BORDER_THICKNESS, GRAY);

            if (4..ROW_COUNT).contains(&row_idx) {
                draw_rectangle(
                    label_rect.x,
                    label_rect.y,
                    label_rect.w,
                    label_rect.h,
                    COLOR_CLICKABLE_OVERLAY,
                );
            }

            self.draw_label_text_in_rect(row_label, label_rect, WHITE);

            for metal_idx in 0..Metal::COUNT {
                let x = OUTER_MARGIN_X + LABEL_COLUMN_WIDTH + metal_idx as f32 * cell_w;
                let cell_rect = Rect::new(x, y, cell_w, row_h);
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
                        draw_rectangle(x, y, cell_w, row_h, COLOR_CLICKABLE_OVERLAY);
                        let value = self.inputs.get(Metal::from(metal_idx));
                        self.draw_number_text_in_rect(&format_rounded(value, 0), cell_rect, WHITE);

                        if mouse_x >= x && mouse_x <= x + cell_w && mouse_y >= y && mouse_y <= y + row_h {
                            self.draw_add_sub_hints(cell_rect, shift_state, ctrl_state);
                        }
                    }
                    2 => {
                        draw_rectangle(x, y, cell_w, row_h, COLOR_CLICKABLE_OVERLAY);
                        let value = self.target.get(Metal::from(metal_idx));
                        self.draw_number_text_in_rect(&format_rounded(value, 0), cell_rect, WHITE);

                        if mouse_x >= x && mouse_x <= x + cell_w && mouse_y >= y && mouse_y <= y + row_h {
                            self.draw_add_sub_hints(cell_rect, shift_state, ctrl_state);
                        }
                    }
                    3 => {
                        let text = if let Ok(solution) = &self.solution {
                            decimal_to_fraction(solution.outputs[metal_idx])
                        } else {
                            "/".to_string()
                        };
                        self.draw_number_text_in_rect(&text, cell_rect, COLOR_OUTPUT_TEXT);
                    }
                    4..ROW_COUNT => {
                        let text = self.transition_value(row_idx, metal_idx);
                        self.draw_number_text_in_rect(&text, cell_rect, COLOR_TRANSITION_TEXT);
                    }
                    _ => {}
                }
            }

            if (4..ROW_COUNT).contains(&row_idx) && !self.transition_enabled(row_idx) {
                draw_rectangle(
                    OUTER_MARGIN_X,
                    y,
                    total_grid_width,
                    row_h,
                    COLOR_DISABLED_ROW_OVERLAY,
                );
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

        let ratio_label_x = OUTER_MARGIN_X;
        let ratio_label_w = total_grid_width * 0.56;
        let ratio_label_rect = Rect::new(ratio_label_x, ratio_box_y, ratio_label_w, OUTPUT_RATIO_HEIGHT);
        match &self.solution {
            Ok(solution) => {
                self.draw_label_text_in_rect("Total Ratio Achieved:", ratio_label_rect, COLOR_OUTPUT_TEXT);
                let ratio_value_x = OUTER_MARGIN_X + total_grid_width * 0.56;
                let ratio_value_w = total_grid_width * 0.42;
                let ratio_value_rect = Rect::new(ratio_value_x, ratio_box_y, ratio_value_w, OUTPUT_RATIO_HEIGHT);
                let ratio_value_text = format!("{} ({})", format_rounded(solution.ratio, 4), decimal_to_fraction(solution.ratio));
                self.draw_number_text_in_rect(&ratio_value_text, ratio_value_rect, WHITE);
            }
            Err(err) => {
                let error_x = OUTER_MARGIN_X;
                let error_w = total_grid_width;
                let error_rect = Rect::new(error_x, ratio_box_y, error_w, OUTPUT_RATIO_HEIGHT);
                self.draw_label_text_in_rect(err, error_rect, COLOR_OUTPUT_TEXT);
            }
        }
    }

    pub fn solve(&mut self) {
        self.solution = solve_lp(&self.inputs, &self.target, &self.available_transitions);
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
enum FontKind {
    Letters,
    Numbers,
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct TextCacheKey {
    text: String,
    w_px: u16,
    h_px: u16,
    font_kind: FontKind,
}
type TextMaxSize = (f32, f32, f32);
pub struct CachedTextSizer {
    final_size_cache: Mutex<HashMap<TextCacheKey, TextMaxSize>>,
    unscaled_size_cache: Mutex<HashMap<(String, FontKind), (f32, f32)>>,
}

impl CachedTextSizer {
    pub fn new() -> Self {
        Self {
            final_size_cache: Mutex::new(HashMap::new()),
            unscaled_size_cache: Mutex::new(HashMap::new()),
        }
    }

    fn get_text_max_size(&self, text: &str, rect_width: f32, rect_height: f32, font: Option<&Font>, font_kind: FontKind) -> TextMaxSize {
        let w_px = rect_width.round().clamp(0.0, u16::MAX as f32) as u16;
        let h_px = rect_height.round().clamp(0.0, u16::MAX as f32) as u16;

        let key = TextCacheKey {
            text: text.to_string(),
            w_px,
            h_px,
            font_kind,
        };

        if let Ok(cache) = self.final_size_cache.lock()
            && let Some(cached_size) = cache.get(&key) 
        {
            return *cached_size;
        }

        let text_size = self.measure(text, rect_width, rect_height, font, font_kind);
        if let Ok(mut cache) = self.final_size_cache.lock() {
            cache.insert(key, text_size);
        }
        text_size
    }

    fn measure(&self, text: &str, rect_width: f32, rect_height: f32, font: Option<&Font>, font_kind: FontKind) -> TextMaxSize {
        let reference_size = 100u16;
        let text_key = (text.to_string(), font_kind);

        let (size_x, size_y) = if let Ok(cache) = self.unscaled_size_cache.lock()
            && let Some(dimensions) = cache.get(&text_key)
        {
            (dimensions.0, dimensions.1)
        }
        else {
            let dimensions = measure_text(text, font, reference_size, 1.0);
            let size_x = dimensions.width;
            let size_y = dimensions.height;

            if let Ok(mut cache) = self.unscaled_size_cache.lock() {
                cache.insert(text_key, (size_x, size_y));
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

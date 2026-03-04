#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod model;
use macroquad::prelude::{is_mouse_button_pressed, mouse_position, MouseButton};
use macroquad::window::next_frame;
use model::*;

mod solver;

mod ui;
use ui::*;

#[macroquad::main(window_conf)]
async fn main() {
    display_loading_screen().await;
    let initial_state = SolveState::from_input("2 2 0 0 0 0 0").expect("Invalid hardcoded initial state");
    let target_state = SolveState::from_input("0 2 0 1 0 0 0").expect("Invalid hardcoded target state");
    let available_transitions = AvailableTransitions::from_input("1 0 0 1").expect("Invalid hardcoded available transitions");

    let mut ui = UI::new(initial_state, target_state, available_transitions);

    ui.load_assets().await;
    ui.solve();

    loop {
        let (x, y) = mouse_position();
        let shift = macroquad::input::is_key_down(macroquad::prelude::KeyCode::LeftShift) || macroquad::input::is_key_down(macroquad::prelude::KeyCode::RightShift);
        let ctrl = macroquad::input::is_key_down(macroquad::prelude::KeyCode::LeftControl) || macroquad::input::is_key_down(macroquad::prelude::KeyCode::RightControl);
        if is_mouse_button_pressed(MouseButton::Left) {
            ui.handle_click(x, y, shift, ctrl);
        }
        ui.draw(x, y, shift, ctrl);
        next_frame().await;
    }
}

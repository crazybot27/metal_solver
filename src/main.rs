mod model;
use macroquad::window::next_frame;
use model::*;

mod solver;

mod ui;
use ui::*;

#[macroquad::main(window_conf)]
async fn main() {
    let initial_state = SolveState::from_input("0 0 0 5 5 5 30").expect("Invalid hardcoded initial state");
    let target_state = SolveState::from_input("0 0 0 5 3 3 3").expect("Invalid hardcoded target state");
    let available_transitions = AvailableTransitions {
        projection: false,
        rejection: true,
        purification: false,
        deposition: true,
    };

    let mut ui = UI {
        text_renderer: CachedTextSizer::new(),
        textures: vec![],
        inputs: initial_state,
        target: target_state,
        available_transitions,
        solution: None,
    };

    ui.load_textures().await;

    loop {
        ui.draw();
        next_frame().await;
    }
}

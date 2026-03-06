use metal_solver_core::model::{AvailableTransitions, SolveState};
use metal_solver_core::solver::solve_lp;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn solve_ratio(initial: &str, target: &str, transitions: &str) -> String {
    let initial_state = match SolveState::from_input(initial) {
        Ok(state) => state,
        Err(e) => return format!(r#"{{"error": "Error with initial state: {}"}}"#, e),
    };
    let target_state = match SolveState::from_input(target) {
        Ok(state) => state,
        Err(e) => return format!(r#"{{"error": "Error with target state: {}"}}"#, e),
    };
    let available_transitions = match AvailableTransitions::from_input(transitions) {
        Ok(trans) => trans,
        Err(e) => return format!(r#"{{"error": "Error with available transitions: {}"}}"#, e),
    };

    match solve_lp(&initial_state, &target_state, &available_transitions) {
        Ok(solution) => solution.to_json_string(false, true),
        Err(e) => format!(r#"{{"error": "Error with solving: {}"}}"#, e),
    }
}
#[wasm_bindgen]
pub fn get_transition_names() -> String {
    let transition_names: Vec<&str> = metal_solver_core::model::Transition::all().iter().map(|t| t.name()).collect();
    let result = transition_names.join("\",\"");
    format!("[\"{}\"]", result)
    
}
#[wasm_bindgen]
pub fn get_metal_names() -> String {
    let metal_names: Vec<&str> = metal_solver_core::model::Metal::all().iter().map(|m| m.name()).collect();
    let result = metal_names.join("\",\"");
    format!("[\"{}\"]", result)
}

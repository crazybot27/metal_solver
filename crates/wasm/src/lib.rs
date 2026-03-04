use metal_solver_core::model::{AvailableTransitions, SolveState};
use metal_solver_core::solver::solve_lp;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn solve_ratio(initial: &str, target: &str, transitions: &str) -> Result<String, String> {
    let initial_state = SolveState::from_input(initial).map_err(|e| format!("Invalid initial state: {e}"))?;
    let target_state = SolveState::from_input(target).map_err(|e| format!("Invalid target state: {e}"))?;
    let available_transitions = AvailableTransitions::from_input(transitions).map_err(|e| format!("Invalid transitions: {e}"))?;

    solve_lp(&initial_state, &target_state, &available_transitions)
        .map(|solution| solution.to_json_string(false, true))
        .map_err(|e| format!("Solve failed: {e}"))
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

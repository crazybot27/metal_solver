use metal_solver_core::model::{AvailableTransitions, SolveState};
use metal_solver_core::solver::solve_lp;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn solve_ratio(initial: &str, target: &str, transitions: &str) -> Result<String, String> {
    let initial_state = SolveState::from_input(initial)?;
    let target_state = SolveState::from_input(target)?;
    let available_transitions = AvailableTransitions::from_input(transitions)?;

    let solution = solve_lp(&initial_state, &target_state, &available_transitions)?;
    Ok(format!("{} ({})", solution.ratio, metal_solver_core::model::decimal_to_fraction(solution.ratio)))
}

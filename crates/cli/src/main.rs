use metal_solver_core::model::{AvailableTransitions, SolveState};
use metal_solver_core::solver::solve_lp;

fn main() {
    let initial_state = SolveState::from_input("2 2 0 0 0 0 0").expect("Invalid hardcoded initial state");
    let target_state = SolveState::from_input("0 2 0 1 0 0 0").expect("Invalid hardcoded target state");
    let available_transitions = AvailableTransitions::from_input("1 0 0 1").expect("Invalid hardcoded available transitions");

    match solve_lp(&initial_state, &target_state, &available_transitions) {
        Ok(solution) => println!("{solution:?}"),
        Err(err) => eprintln!("Solve failed: {err}"),
    }
}

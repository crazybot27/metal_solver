use metal_solver_core::model::{escape_json_string, AvailableTransitions, SolveState};
use metal_solver_core::solver::solve_lp;
use std::env;

enum OutputMode {
    Human,
    Json,
}

struct JsonOptions {
    use_names: bool,
    pretty_values: bool,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let (mode, json_options, positional) = parse_args(&args);
    if positional.len() != 3 {
        print_usage(&args[0]);
        std::process::exit(2);
    }

    let initial_state = match SolveState::from_input(&positional[0]) {
        Ok(state) => state,
        Err(err) => {
            eprintln!("Invalid inputs: {err}");
            std::process::exit(2);
        }
    };
    let target_state = match SolveState::from_input(&positional[1]) {
        Ok(state) => state,
        Err(err) => {
            eprintln!("Invalid outputs: {err}");
            std::process::exit(2);
        }
    };
    let available_transitions = match AvailableTransitions::from_input(&positional[2]) {
        Ok(transitions) => transitions,
        Err(err) => {
            eprintln!("Invalid transitions: {err}");
            std::process::exit(2);
        }
    };

    match solve_lp(&initial_state, &target_state, &available_transitions) {
        Ok(solution) => match mode {
            OutputMode::Human => println!("{solution:?}"),
            OutputMode::Json => println!("{}", solution.to_json_string(json_options.use_names, json_options.pretty_values)),
        },
        Err(err) => match mode {
            OutputMode::Human => eprintln!("Solve failed: {err}"),
            OutputMode::Json => {
                eprintln!("{{\"error\":\"{}\"}}", escape_json_string(&err));
                std::process::exit(1);
            }
        },
    }
}

fn parse_args(args: &[String]) -> (OutputMode, JsonOptions, Vec<String>) {
    let mut mode = OutputMode::Json;
    let mut use_names = false;
    let mut pretty_values = false;
    let mut positional = Vec::new();
    for arg in args.iter().skip(1) {
        // match arg.as_str() {
        //     "-h" => mode = OutputMode::Human,
        //     "-n" => use_names = true,
        //     "-p" => pretty_values = true,
        //     _ => positional.push(arg.clone()),
        // }
        if arg.starts_with("-") {
            for ch in arg.chars().skip(1) {
                match ch {
                    'h' => mode = OutputMode::Human,
                    'n' => use_names = true,
                    'p' => pretty_values = true,
                    _ => {
                        eprintln!("Unknown option: -{ch}");
                        print_usage(&args[0]);
                        std::process::exit(2);
                    }
                }
            }
        } else {
            positional.push(arg.clone());
        }
    }
    (
        mode,
        JsonOptions {
            use_names,
            pretty_values,
        },
        positional,
    )
}

fn print_usage(program: &str) {
    eprintln!("Usage: {program} [-h] [-n] [-p] <inputs> <outputs> <transitions>");
    eprintln!("Example (json): {program} \"2 2 0 0 0 0 0\" \"0 2 0 1 0 0 0\" \"1 0 0 1\"");
    eprintln!("Example (human): {program} -h \"2 2 0 0 0 0 0\" \"0 2 0 1 0 0 0\" \"1 0 0 1\"");
}


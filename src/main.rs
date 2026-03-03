use std::fmt::Debug;
use std::io::{self, Write};
use std::collections::HashSet;

use good_lp::{
    constraint, default_solver, variable, variables, Expression, Solution, SolverModel, Variable,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Metal {
    Quicksilver = 0,
    Lead = 1,
    Tin = 2,
    Iron = 3,
    Copper = 4,
    Silver = 5,
    Gold = 6,
}
impl Metal {
    const COUNT: usize = 7;

    pub const fn idx(self) -> usize {
        self as usize
    }

    pub const fn from(idx: usize) -> Self {
        match idx {
            0 => Metal::Quicksilver,
            1 => Metal::Lead,
            2 => Metal::Tin,
            3 => Metal::Iron,
            4 => Metal::Copper,
            5 => Metal::Silver,
            6 => Metal::Gold,
            _ => panic!("Invalid metal index"),
        }
    }

    pub const fn all() -> [Self; Self::COUNT] {
        [
            Metal::Quicksilver,
            Metal::Lead,
            Metal::Tin,
            Metal::Iron,
            Metal::Copper,
            Metal::Silver,
            Metal::Gold,
        ]
    }

    pub const fn normals() -> [Self; Self::COUNT - 1] {
        [
            Metal::Lead,
            Metal::Tin,
            Metal::Iron,
            Metal::Copper,
            Metal::Silver,
            Metal::Gold,
        ]
    }

    pub const fn next(self) -> Option<Self> {
        match self {
            Metal::Lead => Some(Metal::Tin),
            Metal::Tin => Some(Metal::Iron),
            Metal::Iron => Some(Metal::Copper),
            Metal::Copper => Some(Metal::Silver),
            Metal::Silver => Some(Metal::Gold),
            Metal::Gold | Metal::Quicksilver => None,
        }
    }
    pub const fn prev(self) -> Option<Self> {
        match self {
            Metal::Tin => Some(Metal::Lead),
            Metal::Iron => Some(Metal::Tin),
            Metal::Copper => Some(Metal::Iron),
            Metal::Silver => Some(Metal::Copper),
            Metal::Gold => Some(Metal::Silver),
            Metal::Quicksilver | Metal::Lead => None,
        }
    }
    pub const fn get_split_metals(self) -> Option<(Self, Self)> {
        match self {
            Metal::Lead => None,
            Metal::Tin => Some((Metal::Lead, Metal::Lead)),
            Metal::Iron => Some((Metal::Lead, Metal::Tin)),
            Metal::Copper => Some((Metal::Tin, Metal::Tin)),
            Metal::Silver => Some((Metal::Tin, Metal::Iron)),
            Metal::Gold => Some((Metal::Iron, Metal::Iron)),
            Metal::Quicksilver => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Transition {
    Projection, // Uses one QS to raise a metal to the next level
    Rejection, // Lowers a metal and yields a QS
    Purification, // Turns two metals into one of the next level
    Deposition, // Splits a metal of tier N into two of tiers floor(N/2) and ceil(N/2)
}

#[derive(Clone, Copy)]
pub struct AvailableTransitions {
    projection: bool,
    rejection: bool,
    purification: bool,
    deposition: bool,
}
impl std::fmt::Debug for AvailableTransitions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut transitions = Vec::new();
        if self.projection {
            transitions.push("Pro");
        }
        if self.rejection {
            transitions.push("Rej");
        }
        if self.purification {
            transitions.push("Pur");
        }
        if self.deposition {
            transitions.push("Dep");
        }
        let output = match transitions.len() {
            0 => "None".to_string(),
            4 => "All".to_string(),
            _ => transitions.join(", "),
        };
        write!(f, "[{}]", output)
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct SolveState {
    metals: [f64; Metal::COUNT],
}

impl std::fmt::Debug for SolveState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        for metal in Metal::all() {
            parts.push(format!("{:?}: {}", metal, self.get(metal)));
        }
        write!(f, "{{{}}}", parts.join(", "))
    }
}

impl SolveState {
    pub fn from_input(input: &str) -> Result<Self, String> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.len() != Metal::COUNT {
            return Err(format!("Expected {} values for metals, got {}", Metal::COUNT, parts.len()));
        }
        let mut metals = [0.0; Metal::COUNT];
        for (i, part) in parts.iter().enumerate() {
            metals[i] = part.parse::<f64>().map_err(|e| format!("Failed to parse metal count '{}': {}", part, e))?;
        }
        Ok(SolveState { metals })
    }

    pub fn get(&self, metal: Metal) -> f64 {
        self.metals[metal.idx()]
    }

    fn theoretically_reachable_metals(&self, transitions: &AvailableTransitions) -> HashSet<Metal> {
        let mut available_normal_metals: HashSet<Metal> = self
            .metals
            .iter()
            .enumerate()
            .filter_map(|(i, &count)| if count > 0.0 { Some(Metal::from(i)) } else { None })
            .collect();

        // quicksilver is special since it can't be purified or deposited, so we track it separately and add it back in at the end if it was available or could be made available by rejection.
        let mut quicksilver_available = self.get(Metal::Quicksilver) > 0.0;
        if available_normal_metals.contains(&Metal::Quicksilver) {
            available_normal_metals.remove(&Metal::Quicksilver);
        }


        // add all metals reachable by deposition from the highest available metal. 
        // If the number of available metals was higher the way we do this would be inaccurate (ie a metal of tier 8 would split into 4,4 which cannot reach 3)
        // but since the highest tier is 6->3 and holes only start at 4+ we can just assume there's no holes)
        if transitions.deposition 
            && let Some(&max_available) = available_normal_metals.iter().max_by_key(|m| m.idx())
            && let Some((metal1, metal2)) = max_available.get_split_metals()
        {
            let max_deposition_product_idx = metal1.idx().max(metal2.idx());
            for metal in Metal::normals() {
                if metal.idx() <= max_deposition_product_idx {
                    available_normal_metals.insert(metal);
                }
            }
        }

        // purification can reach any metal as long as low enough metals exist and costs no qs, so we put it early. Deposition goes first since it has "holes" and purification fills them.
        if transitions.purification
            && let Some(&min_available) = available_normal_metals.iter().min_by_key(|m| m.idx())
        {
            for metal in Metal::normals() {
                if metal.idx() >= min_available.idx() {
                    available_normal_metals.insert(metal);
                }
            }
        }

        // rejection adds qs if there are higher metals available which is why purification is before it
        if transitions.rejection
            && let Some(&max_available) = available_normal_metals.iter().max_by_key(|m| m.idx())
        {
            for metal in Metal::normals() {
                if metal.idx() <= max_available.idx() {
                    available_normal_metals.insert(metal);
                }
            }
            if max_available.idx() > Metal::Lead.idx() {
                quicksilver_available = true;
            }
        }

        // finally, projection needs qs so we need to put the step that could create qs before it. 
        // If you work out all the other relationships, you'll find that either order doesn't matter or this order works.
        // deposition and rejection or purification and projection can be done in any order
        // and projection and deposition need to be in that order so projection can fill in holes in the deposition tree.
        if transitions.projection
            && quicksilver_available
            && let Some(&min_available) = available_normal_metals.iter().min_by_key(|m| m.idx())
        {
            for metal in Metal::normals() {
                if metal.idx() >= min_available.idx() {
                    available_normal_metals.insert(metal);
                }
            }
        }

        if quicksilver_available {
            available_normal_metals.insert(Metal::Quicksilver);
        }
        available_normal_metals
        // this order guarantees that all creatable metals are checked, so no need to iterate to a fixed point or anything like that.
    }

    fn target_within_reachable_metals(reachable_metals: &HashSet<Metal>, target: &SolveState) -> bool {
        let desired_metals: HashSet<Metal> = target
            .metals
            .iter()
            .enumerate()
            .filter_map(|(i, &count)| if count > 0.0 { Some(Metal::from(i)) } else { None })
            .collect();

        desired_metals.is_subset(reachable_metals)
    }

    pub fn can_theoretically_reach(&self, target: &SolveState, transitions: &AvailableTransitions) -> bool {
        let reachable_metals = self.theoretically_reachable_metals(transitions);
        Self::target_within_reachable_metals(&reachable_metals, target)
    }
}

struct OptimalSolution {
    ratio: f64,
    outputs: [f64; Metal::COUNT],
    projection: [f64; 5],
    rejection: [f64; 5],
    purification: [f64; 5],
    deposition: [f64; 5],
}

fn format_rounded(value: f64, digits: usize) -> String {
    if (value * 10.0).round() == (value * 10.0) {
        format!("{:.1}", value)
    } else {
        format!("{:.1$}", value, digits)
    }
}

fn decimal_to_fraction(value: f64) -> String {
    let tolerance = 1e-6;
    let mut numerator = 1;
    let mut denominator = 1;

    while (numerator as f64 / denominator as f64 - value).abs() > tolerance {
        if numerator > 1000 || denominator > 1000 {
            return format!("{:.5}", value);
        }
        if (numerator as f64) / (denominator as f64) < value {
            numerator += 1;
        } else {
            denominator += 1;
        }
    }

    format!("{}/{}", numerator, denominator)
}

impl Debug for OptimalSolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut outputs_string = "".to_string();
        outputs_string += &format!("Ratio: {}\n", format_rounded(self.ratio, 8));
        outputs_string += "Outputs:\n";
        for metal in Metal::all() {
            let idx = metal.idx();
            outputs_string += &format!(
                "- {:?}: {}\n",
                metal,
                format_rounded(self.outputs[idx], 5),
            );
        }
        let mut potencies_string = "Transition potencies:\n".to_string();

        for (transition_type, transition_values, metals_index_offset) in [
            ("Projection", &self.projection, 1),
            ("Rejection", &self.rejection, 2),
            ("Purification", &self.purification, 1),
            ("Deposition", &self.deposition, 2),
        ] {
            if transition_values.iter().all(|&x| x == 0.0) {
                potencies_string += &format!("- {}: N/A\n", transition_type);
            } else {
                potencies_string += &format!("- {} on:\n", transition_type);
                for (index, value) in transition_values.iter().enumerate() {
                    if *value == 0.0 {continue;}
                    potencies_string += &format!("  - {:?}: {} ({})\n", Metal::from(index + metals_index_offset), decimal_to_fraction(*value), format_rounded(*value, 5));
                }
            }
        }
        write!(f, "{}{}", outputs_string, potencies_string)
    }
}

pub fn solve_lp(
    initial: &SolveState,
    target: &SolveState,
    transitions: &AvailableTransitions
) -> Result<OptimalSolution, String> {
    let mut vars = variables!();
    let projection: [Variable; 5] = std::array::from_fn(|_| vars.add(variable().min(0.0)));
    let rejection: [Variable; 5] = std::array::from_fn(|_| vars.add(variable().min(0.0)));
    let purification: [Variable; 5] = std::array::from_fn(|_| vars.add(variable().min(0.0)));
    let deposition: [Variable; 5] = std::array::from_fn(|_| vars.add(variable().min(0.0)));
    let ratio = vars.add(variable().min(0.0));

    let i = initial.metals;
    let p = projection;
    let r = rejection;
    let pu = purification;
    let d = deposition;

    /*
                    O1 = I1          - P2   + R2                  - 2.0 * Pu2          + 2.0 * D2   + D3       
                    O2 = I2   + P2   - P3   + R3   - R2   + Pu2   - 2.0 * Pu3   + D3   + 2.0 * D4   + D5   - D2
                    O3 = I3   + P3   - P4   + R4   - R3   + Pu3   - 2.0 * Pu4   + D5   + 2.0 * D6          - D3
                    O4 = I4   + P4   - P5   + R5   - R4   + Pu4   - 2.0 * Pu5                              - D4
                    O5 = I5   + P5   - P6   + R6   - R5   + Pu5   - 2.0 * Pu6                              - D5
                    O6 = I6   + P6                 - R6   + Pu6                                            - D6
    */
    let projection_terms: [Expression; 6] = [
             - p[0],
        p[0] - p[1],
        p[1] - p[2],
        p[2] - p[3],
        p[3] - p[4],
        p[4].into(),

    ];
    let rejection_terms: [Expression; 6] = [
        r[0].into(),
        r[1] - r[0],
        r[2] - r[1],
        r[3] - r[2],
        r[4] - r[3],
             - r[4],
    ];
    let purification_terms: [Expression; 6] = [
              - 2.0 * pu[0],
        pu[0] - 2.0 * pu[1],
        pu[1] - 2.0 * pu[2],
        pu[2] - 2.0 * pu[3],
        pu[3] - 2.0 * pu[4],
        pu[4].into(),
    ];
    let deposition_terms: [Expression; 6] = [
                        (2.0*d[0]) + d[1],
        - d[0] + d[1] + (2.0*d[2]) + d[3],
        - d[1] + d[3] + (2.0*d[4]),
        - d[2],
        - d[3],
        - d[4],
    ];

    let o0: Expression = i[0]
        + r.into_iter().fold(0.0.into(), |acc: Expression, x| acc + x)
        - p.into_iter().fold(0.0.into(), |acc: Expression, x| acc + x);

    let mut output_expressions = vec![o0];
    for idx in 0..6 {
        let output = i[idx + 1] 
            + projection_terms[idx].clone() 
            + rejection_terms[idx].clone() 
            + purification_terms[idx].clone() 
            + deposition_terms[idx].clone();
        output_expressions.push(output);
    }

    let mut model = vars.maximise(ratio).using(default_solver);

    for output in &output_expressions {
        model = model.with(constraint!(output.clone() >= 0.0));
    }

    let mut has_target_component = false;
    for (idx, output) in output_expressions.iter().enumerate() {
        let required = target.metals[idx];
        if required > 0.0 {
            has_target_component = true;
            model = model.with(constraint!(output.clone() >= ratio * required));
        }
    }

    if !has_target_component {
        return Err("Target must have at least one positive metal amount".to_string());
    }

    if !transitions.projection {
        for var in &projection {
            model = model.with(constraint!(*var == 0.0));
        }
    }
    if !transitions.rejection {
        for var in &rejection {
            model = model.with(constraint!(*var == 0.0));
        }
    }
    if !transitions.purification {
        for var in &purification {
            model = model.with(constraint!(*var == 0.0));
        }
    }
    if !transitions.deposition {
        for var in &deposition {
            model = model.with(constraint!(*var == 0.0));
        }
    }

    let solution = model
        .solve()
        .map_err(|e| format!("Linear program failed to solve: {e}"))?;

    let projection_values = std::array::from_fn(|idx| solution.value(projection[idx]));
    let rejection_values = std::array::from_fn(|idx| solution.value(rejection[idx]));
    let purification_values = std::array::from_fn(|idx| solution.value(purification[idx]));
    let deposition_values = std::array::from_fn(|idx| solution.value(deposition[idx]));
    let ratio_value = solution.value(ratio);

    let outputs: [f64; Metal::COUNT] = std::array::from_fn(|idx| {
        solution.eval(output_expressions[idx].clone())
    });

    Ok(OptimalSolution {
        ratio: ratio_value,
        outputs,
        projection: projection_values,
        rejection: rejection_values,
        purification: purification_values,
        deposition: deposition_values,
    })
}
fn main() {
    const USE_INTERACTIVE_INPUT: bool = false;

    let (initial_state, target_state, available_transitions) = if !USE_INTERACTIVE_INPUT {
        // For testing / rapid iteration
        let initial_state = SolveState::from_input("0 0 0 5 5 5 30").expect("Invalid hardcoded initial state");
        let target_state = SolveState::from_input("0 0 0 5 3 3 3").expect("Invalid hardcoded target state");
        let available_transitions = AvailableTransitions {
            projection: false,
            rejection: true,
            purification: false,
            deposition: true,
        };

        (initial_state, target_state, available_transitions)
    } else {
        println!("Enter initial state (7 numbers for each metal seperated by spaces: Quicksilver Lead Tin Iron Copper Silver Gold):");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line");
        let initial_state = SolveState::from_input(&input).expect("Invalid input state");

        println!("Enter target state (7 numbers for each metal seperated by spaces: Quicksilver Lead Tin Iron Copper Silver Gold):");
        io::stdout().flush().unwrap();
        input.clear();
        io::stdin().read_line(&mut input).expect("Failed to read line");
        let target_state = SolveState::from_input(&input).expect("Invalid input state");

        println!("Which transitions are available? (Enter 4 booleans (or 0/1) for Projection Rejection Purification Deposition, e.g. 'true false true true' or '1 0 1 1'):");
        io::stdout().flush().unwrap();
        input.clear();
        io::stdin().read_line(&mut input).expect("Failed to read line");
        let transitions_input: Vec<_> = input.split_whitespace().collect();
        if transitions_input.len() != 4 {
            panic!("Expected 4 transitions input");
        }
        let transitions_input: Vec<bool> = transitions_input
            .iter()
            .map(|s| match s.trim() {
                "1" | "true" => true,
                "0" | "false" => false,
                _ => s.parse().expect("Invalid boolean for transition availability"),
            })
            .collect();
        let available_transitions = AvailableTransitions {
            projection: transitions_input[0],
            rejection: transitions_input[1],
            purification: transitions_input[2],
            deposition: transitions_input[3],
        };

        (initial_state, target_state, available_transitions)
    };

    if !initial_state.can_theoretically_reach(&target_state, &available_transitions) {
        println!("Target is completely unreachable with the selected transitions.");
        return;
    }

    // calculate the optimal ratio and one possible optimal set of transitions

    println!("Initial State: {:?}", initial_state);
    println!("Target State: {:?}", target_state);
    println!("Available transitions: {:?}", available_transitions);

    match solve_lp(
        &initial_state,
        &target_state,
        &available_transitions
    ) {
        Ok(solution) => {
            println!("Optimal solution found:\n{:?}", solution);
        }
        Err(error) => {
            println!("Failed to solve optimization problem: {error}");
        }
    }
}


/*
Mathematical relationships:
Inputs: I0 to I6, U0 to U6
Knobs: P2 to P6, R2 to R6, Pu2 to Pu6, D2 to D6
Outputs: O0 to O6
Maximize min(O0/U0, O1/U1, ..., O6/U6)
Constraint: O, P, R, Pu, D >= 0
O0 = I0 + R2 + R3 + R4 + R5 + R6 - P2 - P3 - P4 - P5 - P6
O1 = I1      - P2 + R2            - 2.0 * Pu2      + 2.0 * D2 + D3     
O2 = I2 + P2 - P3 + R3 - R2 + Pu2 - 2.0 * Pu3 + D3 + 2.0 * D4 + D5 - D2
O3 = I3 + P3 - P4 + R4 - R3 + Pu3 - 2.0 * Pu4 + D5 + 2.0 * D6      - D3
O4 = I4 + P4 - P5 + R5 - R4 + Pu4 - 2.0 * Pu5                      - D4
O5 = I5 + P5 - P6 + R6 - R5 + Pu5 - 2.0 * Pu6                      - D5
O6 = I6 + P6           - R6 + Pu6                                  - D6
*/

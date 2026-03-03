#![allow(dead_code)]

use std::io::{self, Write};
use std::collections::{HashSet, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
enum Metal {
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

    const fn idx(self) -> usize {
        self as usize
    }

    const fn from(idx: usize) -> Self {
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

    fn all() -> [Self; Self::COUNT] {
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

    fn normals() -> [Self; Self::COUNT - 1] {
        [
            Metal::Lead,
            Metal::Tin,
            Metal::Iron,
            Metal::Copper,
            Metal::Silver,
            Metal::Gold,
        ]
    }

    fn next(self) -> Option<Self> {
        match self {
            Metal::Lead => Some(Metal::Tin),
            Metal::Tin => Some(Metal::Iron),
            Metal::Iron => Some(Metal::Copper),
            Metal::Copper => Some(Metal::Silver),
            Metal::Silver => Some(Metal::Gold),
            Metal::Gold | Metal::Quicksilver => None,
        }
    }
    fn prev(self) -> Option<Self> {
        match self {
            Metal::Tin => Some(Metal::Lead),
            Metal::Iron => Some(Metal::Tin),
            Metal::Copper => Some(Metal::Iron),
            Metal::Silver => Some(Metal::Copper),
            Metal::Gold => Some(Metal::Silver),
            Metal::Quicksilver | Metal::Lead => None,
        }
    }
    fn get_split_metals(self) -> Option<(Self, Self)> {
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
struct MetalTransitionKey {
    target: Metal,
    transition_type: Transition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Transition {
    Projection, // Uses one QS to raise a metal to the next level
    Rejection, // Lowers a metal and yields a QS
    Purification, // Turns two metals into one of the next level
    Deposition, // Splits a metal of tier N into two of tiers floor(N/2) and ceil(N/2)
}

#[derive(Clone, Copy)]
struct AvailableTransitions {
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
struct SolveState {
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
    fn from_input(input: &str) -> Result<Self, String> {
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

    fn get(&self, metal: Metal) -> f64 {
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

    fn can_theoretically_reach(&self, target: &SolveState, transitions: &AvailableTransitions) -> bool {
        let reachable_metals = self.theoretically_reachable_metals(transitions);
        Self::target_within_reachable_metals(&reachable_metals, target)
    }
}

struct Solver {
    initial_state: SolveState,
    target_state: SolveState,
    available_transitions: AvailableTransitions,
    metal_transition_potencies: HashMap<MetalTransitionKey, f64>,
}

impl Solver {
    fn new(initial_state: SolveState, target_state: SolveState, available_transitions: AvailableTransitions) -> Self {
        let mut metal_transition_potencies = HashMap::new();
        for &metal in Metal::normals().iter().skip(1) {
            if available_transitions.projection {
                metal_transition_potencies.insert(MetalTransitionKey {
                    target: metal,
                    transition_type: Transition::Projection,
                }, 0.0);
            }
            if available_transitions.rejection {
                metal_transition_potencies.insert(MetalTransitionKey {
                    target: metal,
                    transition_type: Transition::Rejection,
                }, 0.0);
            }
            if available_transitions.purification {
                metal_transition_potencies.insert(MetalTransitionKey {
                    target: metal,
                    transition_type: Transition::Purification,
                }, 0.0);
            }
            if available_transitions.deposition {
                metal_transition_potencies.insert(MetalTransitionKey {
                    target: metal,
                    transition_type: Transition::Deposition,
                }, 0.0);
            }
        }
        Solver {
            initial_state,
            target_state,
            available_transitions,
            metal_transition_potencies,
        }
    }

    fn get_transition_potency(&self, target: Metal, transition_type: Transition) -> f64 {
        let key = MetalTransitionKey { target, transition_type };
        *self.metal_transition_potencies.get(&key).unwrap_or_else(|| panic!("Transition not found for target {:?} and type {:?}", target, transition_type))
    }

    pub fn get_available_keys(&self) -> Vec<MetalTransitionKey> {
        self.metal_transition_potencies.keys().cloned().collect()
    }

    pub fn set_transition_potency(&mut self, target: Metal, transition_type: Transition, potency: f64) {
        if potency < 0.0 {
            panic!("Potency cannot be negative");
        }
        let key = MetalTransitionKey { target, transition_type };
        if let Some(value) = self.metal_transition_potencies.get_mut(&key) {
            *value = potency;
            return;
        }
        panic!("Transition not found for target {:?} and type {:?}", target, transition_type);
    }

    fn calculate_metal_values(&self) -> SolveState {
        let mut metal_values = self.initial_state;
        for (&key, &potency) in self.metal_transition_potencies.iter() {
            let target = key.target;
            let target_index = target.idx();
            
            match key.transition_type {
                Transition::Projection => {
                    metal_values.metals[target_index] += metal_values.get(target) * potency;
                    metal_values.metals[Metal::Quicksilver.idx()] -= metal_values.get(target) * potency;
                    metal_values.metals[target.prev().expect("Cannot project to get quicksilver or lead").idx()] -= metal_values.get(target) * potency;
                }
                Transition::Rejection => {
                    metal_values.metals[target_index] -= metal_values.get(target) * potency;
                    metal_values.metals[Metal::Quicksilver.idx()] += metal_values.get(target) * potency;
                    metal_values.metals[target.prev().expect("Cannot reject quicksilver or lead").idx()] += metal_values.get(target) * potency;
                }
                Transition::Purification => {
                    metal_values.metals[target_index] += metal_values.get(target) * potency;
                    metal_values.metals[target.prev().expect("Cannot purify to get quicksilver or lead").idx()] -= metal_values.get(target) * potency * 2.0;
                }
                Transition::Deposition => {
                    let (metal1, metal2) = target.get_split_metals().expect("Cannot deposit quicksilver or lead");
                    metal_values.metals[target_index] -= metal_values.get(target) * potency;
                    metal_values.metals[metal1.idx()] += metal_values.get(target) * potency;
                    metal_values.metals[metal2.idx()] += metal_values.get(target) * potency;
                }
            }
        }
        metal_values
    }

    pub fn calculate_score(&self) -> f64 {
        let calculated_values = self.calculate_metal_values();
        let mut output = f64::INFINITY;
        for metal in Metal::all() {
            let self_count = calculated_values.get(metal);
            if self_count < 0.0 {
                return f64::NEG_INFINITY; // do not allow negative metals
            }
            let target_count = self.target_state.get(metal);
            if target_count > 0.0 {
                output = output.min(self_count / target_count);
            }
        }
        output
    }
}

fn main() {
    const USE_INTERACTIVE_INPUT: bool = false;
    const MULTIPLICATION_FACTOR: f64 = 1.0;

    let (initial_state, target_state, available_transitions) = if !USE_INTERACTIVE_INPUT {
        // Flip USE_INTERACTIVE_INPUT to false to use this fast test path.
        let initial_state = SolveState::from_input("0 0 0 4 4 4 24").expect("Invalid hardcoded initial state");
        let target_state = SolveState::from_input("0 0 0 5 3 3 3").expect("Invalid hardcoded target state");
        let available_transitions = AvailableTransitions {
            projection: false,
            rejection: true,
            purification: false,
            deposition: true,
        };

        (initial_state, target_state, available_transitions)
    } else {
        println!("Enter initial state (7 numbers for each metal: Quicksilver Lead Tin Iron Copper Silver Gold):");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line");
        let initial_state = SolveState::from_input(&input).expect("Invalid input state");

        println!("Enter target state (7 numbers for each metal: Quicksilver Lead Tin Iron Copper Silver Gold):");
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
        println!("Target is not theoretically reachable with the selected transitions.");
        return;
    }

    let mut solver = Solver::new(initial_state, target_state, available_transitions);


}
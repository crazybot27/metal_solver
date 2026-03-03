#![allow(dead_code)]

use std::io::{self, Write};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

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

#[derive(Debug, Clone, Copy)]
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

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
struct SolveState {
    metals: [usize; Metal::COUNT],
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
    fn get(&self, metal: Metal) -> usize {
        self.metals[metal.idx()]
    }

    fn add(&mut self, metal: Metal, amount: usize) {
        self.metals[metal.idx()] += amount;
    }

    fn try_take(&mut self, metal: Metal, amount: usize) -> bool {
        let slot = &mut self.metals[metal.idx()];
        if *slot < amount {
            return false;
        }
        *slot -= amount;
        true
    }

    fn from_input(input: &str) -> Option<Self> {
        let parts: Vec<_> = input.split_whitespace().collect();
        if parts.len() != Metal::COUNT {
            return None;
        }

        let mut metals = [0; Metal::COUNT];
        for (i, part) in parts.iter().enumerate() {
            metals[i] = part.parse().ok()?;
        }

        Some(SolveState { metals })
    }

    fn apply_projection(&mut self, metal: Metal) -> bool {
        let Some(next_metal) = metal.next() else {
            return false;
        };

        if self.get(Metal::Quicksilver) < 1 || self.get(metal) < 1 {
            return false;
        }

        self.try_take(Metal::Quicksilver, 1);
        self.try_take(metal, 1);
        self.add(next_metal, 1);
        true
    }

    fn apply_rejection(&mut self, metal: Metal) -> bool {
        let Some(prev_metal) = metal.prev() else {
            return false;
        };

        if self.get(metal) < 1 {
            return false;
        }

        self.try_take(metal, 1);
        self.add(Metal::Quicksilver, 1);
        self.add(prev_metal, 1);
        true
    }

    fn apply_purification(&mut self, metal: Metal) -> bool {
        let Some(next_metal) = metal.next() else {
            return false;
        };

        if self.get(metal) < 2 {
            return false;
        }

        self.try_take(metal, 2);
        self.add(next_metal, 1);
        true
    }

    fn apply_deposition(&mut self, metal: Metal) -> bool {
        let Some((metal1, metal2)) = metal.get_split_metals() else {
            return false;
        };

        if self.get(metal) < 1 {
            return false;
        }

        self.try_take(metal, 1);
        self.add(metal1, 1);
        self.add(metal2, 1);
        true
    }

    fn apply_transition(&mut self, transition: Transition, metal: Metal) -> bool {
        match transition {
            Transition::Projection => self.apply_projection(metal),
            Transition::Rejection => self.apply_rejection(metal),
            Transition::Purification => self.apply_purification(metal),
            Transition::Deposition => self.apply_deposition(metal),
        }
    }

    // fn strictly_dominates(&self, other: &SolveState) -> bool {
    //     self.metals
    //         .iter()
    //         .zip(other.metals.iter())
    //         .all(|(self_count, other_count)| self_count > other_count)
    // }

    fn partially_dominates(&self, other: &SolveState) -> bool {
        self.metals
            .iter()
            .zip(other.metals.iter())
            .enumerate()
            .filter(|(i, _)| *i >= 3) // TEMPORARY DEBUG CODE this will make it not count anything lower than iron
            .all(|(_, (self_count, other_count))| self_count >= other_count)
        }

    fn domination_multiple(&self, other: &SolveState) -> Option<f64> {
        let mut threshold = f64::INFINITY;

        for (first_count, second_count) in self.metals.iter().zip(other.metals.iter()) {
            if *first_count == 0 {
                if *second_count > 0 {
                    return None;
                }
                continue;
            }

            threshold = threshold.min(*first_count as f64 / *second_count as f64);
        }

        Some(threshold)
    }

    fn get_metallicity(&self, count_qs: bool, min_metal_to_count: Metal) -> usize {
        // lead and quicksilver have value 1, tin is 2, iron is 3, copper is 4, silver is 5, gold is 6.
        let mut metallicity = 0;
        if count_qs {
            metallicity += self.get(Metal::Quicksilver);
        } 
        for metal in Metal::normals() {
            if metal.idx() >= min_metal_to_count.idx() {
                metallicity += self.get(metal) * (metal.idx());
            }
        }

        metallicity
    }

    fn theoretically_reachable_metals(&self, transitions: &AvailableTransitions) -> HashSet<Metal> {
        let mut available_normal_metals: HashSet<Metal> = self
            .metals
            .iter()
            .enumerate()
            .filter_map(|(i, &count)| if count > 0 { Some(Metal::from(i)) } else { None })
            .collect();

        // quicksilver is special since it can't be purified or deposited, so we track it separately and add it back in at the end if it was available or could be made available by rejection.
        let mut quicksilver_available = self.get(Metal::Quicksilver) > 0;
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
            .filter_map(|(i, &count)| if count > 0 { Some(Metal::from(i)) } else { None })
            .collect();

        desired_metals.is_subset(reachable_metals)
    }

    fn can_theoretically_reach(&self, target: &SolveState, transitions: &AvailableTransitions) -> bool {
        let reachable_metals = self.theoretically_reachable_metals(transitions);
        Self::target_within_reachable_metals(&reachable_metals, target)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct QueueEntry {
    sort_key: usize,
    state: SolveState,
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort_key
            .cmp(&other.sort_key)
    }
}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn all_possible_next_states(state: &SolveState, available_transitions: &AvailableTransitions) -> Vec<(Transition, Metal, SolveState)> {
    let mut transitions = Vec::new();
    for metal in [
        Metal::Lead,
        Metal::Tin,
        Metal::Iron,
        Metal::Copper,
        Metal::Silver,
        Metal::Gold,
    ] {
        for transition in [
            Transition::Projection,
            Transition::Rejection,
            Transition::Purification,
            Transition::Deposition,
        ] {
            let is_available = match transition {
                Transition::Projection => available_transitions.projection,
                Transition::Rejection => available_transitions.rejection,
                Transition::Purification => available_transitions.purification,
                Transition::Deposition => available_transitions.deposition,
            };
            if !is_available {
                continue;
            }

            let mut next = *state;
            if next.apply_transition(transition, metal) {
                transitions.push((transition, metal, next));
            }
        }
    }
    transitions
}

fn reachable_states(initial_state: SolveState, available_transitions: &AvailableTransitions, target: &SolveState) -> Vec<SolveState> {
    let mut visited = HashSet::new();
    let mut queue = BinaryHeap::new();
    let mut frontier = vec![initial_state];

    let count_qs = available_transitions.projection || target.get(Metal::Quicksilver) > 0;
    let lowest_metal_in_target = Metal::from(
        (0..Metal::COUNT)
            .find(|&i| target.metals[i] > 0)
            .expect("Target state must have at least one metal"),
    );
    let min_metal_to_count = 
        if available_transitions.projection {
            Metal::Lead
        } else if lowest_metal_in_target.idx() <= Metal::Tin.idx() && available_transitions.purification {
            Metal::Lead
        } else {
            lowest_metal_in_target
        };

    visited.insert(initial_state);
    queue.push(QueueEntry {
        sort_key: initial_state.get_metallicity(count_qs, min_metal_to_count),
        state: initial_state,
    });
    let mut print_index = 0;
    while let Some(QueueEntry { state, .. }) = queue.pop() {
        print_index += 1;
        if print_index % 20 == 0 {
            print!("\rExploring state: {:?} (frontier size: {})", state, frontier.len());
        }
        let dominated_now = frontier
            .iter()
            .any(|other| *other != state && other.partially_dominates(&state));
        if dominated_now {
            continue;
        }

        for (_, _, next_state) in all_possible_next_states(&state, available_transitions) {
            if !visited.insert(next_state) {
                continue;
            }

            let dominated = frontier
                .iter()
                .any(|other| other.partially_dominates(&next_state));
            if dominated {
                continue;
            }

            frontier.retain(|existing| !next_state.partially_dominates(existing));
            frontier.push(next_state);
            queue.push(QueueEntry {
                sort_key: next_state.get_metallicity(count_qs, min_metal_to_count),
                state: next_state,
            });
        }
    }

    frontier
}

fn best_state_by_domination_multiple(
    reachable: &[SolveState],
    target: &SolveState,
) -> Option<(SolveState, f64)> {
    let mut best: Option<(SolveState, f64)> = None;

    for state in reachable {
        let Some(ratio) = state.domination_multiple(target) else {
            continue;
        };

        match best {
            None => best = Some((*state, ratio)),
            Some((_, best_ratio)) if ratio > best_ratio => best = Some((*state, ratio)),
            _ => {}
        }
    }

    best
}


fn main() {
    const USE_INTERACTIVE_INPUT: bool = false;
    const MULTIPLICATION_FACTOR: f64 = 1.0;

    let (initial_state, target_state, available_transitions) = if !USE_INTERACTIVE_INPUT {
        // Flip USE_INTERACTIVE_INPUT to false to use this fast test path.
        let (quicksilver, lead, tin, iron, copper, silver, gold) = 
        (0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 6.0);
        let initial_state = SolveState {
            metals: [
                (quicksilver * MULTIPLICATION_FACTOR) as usize,
                (lead * MULTIPLICATION_FACTOR) as usize,
                (tin * MULTIPLICATION_FACTOR) as usize,
                (iron * MULTIPLICATION_FACTOR) as usize,
                (copper * MULTIPLICATION_FACTOR) as usize,
                (silver * MULTIPLICATION_FACTOR) as usize,
                (gold * MULTIPLICATION_FACTOR) as usize,
            ],
        };
        // let initial_state = SolveState::from_input("0 0 0 4 4 4 24").expect("Invalid hardcoded initial state");
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
    if available_transitions.projection && available_transitions.rejection {
        println!("Projection and rejection together have already been solved analytically and lead to lots of loops and so are not supported by this tool. Please select a different set of transitions.");
    }

    let reachable = reachable_states(initial_state, &available_transitions, &target_state);

    match best_state_by_domination_multiple(&reachable, &target_state) {
        Some((best_state, ratio)) => {
            println!(
                "Best reachable state for target's ratio: {:?}",
                best_state
            );
            println!("Ratio on target: {:.12}", ratio/MULTIPLICATION_FACTOR);
        }
        None => println!("No reachable state can strictly dominate target by scaling."),
    }
}

// fn main() {
//     let mut possible_transition_sets = Vec::new();
//     for &projection in &[true, false] {
//         for &rejection in &[true, false] {
//             for &purification in &[true, false] {
//                 for &deposition in &[true, false] {
//                     possible_transition_sets.push(AvailableTransitions {
//                         projection,
//                         rejection,
//                         purification,
//                         deposition,
//                     });
//                 }
//             }
//         }
//     }

//     for transitions in &possible_transition_sets {
//         let mut all_quicksilver_starts_reach_every_metal = true;
//         let mut all_non_quicksilver_starts_reach_every_metal = true;
//         let mut all_non_quicksilver_starts_reach_every_non_quicksilver_metal = true;

//         for metal in Metal::normals() {
//             let mut with_quicksilver = SolveState::default();
//             with_quicksilver.add(Metal::Quicksilver, 1);
//             with_quicksilver.add(metal, 1);

//             let reachable_with_quicksilver = with_quicksilver.theoretically_reachable_metals(transitions);
//             let with_quicksilver_reaches_all = Metal::all()
//                 .iter()
//                 .all(|reachable_metal| reachable_with_quicksilver.contains(reachable_metal));
//             if !with_quicksilver_reaches_all {
//                 all_quicksilver_starts_reach_every_metal = false;
//             }

//             let mut without_quicksilver = SolveState::default();
//             without_quicksilver.add(metal, 1);

//             let reachable_without_quicksilver = without_quicksilver.theoretically_reachable_metals(transitions);
//             let without_quicksilver_reaches_all = Metal::all()
//                 .iter()
//                 .all(|reachable_metal| reachable_without_quicksilver.contains(reachable_metal));
//             if !without_quicksilver_reaches_all {
//                 all_non_quicksilver_starts_reach_every_metal = false;
//             }

//             let without_quicksilver_reaches_all_non_quicksilver = Metal::normals()
//                 .iter()
//                 .all(|reachable_metal| reachable_without_quicksilver.contains(reachable_metal));
//             if !without_quicksilver_reaches_all_non_quicksilver {
//                 all_non_quicksilver_starts_reach_every_non_quicksilver_metal = false;
//             }
//         }

//         if all_non_quicksilver_starts_reach_every_metal {
//             println!(
//                 "Transitions {:?}: All reachable",
//                 transitions
//             );
//             continue;
//         }

//         if all_non_quicksilver_starts_reach_every_non_quicksilver_metal {
//             println!(
//                 "Transitions {:?}: All reachable no QS made",
//                 transitions
//             );
//             continue;
//         }
        
//         if all_quicksilver_starts_reach_every_metal {
//             println!(
//                 "Transitions {:?}: All reachable QS needed",
//                 transitions
//             );
//             continue;
//         }

//         println!(
//             "Transitions {:?}: Some unreachable",
//             transitions
//         );
//     }
// }
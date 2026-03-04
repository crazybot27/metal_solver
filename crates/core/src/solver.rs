// use std::collections::HashSet;
use good_lp::{
    constraint, default_solver, variable, variables, Expression, Solution, SolverModel, Variable,
};
use crate::model::*;

/* 
// It's painful to leave this out because of how proud I am of it but the system seems to flow better
// If you just make all the numbers 0 when it's impossible to make a solution.
// This would check which metals can be made with a given set of transitions and initial metals, 
// and if the target state contains any metals that aren't in the reachable set, it would skip trying to solve
// and just tell you it was unsolvable.
impl SolveState {
    fn theoretically_reachable_metals(&self, transitions: &AvailableTransitions) -> HashSet<Metal> {
        let mut available_metals: HashSet<Metal> = self
            .metals
            .iter()
            .enumerate()
            .filter_map(|(i, &count)| if count > 0.0 { Some(Metal::from(i)) } else { None })
            .collect();

        //example of an additional transition (antiquation)
        
        /*
        if transitions.get(Transition::Antiquation) {
            available_metals.insert(Metal::Lead);
            available_metals.insert(Metal::Quicksilver);
        }
        */
        

        // add all metals reachable by deposition from the highest available metal. 
        // If the number of available metals was higher the way we do this would be inaccurate (ie a metal of tier 8 would split into 4,4 which cannot reach 3)
        // but since the highest tier is 6->3 and holes only start at 4+ we can just assume there's no holes)
        if transitions.get(Transition::Deposition)
            && let Some(&max_available) = available_metals.iter().max_by_key(|m| m.idx())
            && let Some((metal1, metal2)) = max_available.get_split_metals()
        {
            let max_deposition_product_idx = metal1.idx().max(metal2.idx());
            for metal in Metal::from(max_deposition_product_idx).get_lower_metals() {
                available_metals.insert(metal);
            }
        }

        // purification can reach any metal as long as low enough metals exist and costs no qs, so we put it early. Deposition goes first since it has "holes" and purification fills them.
        if transitions.get(Transition::Purification)
            && let Some(&min_available) = available_metals.iter().min_by_key(|m| m.idx())
        {
            for metal in Metal::normals() {
                if metal.idx() >= min_available.idx() {
                    available_metals.insert(metal);
                }
            }
        }

        // rejection adds qs if there are higher metals available which is why purification is before it
        if transitions.get(Transition::Rejection)
            && let Some(&max_available) = available_metals.iter().max_by_key(|m| m.idx())
        {
            for metal in Metal::normals() {
                if metal.idx() <= max_available.idx() {
                    available_metals.insert(metal);
                }
            }
            if max_available.idx() > Metal::Lead.idx() {
                available_metals.insert(Metal::Quicksilver);
            }
        }

        // finally, projection needs qs so we need to put the step that could create qs before it. 
        // If you work out all the other relationships, you'll find that either order doesn't matter or this order works.
        // deposition and rejection or purification and projection can be done in any order
        // and projection and deposition need to be in that order so projection can fill in holes in the deposition tree.
        // if you add your own transitions you might need to work this out differently or put it in a loop
        if transitions.get(Transition::Projection)
            && available_metals.contains(&Metal::Quicksilver)
            && let Some(&min_available) = available_metals.iter().min_by_key(|m| m.idx())
        {
            for metal in Metal::normals() {
                if metal.idx() >= min_available.idx() {
                    available_metals.insert(metal);
                }
            }
        }

        if available_metals.contains(&Metal::Quicksilver) {
            available_metals.insert(Metal::Quicksilver);
        }
        available_metals
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
*/

pub fn solve_lp(
    initial: &SolveState,
    target: &SolveState,
    transitions: &AvailableTransitions
) -> Result<OptimalSolution, String> {
    // if !initial.can_theoretically_reach(target, transitions) {
    //     return Err("Target is Unreachable".to_string());
    // }
    let mut problem = variables!();
    // here's where we specify that no transition can be used a negative number of times, since that wouldn't make sense
    let vars: [Variable; Metal::COUNT * Transition::COUNT] =
        std::array::from_fn(|_| problem.add(variable().min(0.0)));

    let ratio = problem.add(variable().min(0.0));

    let transition_vars = |transition: Transition| {
        let start = transition.idx() * Metal::COUNT;
        let end = start + Metal::COUNT;
        &vars[start..end]
    };

    let p = transition_vars(Transition::Projection);
    let r = transition_vars(Transition::Rejection);
    let pu = transition_vars(Transition::Purification);
    let d = transition_vars(Transition::Deposition);
    // let a = transition_vars(Transition::Antiquation); //antiquation

    // p[1] is the number of times projection is used on lead (ID 1), p[2] on tin (ID 2), etc. Same for the other transitions.
    // p[0] is projecting quicksilver, which isn't actually possible, but we include it for ease of indexing

    let projection_terms: [Expression; Metal::COUNT] = [
        - p[1] - p[2] - p[3] - p[4] - p[5], // resulting quicksilver
             - p[1], // resulting lead
        p[1] - p[2], // resulting tin
        p[2] - p[3], // resulting copper
        p[3] - p[4], // resulting iron
        p[4] - p[5], // resulting silver
        p[5].into(), // resulting gold

    ];

    let rejection_terms: [Expression; Metal::COUNT] = [
        r[2] + r[3] + r[4] + r[5] + r[6],
        r[2].into(), // lead is increased by one when we reject tin
        r[3] - r[2], // tin is increased by one when we reject iron but decreased by one when we reject tin, so net is r[3] - r[2]
        r[4] - r[3],
        r[5] - r[4],
        r[6] - r[5],
             - r[6], // gold is decreased by one when we reject it, and there's no higher metal to increase it, so net is just -r[6]
    ];

    let purification_terms: [Expression; Metal::COUNT] = [
        0.0.into(), // quicksilver is unaffected by purification
              - 2.0 * pu[1],
        pu[1] - 2.0 * pu[2],
        pu[2] - 2.0 * pu[3],
        pu[3] - 2.0 * pu[4],
        pu[4] - 2.0 * pu[5],
        pu[5].into(),
    ];

    let deposition_terms: [Expression; Metal::COUNT] = [
        0.0.into(), // quicksilver is unaffected by deposition
                        (2.0*d[2]) + d[3],
        // here, tin is reduced by one when you deposit it, increased by one when you deposit iron, by two when copper, and one more for silver
        - d[2] + d[3] + (2.0*d[4]) + d[5], 
        - d[3] + d[5] + (2.0*d[6]),
        - d[4],
        - d[5],
        - d[6],
    ];

    
    // Example for Antiquation. The equations are set up so that anything you use it on (including quicksilver) turns into lead, and lead turns into quicksilver
    /*
    let antiquation_terms: [Expression; Metal::COUNT] = [
        - a[0] + a[1], // for every time antiquation is used on quicksilver, you lose one quicksilver. For every time antiquiation is used on lead, you gain one quicksilver
        - a[1] + a[0] + a[2] + a[3] + a[4] + a[5] + a[6],
        - a[2], // for every time you use antiquation on the element with ID 2 (tin), you lose one tin (tin's ID is 2 so it ends up at the third place in the array, because it counts from 0)
        - a[3],
        - a[4],
        - a[5],
        - a[6],
    ];
    */
    // I hope I explained this well enough
    

    
    let initial_metals = initial.metals;
    let mut output_expressions = vec![];
    for idx in 0..Metal::COUNT {
        let output = initial_metals[idx]
            + projection_terms[idx].clone() 
            + rejection_terms[idx].clone() 
            + purification_terms[idx].clone() 
            + deposition_terms[idx].clone()
            // + antiquation_terms[idx].clone() // just add the new terms in the same way as the others
        ;
        output_expressions.push(output);
    }

    // this is where we specify we want to maximize the ratio variable
    let mut model = problem.maximise(ratio).using(default_solver);

    // this is where we specify you can never have a result of negative amount of any metal, since that would mean using metals that don't exist
    for output in &output_expressions {
        model = model.with(constraint!(output.clone() >= 0.0));
    }
    
    // this is where we specify that in order for the model to say it found a ratio of 5, it needs 5 times the metals being asked for for each metal
    let mut has_target_component = false;
    for (idx, output) in output_expressions.iter().enumerate() {
        let required = target.metals[idx];
        if required > 0.0 {
            has_target_component = true;
            model = model.with(constraint!(output.clone() >= ratio * required));
        }
    }

    if !has_target_component {
        return Err("Nothing to make".to_string());
    }

    // here's where we specify that if a transition is not available, you can't use it
    for (transition_idx, transition) in Transition::all().iter().enumerate() {
        if !transitions.get(*transition) {
            for var in &vars[transition_idx*Metal::COUNT..(transition_idx+1)*Metal::COUNT] {
                model = model.with(constraint!(*var == 0.0));
            }
        }
    }

    let solution = model
        .solve()
        .map_err(|e| format!("Solve failed: {e}"))?;

    let values: [f64; Transition::COUNT * Metal::COUNT] = std::array::from_fn(|idx| {
        solution.value(vars[idx])
    });
    let ratio_value = solution.value(ratio);
    let outputs: [f64; Metal::COUNT] = std::array::from_fn(|idx| {
        solution.eval(output_expressions[idx].clone())
    });

    Ok(OptimalSolution {
        ratio: ratio_value,
        outputs,
        values
    })
}

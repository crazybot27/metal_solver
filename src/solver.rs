use std::collections::HashSet;
use good_lp::{
    constraint, default_solver, variable, variables, Expression, Solution, SolverModel, Variable,
};
use crate::model::*;

impl SolveState {
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

impl UI {
    pub fn solve(&mut self) {
        match solve_lp(
            &self.inputs,
            &self.target,
            &self.available_transitions
        ) {
            Ok(solution) => {
                self.solution = Some(solution);
            }
            Err(error) => {
                println!("Failed to solve: {error}");
            }
        }
    }
}
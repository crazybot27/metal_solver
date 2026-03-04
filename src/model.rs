
use std::fmt::Debug;

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
#[allow(dead_code)]
impl Metal {
    pub const COUNT: usize = 7;

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

    pub const fn get_next(self) -> Option<Self> { 
        //what you'd get if you purified/projected a metal
        match self {
            Metal::Lead => Some(Metal::Tin),
            Metal::Tin => Some(Metal::Iron),
            Metal::Iron => Some(Metal::Copper),
            Metal::Copper => Some(Metal::Silver),
            Metal::Silver => Some(Metal::Gold),
            Metal::Gold | Metal::Quicksilver => None,
        }
    }

    pub const fn get_prev(self) -> Option<Self> { 
        // same but rejection
        match self {
            Metal::Tin => Some(Metal::Lead),
            Metal::Iron => Some(Metal::Tin),
            Metal::Copper => Some(Metal::Iron),
            Metal::Silver => Some(Metal::Copper),
            Metal::Gold => Some(Metal::Silver),
            Metal::Quicksilver | Metal::Lead => None,
        }
    }

    pub fn get_higher_metals(self) -> Vec<Self> {
        // if you had infinite of this metal, what could you get by purifying/projecting
        let mut metals = Vec::new();
        let mut current = self.get_next();
        while let Some(metal) = current {
            metals.push(metal);
            current = metal.get_next();
        }
        metals
    }

    pub fn get_lower_metals(self) -> Vec<Self> {
        // if you had infinite of this metal, what could you get by rejecting
        let mut metals = Vec::new();
        let mut current = self.get_prev();
        while let Some(metal) = current {
            metals.push(metal);
            current = metal.get_prev();
        }
        metals
    }

    pub const fn get_split_metals(self) -> Option<(Self, Self)> { 
        // same but deposition, actually used for caluclating possible metals you can reach with a given set of transitions
        // if you add an eigth normal metal or higher, please review theoretically_reachable_metals in solver.rs 
        // to make sure it accounts for the fact that depositing a metal of value 8 does not allow you to reach 3
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
#[repr(u8)]
pub enum Transition {
    Projection = 0, // Uses one QS to raise a metal to the next level
    Rejection = 1, // Lowers a metal and yields a QS
    Purification = 2, // Turns two metals into one of the next level
    Deposition = 3, // Splits a metal of tier N into two of tiers floor(N/2) and ceil(N/2)
    // Antiquation = 4, // Example extra transition
}

impl Transition {
    pub const COUNT: usize = 4;
    // pub const COUNT: usize = 5; //Antiquation

    pub const fn idx(self) -> usize {
        self as usize
    }

    pub const fn from(idx: usize) -> Self {
        match idx {
            0 => Transition::Projection,
            1 => Transition::Rejection,
            2 => Transition::Purification,
            3 => Transition::Deposition,
            // 4 => Transition::Antiquation,
            _ => panic!("Invalid transition index"),
        }
    }
    
    pub const fn all() -> [Self; Self::COUNT] {
        [
            Transition::Projection,
            Transition::Rejection,
            Transition::Purification,
            Transition::Deposition,
            // Transition::Antiquation,
        ]
    }

    pub const fn name(self) -> &'static str {
        match self {
            Transition::Projection => "Projection",
            Transition::Rejection => "Rejection",
            Transition::Purification => "Purification",
            Transition::Deposition => "Deposition",
            // Transition::Antiquation => "Antiquation",
        }
    }

    pub const fn short_name(self) -> &'static str {
        match self {
            Transition::Projection => "Pro",
            Transition::Rejection => "Rej",
            Transition::Purification => "Pur",
            Transition::Deposition => "Dep",
            // Transition::Antiquation => "Ant",
        }
    }

    pub const fn valid_targets(self) -> [bool; Metal::COUNT] {
        // determines which fields in the UI will show up. This has no effect on the solving logic
        match self {
            Transition::Projection => [false, true, true, true, true, true, false], //You can project anything other than QS and Gold
            Transition::Rejection => [false, false, true, true, true, true, true], //You can reject anything tin or above other than QS
            Transition::Purification => [false, true, true, true, true, true, false], //You can purify anything tin or above other than QS and Gold
            Transition::Deposition => [false, false, true, true, true, true, true], //You can deposit anything tin or above other than QS
            // Transition::Antiquation => [true, true, true, true, true, true, true], //Example transition that can be applied to any metal
        }
    }
    pub const fn is_valid_target(self, metal: Metal) -> bool {
        self.valid_targets()[metal.idx()]
    }
}

#[derive(Clone, Copy)]
pub struct AvailableTransitions {
    transitions: [bool; Transition::COUNT],
}

impl AvailableTransitions {
    pub fn get(&self, transition: Transition) -> bool {
        self.transitions[transition as usize]
    }

    pub fn set(&mut self, transition: Transition, value: bool) {
        self.transitions[transition as usize] = value;
    }

    pub fn from_names(input: &str) -> Result<Self, String> {
        if input == "All" {
            return Ok(AvailableTransitions { transitions: [true; Transition::COUNT] });
        } else if input == "None" {
            return Ok(AvailableTransitions { transitions: [false; Transition::COUNT] });
        }
        let mut transitions = [false; Transition::COUNT];
        let long_names = Transition::all().map(|t| t.name());
        let short_names = Transition::all().map(|t| t.short_name());
        for part in input.split_whitespace() {
            let transition = long_names.iter().chain(short_names.iter()).position(|&name| name == part)
                .ok_or_else(|| format!("Invalid transition name: {}", part))?;
            transitions[transition] = true;
        }
        Ok(AvailableTransitions { transitions })
    }

    pub fn from_bool_string(input: &str) -> Result<Self, String> {
        let mut transitions = [false; Transition::COUNT];
        for (i, part) in input.split_whitespace().enumerate() {
            if i >= Transition::COUNT {
                return Err(format!("Too many values for transitions: expected {}, got {}", Transition::COUNT, i + 1));
            }
            match part.to_ascii_lowercase().as_str() {
                "1" | "y" | "true"=> transitions[i] = true,
                "0" | "n" | "false" => transitions[i] = false,
                _ => return Err(format!("Invalid value for transition {}: {}", i, part)),
            }
        }
        Ok(AvailableTransitions { transitions })
    }

    pub fn from_input(input: &str) -> Result<Self, String> {
        Self::from_bool_string(input).or_else(|_| Self::from_names(input))
    }
}

impl std::fmt::Debug for AvailableTransitions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut transitions = Vec::new();
        for i in 0..Transition::COUNT {
            let transition = Transition::from(i);
            if self.get(transition) {
                transitions.push(transition.short_name());
            }
        }
        let output = match transitions.len() {
            0 => "None".to_string(),
            Transition::COUNT => "All".to_string(),
            _ => transitions.join(", "),
        };
        write!(f, "[{}]", output)
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct SolveState {
    pub metals: [f64; Metal::COUNT],
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
}

pub struct OptimalSolution {
    pub ratio: f64,
    pub outputs: [f64; Metal::COUNT],
    pub values: [f64; Metal::COUNT * Transition::COUNT],
}

impl OptimalSolution {
    pub fn get_transition_value(&self, transition: Transition, metal: Metal) -> Option<f64> {
        if transition.is_valid_target(metal) {
            Some(self.values[transition.idx() * Metal::COUNT + metal.idx()])
        } else {
            None
        }
    }
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

        for transition in Transition::all() {
            let transition_values = &self.values[transition.idx() * Metal::COUNT..(transition.idx() + 1) * Metal::COUNT];
            if transition_values.iter().all(|&x| x == 0.0) {
                potencies_string += &format!("- {}: N/A\n", transition.name());
            } else {
                potencies_string += &format!("- {} on:\n", transition.name());
                for (index, value) in transition_values.iter().enumerate() {
                    if *value == 0.0 {continue;}
                    potencies_string += &format!("  - {:?}: {} ({})\n", Metal::from(index), decimal_to_fraction(*value), format_rounded(*value, 5));
                }
            }
        }
        write!(f, "{}{}", outputs_string, potencies_string)
    }
}

fn check_repeating(digits: &str, repeat_start: usize, repeat_length: usize) -> bool {
    for i in 0..repeat_length {
        let current_digit = digits.chars().nth(repeat_start + i).unwrap_or(' ');
        for j in (repeat_start + repeat_length + i..digits.len()).step_by(repeat_length) {
            if digits.chars().nth(j).unwrap_or(' ') != current_digit {
                return false;
            }
        }
    }
    true
}

pub fn format_rounded(value: f64, max_digits: usize) -> String {
    if value.round() == value || max_digits == 0 {
        return format!("{:.0}", value);
    }
    let value_string = value.to_string();
    let decimals = value_string.split('.').nth(1).unwrap_or("").chars().take(10).collect::<String>();
    for repeat_length in 1..=decimals.len() / 2 {
        for repeat_start in 0..repeat_length {
            if check_repeating(&decimals, repeat_start, repeat_length) {
                let length = (
                    repeat_start + repeat_length.max(2)
                    ).min(max_digits);
                return format!("{:.1$}", value, length);
            }
        }
    }
    let decimals_to_show = decimals.len().min(max_digits);
    format!("{:.1$}", value, decimals_to_show)
}

pub fn decimal_to_fraction(value: f64) -> String {
    let tolerance = 1e-6;
    let mut numerator = 1;
    let mut denominator = 1;
    if (value.round() - value).abs() < 1e-7 {
        return format!("{:.0}", value);
    }

    while (numerator as f64 / denominator as f64 - value).abs() > tolerance {
        if numerator > 1000 || denominator > 1000 {
            return format_rounded(value, 5);
        }
        if (numerator as f64) / (denominator as f64) < value {
            numerator += 1;
        } else {
            denominator += 1;
        }
    }
    if denominator == 1 {
        format!("{}", numerator)
    } else {
        format!("{}/{}", numerator, denominator)
    }
}
use std::fmt::Debug;
use regex::Regex;

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
        // same but division, actually used for caluclating possible metals you can reach with a given set of transitions
        // if you add an eigth normal metal or higher, please review theoretically_reachable_metals in solver.rs 
        // to make sure it accounts for the fact that dividing a metal of value 8 does not allow you to reach 3
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

    pub const fn name(self) -> &'static str {
        match self {
            Metal::Quicksilver => "Quicksilver",
            Metal::Lead => "Lead",
            Metal::Tin => "Tin",
            Metal::Iron => "Iron",
            Metal::Copper => "Copper",
            Metal::Silver => "Silver",
            Metal::Gold => "Gold",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.to_ascii_lowercase();
        Self::all().iter().find(|m| m.name().to_ascii_lowercase() == name).copied()
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Transition {
    Projection = 0, // Uses one QS to raise a metal to the next level
    Rejection = 1, // Lowers a metal and yields a QS
    Purification = 2, // Turns two metals into one of the next level
    Deposition = 3, // Splits a metal of tier N into two of tiers floor(N/2) and ceil(N/2)
    Proliferation = 4, //Turns a QS into another metal
    // Antiquation = 4, // Example extra transition
}

impl Transition {
    pub const COUNT: usize = 5;
    // pub const COUNT: usize = 6; //Antiquation

    pub const fn idx(self) -> usize {
        self as usize
    }

    pub const fn from(idx: usize) -> Self {
        match idx {
            0 => Transition::Projection,
            1 => Transition::Rejection,
            2 => Transition::Purification,
            3 => Transition::Deposition,
            4 => Transition::Proliferation,
            // 4 => Transition::Antiquation,
            _ => panic!("Invalid transition index"),
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.to_ascii_lowercase();
        if name == "dep" || name == "deposition" {
            return Some(Transition::Division); // special case for legacy name of division
         }
        Self::all().iter().find(|t| t.name().to_ascii_lowercase() == name || t.short_name().to_ascii_lowercase() == name).copied()
    }
    
    pub const fn all() -> [Self; Self::COUNT] {
        [
            Transition::Projection,
            Transition::Rejection,
            Transition::Purification,
            Transition::Deposition,
            Transition::Proliferation,
            // Transition::Antiquation,
        ]
    }

    pub const fn name(self) -> &'static str {
        match self {
            Transition::Projection => "Projection",
            Transition::Rejection => "Rejection",
            Transition::Purification => "Purification",
            Transition::Deposition => "Deposition",
            Transition::Proliferation => "Proliferation",
            // Transition::Antiquation => "Antiquation",
        }
    }

    pub const fn short_name(self) -> &'static str {
        match self {
            Transition::Projection => "Prj",
            Transition::Rejection => "Rej",
            Transition::Purification => "Pur",
            Transition::Deposition => "Dep",
            Transition::Proliferation => "Plf",
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
            Transition::Proliferation => [false, true, true, true, true, true, true], //You can proliferate QS into anything other than QS
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

    pub fn from_input(input: &str) -> Result<Self, String> {
        // supported input formats:
        // "1011" (values in order)
        // "t y true yes" (any combination of boolean words) 
        // "truefalse,nono" (no spaces needed, other spacers optional)
        // "Divisionprojection" (named transitions in any order with or without spaces, also accepts short names like "Pro, Div")
        // "all" or "none" (cannot be combined with anything else, sets all transitions on or off respectively)
        // "0001 proj rej" (each name adds a transition. If you include any values you must either include all four, or the exact number needed in addition to the names)
        // "10 rej pur" means projection is on from the values, rejection and purification are on because of the names, and division is off
        let mut sanitized = input.to_ascii_lowercase().chars()
        .filter(|c| c.is_ascii_alphanumeric()).collect::<String>();

        if sanitized == "all" {
            return Ok(Self { transitions: [true; Transition::COUNT] });
        }
        if sanitized == "none" {
            return Ok(Self { transitions: [false; Transition::COUNT] });
        }

        let mut transitions = [false; Transition::COUNT];

        for transition in Transition::all() {
            let name = transition.name().to_ascii_lowercase();
            let short_name = transition.short_name().to_ascii_lowercase();
            if sanitized.contains(&name) {
                transitions[transition.idx()] = true;
                sanitized = sanitized.replace(&name, "");
            } else if sanitized.contains(&short_name) {
                transitions[transition.idx()] = true;
                sanitized = sanitized.replace(&short_name, "");
            }
        }
        let re = Regex::new(r"(true|t|yes|y|1)").unwrap();
        sanitized = re.replace_all(&sanitized, "1").to_string();
        let re = Regex::new(r"(false|f|no|n|0)").unwrap();
        sanitized = re.replace_all(&sanitized, "0").to_string();
        if !sanitized.chars().all(|c| c == '0' || c == '1') {
            return Err(format!("Invalid alphabetical characters in input. Only 0, 1, true, false, yes, no, t, f, y, n, and transition short or long names are allowed.\n Additional characters: {}", sanitized));
        }
        let named_count = transitions.iter().filter(|&&x| x).count();
        let value_count = sanitized.len();
        if value_count > 0 && value_count != Transition::COUNT && (value_count + named_count != Transition::COUNT) {
            return Err(format!("If you include any 0/1 values, you must include either all {} values or exactly the number of values needed to supplement the names you included. Found {} values and {} names.", Transition::COUNT, value_count, named_count));
        }
        for (i, c) in sanitized.chars().enumerate() {
            transitions[i] = transitions[i] || c == '1';
        }
        Ok(Self { transitions })
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
        // supported input formats:
        // "1.5 2 3 4 5 6 7" (space-separated values in order of metals)
        // "Quicksilver: 1.5, Lead: 2, Tin: 3, Iron: 4, Copper: 5, Silver: 6, Gold: 7" (named values, order doesn't matter)
        // [1.5,2,3,4,5,6,7] (JSON array)
        // {"Quicksilver": 1.5, "Lead": 2, "Tin": 3, "Iron": 4, "Copper": 5, "Silver": 6, "Gold": 7} (JSON object)
        // ["1.5","2","3","4","5","6","7"] (JSON array of strings)
        // {"Quicksilver": "1.5", "Lead": "2", "Tin": "3", "Iron": "4", "Copper": "5", "Silver": "6", "Gold": "7"} (JSON object with string values)
        // technical rules for parsing:
        // - step 1: remove anything that isn't a letter, number, comma, period, or space, and lowercase everything
        // - step 2: replace any sequence of whitespace or commas with a single space, and trim leading/trailing whitespace
        // - step 3: anytime we see the name of a metal followed by a number, we assign that number to the metal. 
        // - step 4: any remaining numbers get assigned to the next unassigned metal
        // this means that for example `  iROn##4,,, 1 2, 3 5, 6gold = "7"` would be parsed as Iron=4, Quicksilver=1, Lead=2, Tin=3, Copper=5, Silver=6, Gold=7
        
        let mut metals = [-1.0; Metal::COUNT];
        // `  iROn##4,,, 1 2, 3 5, 6gold = "7"`
        let mut sanitized = input.to_ascii_lowercase();
        // `  iron##4,,, 1 2, 3 5, 6gold = "7"`
        sanitized = sanitized.chars().filter(|c| c.is_ascii_alphanumeric() || *c == ',' || *c == '.' || *c == '-' || c.is_whitespace()).collect();
        // `  iron4,,, 1 2, 3 5, 6gold 7`
        let re = Regex::new(r"([a-zA-Z])(-?\d)").unwrap();
        sanitized = re.replace_all(&sanitized, "$1 $2").to_string(); 
        // `  iron 4,,, 1 2, 3 5, 6gold 7`
        let re = Regex::new(r"(\d)([a-zA-Z])").unwrap();
        sanitized = re.replace_all(&sanitized, "$1 $2").to_string();
        // `  iron 4,,, 1 2, 3 5, 6 gold 7`
        let re = Regex::new(r"[,\s]+").unwrap();
        sanitized = re.replace_all(&sanitized, " ").trim().to_string();
        // `iron 4 1 2 3 5 6 gold 7`

        let parts: Vec<&str> = sanitized.split_whitespace().collect();
        let mut assign_metal = None;
        for part in parts {
            if let Some(metal) = Metal::from_name(part) {
                assign_metal = Some(metal);
            } else if let Ok(value) = part.parse::<f64>() {
                if value < 0.0 {
                    return Err(format!("Negative values are not allowed: {}", value));
                }
                if let Some(metal) = assign_metal {
                    metals[metal.idx()] = value;
                    assign_metal = None;
                } else {
                    // assign to next unassigned metal
                    if let Some(next_metal) = Metal::all().iter().find(|m| metals[m.idx()] == -1.0) {
                        metals[next_metal.idx()] = value;
                    } else {
                        return Err(format!("Too many values: all metals already assigned, but got extra value {}. \n Current state: {:?}", value, SolveState { metals }));
                    }
                }
            } else {
                return Err(format!("Invalid part in input: {}. The only alphabetical characters should be full metal names.", part));
            }
        } 
        metals.iter().enumerate().try_for_each(|(i, &value)| {
            if value < 0.0 {
                Err(format!("Missing value for metal {}: no value assigned in input", Metal::from(i).name()))
            } else {
                Ok(())
            }
        })?;
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

    pub fn to_json_string(&self, use_names: bool, pretty_values: bool) -> String {
        let mut output = String::from("");

        let ratio = if pretty_values {
            format!("\"{} ({})\"", decimal_to_fraction(self.ratio), format_rounded(self.ratio, 4))
        } else {
            self.ratio.to_string()
        };
        output.push_str(&format!("{{\"ratio\":{},", ratio));

        let mut metal_outputs_string = String::from("\"outputs\":");
        if use_names {metal_outputs_string.push('{');} else {metal_outputs_string.push('[');}
        for (idx, metal) in Metal::all().iter().enumerate() {
            if idx > 0 {
                metal_outputs_string.push(',');
            }
            if use_names {
                metal_outputs_string.push('"');
                metal_outputs_string.push_str(&format!("{:?}", metal));
                metal_outputs_string.push_str("\":");
            }
            let output_value = if pretty_values {
                format!("\"{}\"", decimal_to_fraction(self.outputs[metal.idx()]))
            } else {
                self.outputs[metal.idx()].to_string()
            };
            metal_outputs_string.push_str(&output_value);
        }
        if use_names {metal_outputs_string.push_str("},");} else {metal_outputs_string.push_str("],");}
        output.push_str(&metal_outputs_string);

        let mut transitions_string = String::from("\"transitions\":");
        if use_names {transitions_string.push('{');} else {transitions_string.push('[');}
        for (t_idx, transition) in Transition::all().iter().enumerate() {
            if t_idx > 0 {
                transitions_string.push(',');
            }
            if use_names {
                transitions_string.push('"');
                transitions_string.push_str(transition.name());
                transitions_string.push_str("\":");
            }
            let mut transition_values_string = String::new();
            if use_names {transition_values_string.push('{');} else {transition_values_string.push('[');}
            for (m_idx, metal) in Metal::all().iter().enumerate() {
                if m_idx > 0 {
                    transition_values_string.push(',');
                }
                if use_names {
                    transition_values_string.push('"');
                    transition_values_string.push_str( metal.name());
                    transition_values_string.push_str("\":");
                }
                if let Some(value) = self.get_transition_value(*transition, *metal) {
                    let value_string = if pretty_values {
                        format!("\"{}\"", decimal_to_fraction(value))
                    } else {
                        value.to_string()
                    };
                    transition_values_string.push_str(&value_string);
                } else {
                    transition_values_string.push_str("null");
                }
            }
            if use_names {transition_values_string.push('}');} else {transition_values_string.push(']');}
            transitions_string.push_str(&transition_values_string);
        }
        if use_names {transitions_string.push('}');} else {transitions_string.push(']');}
        output.push_str(&transitions_string);
        output.push('}');
        output
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
    if (value.round() - value).abs() < 10f64.powi(-(max_digits as i32)) || max_digits == 0 {
        return format!("{:.0}", value.round().abs());
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

// adapted from https://ics.uci.edu/~eppstein/numth/frap.c
pub fn decimal_to_fraction(value: f64) -> String {

    let tolerance = 1e-8;
    let maxden: i64 = 10000;

    let mut x = value;
    let mut a: i64 = 0;
    let mut b: i64 = 1;
    let mut c: i64 = 1;
    let mut d: i64 = 0;
    let mut t: i64;
    let mut ai: i64;

    while (c * (x as i64) + d) <= maxden {
        ai = x as i64;
        t = a * ai + b;
        b = a;
        a = t;
        t = c * ai + d;
        d = c;
        c = t;
        if (x - (ai as f64)).abs() < tolerance {
            break;
        }
        x = 1.0/(x - (ai as f64));
    }

    if a == 1 {
        format!("{}", c)
    } else {
        format!("{}/{}", c, a)
    }
}
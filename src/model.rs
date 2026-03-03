use std::fmt::Debug;
use crate::ui::CachedTextSizer;
use macroquad::prelude::Texture2D;

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
    pub projection: bool,
    pub rejection: bool,
    pub purification: bool,
    pub deposition: bool,
}

impl AvailableTransitions {
    pub fn get(&self, transition: Transition) -> bool {
        match transition {
            Transition::Projection => self.projection,
            Transition::Rejection => self.rejection,
            Transition::Purification => self.purification,
            Transition::Deposition => self.deposition,
        }
    }
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
    pub projection: [f64; 5],
    pub rejection: [f64; 5],
    pub purification: [f64; 5],
    pub deposition: [f64; 5],
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

pub fn format_rounded(value: f64, digits: usize) -> String {
    if (value * 10.0).round() == (value * 10.0) {
        format!("{:.1}", value)
    } else {
        format!("{:.1$}", value, digits)
    }
}

pub fn decimal_to_fraction(value: f64) -> String {
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

pub struct UI {
    pub text_renderer: CachedTextSizer,
    pub textures: Vec<Texture2D>,
    pub inputs: SolveState,
    pub target: SolveState,
    pub available_transitions: AvailableTransitions,
    pub solution: Option<OptimalSolution>,
}


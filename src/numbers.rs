use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rational {
    numerator: u32,
    denominator: u32,
}

impl Rational {
    pub fn new(numerator: u32, denominator: u32) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        Some(Rational {
            numerator,
            denominator,
        })
    }

    pub fn numerator(&self) -> u32 {
        self.numerator
    }

    pub fn denominator(&self) -> u32 {
        self.denominator
    }

    pub fn to_float(&self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    // multiplies without overflowing
    pub fn mul(&self, value: u64) -> u64 {
        (self.numerator as u64)
            .saturating_mul(value)
            .saturating_div(self.denominator as u64)
    }
}

impl Default for Rational {
    fn default() -> Self {
        Rational {
            numerator: 1,
            denominator: 1,
        }
    }
}

impl std::fmt::Display for Rational {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.numerator, self.denominator)
    }
}

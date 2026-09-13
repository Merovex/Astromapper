use serde::{Deserialize, Serialize};
use std::fmt;

/// Traveller extended hex (skips I and O), matching the Ruby/Go implementations.
const EHEX: &[u8] = b"0123456789ABCDEFGHJKLMNPQRSTUVWXYZ";

pub fn ehex(n: u8) -> char {
    let i = n as usize;
    if i < EHEX.len() {
        EHEX[i] as char
    } else {
        EHEX[EHEX.len() - 1] as char
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub name: String,
    pub uwp: String,
    pub starport: char,
    pub size: u8,
    pub atmosphere: u8,
    pub hydrographics: u8,
    pub population: u8,
    pub government: u8,
    pub law_level: u8,
    pub tech_level: u8,
    pub temperature: Temperature,
    pub bases: Vec<Base>,
    pub trade_codes: Vec<String>,
    pub factions: Vec<String>,
    pub gas_giant: bool,
    pub row: usize,
    pub col: usize,

    // Traveller 5 extensions (unset when the ruleset's extensions module is "none").
    #[serde(default)]
    pub extended: bool,
    #[serde(default)]
    pub ix: i64,
    #[serde(default)]
    pub ex: [i64; 4], // Resources, Labor, Infrastructure, Efficiency
    #[serde(default)]
    pub cx: [i64; 4], // Homogeneity, Acceptance, Strangeness, Symbols
    #[serde(default)]
    pub ru: i64,
    #[serde(default)]
    pub native: String,
    #[serde(default)]
    pub pop_multiplier: u8, // 1-9 (0 if unpopulated); the P in PBG
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Temperature {
    Frozen,
    Cold,
    Temperate,
    Hot,
    Roasting,
    Twilight, // T5 HZ inner orbit
    Locked,
}

impl Temperature {
    pub fn to_code(&self) -> &'static str {
        match self {
            Temperature::Frozen => "F",
            Temperature::Cold => "C",
            Temperature::Temperate => "T",
            Temperature::Hot => "H",
            Temperature::Roasting => "R",
            Temperature::Twilight => "Tz",
            Temperature::Locked => "Lk",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Base {
    Naval,
    Scout,
    Research,
    TAS,
    Imperial,
    PirateBase,
    Depot,
    Way,
}

impl Base {
    pub fn to_code(&self) -> &'static str {
        match self {
            Base::Naval => "N",
            Base::Scout => "S",
            Base::Research => "R",
            Base::TAS => "T",
            Base::Imperial => "I",
            Base::PirateBase => "P",
            Base::Depot => "D",
            Base::Way => "W",
        }
    }
}

impl World {
    pub fn new(row: usize, col: usize) -> Self {
        World {
            name: String::new(),
            uwp: String::new(),
            starport: 'X',
            size: 0,
            atmosphere: 0,
            hydrographics: 0,
            population: 0,
            government: 0,
            law_level: 0,
            tech_level: 0,
            temperature: Temperature::Temperate,
            bases: Vec::new(),
            trade_codes: Vec::new(),
            factions: Vec::new(),
            gas_giant: false,
            row,
            col,
            extended: false,
            ix: 0,
            ex: [0; 4],
            cx: [0; 4],
            ru: 0,
            native: String::new(),
            pop_multiplier: 0,
        }
    }

    pub fn coords(&self) -> String {
        format!("{:02}{:02}", self.col + 1, self.row + 1)
    }

    pub fn update_uwp(&mut self) {
        self.uwp = format!(
            "{}{}{}{}{}{}{}-{}",
            self.starport,
            ehex(self.size),
            ehex(self.atmosphere),
            ehex(self.hydrographics),
            ehex(self.population),
            ehex(self.government),
            ehex(self.law_level),
            ehex(self.tech_level)
        );
    }

    /// Travel zone (Ruby World#travel_code). T5 leaves zones to the referee; this
    /// auto-assigns: Red for the most oppressive/controlled worlds, Amber for caution.
    pub fn travel_code(&self) -> &'static str {
        if self.law_level >= 15 || self.government >= 15 {
            return "RZ";
        }
        if self.atmosphere > 9
            || matches!(self.government, 0 | 7 | 10)
            || self.law_level == 0
            || (9..=14).contains(&self.law_level)
        {
            return "AZ";
        }
        ".."
    }

    pub fn bases_string(&self) -> String {
        if self.bases.is_empty() {
            ".".to_string()
        } else {
            self.bases.iter().map(|b| b.to_code()).collect::<Vec<_>>().join("")
        }
    }

    pub fn trade_codes_string(&self) -> String {
        if self.trade_codes.is_empty() {
            ".".to_string()
        } else {
            self.trade_codes.join(" ")
        }
    }

    /// T5 extension block: { +Ix } (RLI±E) [HASS] RU:n (empty when not extended).
    pub fn extensions(&self) -> String {
        if !self.extended {
            return String::new();
        }
        format!(
            "{{ {:+} }} ({}{}{}{:+}) [{}{}{}{}] RU:{}",
            self.ix,
            ehex(self.ex[0] as u8),
            ehex(self.ex[1] as u8),
            ehex(self.ex[2] as u8),
            self.ex[3],
            ehex(self.cx[0] as u8),
            ehex(self.cx[1] as u8),
            ehex(self.cx[2] as u8),
            ehex(self.cx[3] as u8),
            self.ru
        )
    }
}

impl fmt::Display for World {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {} {}",
            self.coords(),
            self.uwp,
            self.temperature.to_code(),
            self.bases_string(),
            self.trade_codes_string()
        )
    }
}

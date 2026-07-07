use serde::{Deserialize, Serialize};
use crate::models::World;
use crate::models::world::ehex;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OrbitType {
    Empty,
    World,
    GasGiant,
    Belt,
    Hostile,
    Rockball,
    Companion,
}

pub trait Orbit {
    fn orbit_number(&self) -> u8;
    fn au(&self) -> f64;
    fn orbit_type(&self) -> OrbitType;
    fn to_ascii(&self) -> String;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrbitContent {
    Empty(EmptyOrbit),
    World(WorldOrbit),
    GasGiant(GasGiant),
    Belt(Belt),
    Hostile(Hostile),
    Rockball(Rockball),
    Companion(CompanionOrbit),
}

impl OrbitContent {
    /// The single-letter orbit crib code (Ruby `kid`).
    pub fn kid(&self) -> char {
        match self {
            OrbitContent::Empty(_) => '.',
            OrbitContent::World(_) => 'W',
            OrbitContent::GasGiant(_) => 'G',
            OrbitContent::Belt(_) => 'B',
            OrbitContent::Hostile(_) => 'H',
            OrbitContent::Rockball(_) => 'R',
            OrbitContent::Companion(_) => 'S',
        }
    }

    pub fn set_orbit_number(&mut self, n: u8) {
        match self {
            OrbitContent::Empty(o) => o.orbit_number = n,
            OrbitContent::World(o) => o.orbit_number = n,
            OrbitContent::GasGiant(o) => o.orbit_number = n,
            OrbitContent::Belt(o) => o.orbit_number = n,
            OrbitContent::Hostile(o) => o.orbit_number = n,
            OrbitContent::Rockball(o) => o.orbit_number = n,
            OrbitContent::Companion(o) => o.orbit_number = n,
        }
    }

    pub fn set_au(&mut self, au: f64) {
        match self {
            OrbitContent::Empty(o) => o.au = au,
            OrbitContent::World(o) => o.au = au,
            OrbitContent::GasGiant(o) => o.au = au,
            OrbitContent::Belt(o) => o.au = au,
            OrbitContent::Hostile(o) => o.au = au,
            OrbitContent::Rockball(o) => o.au = au,
            OrbitContent::Companion(o) => o.au = au,
        }
    }

    /// The UWP column for the per-orbit detail line (Ruby Orbit#uwp overrides).
    pub fn uwp_column(&self) -> String {
        match self {
            OrbitContent::Empty(_) => ".......-.".to_string(),
            OrbitContent::World(w) => w.world.uwp.clone(),
            OrbitContent::GasGiant(g) => match g.size {
                GiantSize::Small => "Small GG ".to_string(),
                GiantSize::Large => "Large GG ".to_string(),
            },
            OrbitContent::Belt(_) => "XR00000-0".to_string(),
            OrbitContent::Hostile(h) => format!(
                "X{}{}{}000-0",
                ehex(h.size.max(0) as u8),
                ehex(h.atmosphere),
                ehex(h.hydrographics)
            ),
            OrbitContent::Rockball(r) => format!("X{}00000-0", ehex(r.size.max(0) as u8)),
            OrbitContent::Companion(c) => format!("{:<9}", c.classification),
        }
    }

    pub fn moons(&self) -> Option<&[Moon]> {
        match self {
            OrbitContent::GasGiant(g) => Some(&g.moons),
            OrbitContent::Hostile(h) => Some(&h.moons),
            OrbitContent::Rockball(r) => Some(&r.moons),
            _ => None,
        }
    }
}

impl Orbit for OrbitContent {
    fn orbit_number(&self) -> u8 {
        match self {
            OrbitContent::Empty(o) => o.orbit_number,
            OrbitContent::World(o) => o.orbit_number,
            OrbitContent::GasGiant(o) => o.orbit_number,
            OrbitContent::Belt(o) => o.orbit_number,
            OrbitContent::Hostile(o) => o.orbit_number,
            OrbitContent::Rockball(o) => o.orbit_number,
            OrbitContent::Companion(o) => o.orbit_number,
        }
    }

    fn au(&self) -> f64 {
        match self {
            OrbitContent::Empty(o) => o.au,
            OrbitContent::World(o) => o.au,
            OrbitContent::GasGiant(o) => o.au,
            OrbitContent::Belt(o) => o.au,
            OrbitContent::Hostile(o) => o.au,
            OrbitContent::Rockball(o) => o.au,
            OrbitContent::Companion(o) => o.au,
        }
    }

    fn orbit_type(&self) -> OrbitType {
        match self {
            OrbitContent::Empty(_) => OrbitType::Empty,
            OrbitContent::World(_) => OrbitType::World,
            OrbitContent::GasGiant(_) => OrbitType::GasGiant,
            OrbitContent::Belt(_) => OrbitType::Belt,
            OrbitContent::Hostile(_) => OrbitType::Hostile,
            OrbitContent::Rockball(_) => OrbitType::Rockball,
            OrbitContent::Companion(_) => OrbitType::Companion,
        }
    }

    fn to_ascii(&self) -> String {
        self.kid().to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyOrbit {
    pub orbit_number: u8,
    pub au: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldOrbit {
    pub orbit_number: u8,
    pub au: f64,
    pub world: World,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasGiant {
    pub orbit_number: u8,
    pub au: f64,
    pub size: GiantSize,
    pub moons: Vec<Moon>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GiantSize {
    Small,
    Large,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Belt {
    pub orbit_number: u8,
    pub au: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hostile {
    pub orbit_number: u8,
    pub au: f64,
    /// World size digit (2d6-2, rolled before the hostile overrides).
    #[serde(default)]
    pub size: i16,
    pub atmosphere: u8,
    pub hydrographics: u8,
    #[serde(default)]
    pub moons: Vec<Moon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rockball {
    pub orbit_number: u8,
    pub au: f64,
    #[serde(default)]
    pub size: i16,
    #[serde(default)]
    pub moons: Vec<Moon>,
}

/// A companion star holding down an orbit slot of the primary (Ruby Companion<Orbit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionOrbit {
    pub orbit_number: u8,
    pub au: f64,
    /// Compact classification of the companion star, e.g. "M2V".
    pub classification: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Moon {
    pub orbit: u8,
    /// Distance in planetary radii (Ruby moon @orbit; extreme orbits reach 350).
    pub orbital_radius: u16,
    /// Can be negative for the tiniest moonlets (rendered 'S'; 0 renders 'R').
    pub size: i16,
    pub atmosphere: u8,
    pub hydrographics: u8,
}

impl Moon {
    /// Ruby Moon#uwp: X + size + atmo + hydro + 000 (no dash/tech).
    pub fn uwp(&self) -> String {
        let size = if self.size < 0 {
            "S".to_string()
        } else if self.size == 0 {
            "R".to_string()
        } else {
            ehex(self.size as u8).to_string()
        };
        format!("X{}{}{}000", size, ehex(self.atmosphere), ehex(self.hydrographics))
    }
}

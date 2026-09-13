use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum StarType {
    O, B, A, F, G, K, M, D, // D = white dwarf (converted from size roll)
}

impl StarType {
    /// Position in the O B A F G K M cooling sequence (D falls off the end).
    pub fn seq_index(&self) -> usize {
        match self {
            StarType::O => 0,
            StarType::B => 1,
            StarType::A => 2,
            StarType::F => 3,
            StarType::G => 4,
            StarType::K => 5,
            StarType::M => 6,
            StarType::D => 6, // Ruby: seq.index('D') || seq.size - 1
        }
    }

    pub fn from_seq_index(i: usize) -> StarType {
        match i {
            0 => StarType::O,
            1 => StarType::B,
            2 => StarType::A,
            3 => StarType::F,
            4 => StarType::G,
            5 => StarType::K,
            _ => StarType::M,
        }
    }
}

impl fmt::Display for StarType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StarType::O => write!(f, "O"),
            StarType::B => write!(f, "B"),
            StarType::A => write!(f, "A"),
            StarType::F => write!(f, "F"),
            StarType::G => write!(f, "G"),
            StarType::K => write!(f, "K"),
            StarType::M => write!(f, "M"),
            StarType::D => write!(f, "D"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum StarSize {
    Ia,  // 0 Bright supergiant
    Ib,  // 1 Supergiant
    II,  // 2 Bright giant
    III, // 3 Giant
    IV,  // 4 Subgiant
    V,   // 5 Main sequence
    VI,  // 6 Subdwarf
    D,   // White dwarf (Ruby size 500)
}

impl StarSize {
    /// The Ruby integer size (0-6); D behaves as index 0 for the per-size tables
    /// (Ruby indexes with `size % 10`, and 500 % 10 == 0).
    pub fn table_index(&self) -> usize {
        match self {
            StarSize::Ia => 0,
            StarSize::Ib => 1,
            StarSize::II => 2,
            StarSize::III => 3,
            StarSize::IV => 4,
            StarSize::V => 5,
            StarSize::VI => 6,
            StarSize::D => 0,
        }
    }

    pub fn roman(&self) -> &'static str {
        match self {
            StarSize::Ia => "Ia",
            StarSize::Ib => "Ib",
            StarSize::II => "II",
            StarSize::III => "III",
            StarSize::IV => "IV",
            StarSize::V => "V",
            StarSize::VI => "VI",
            StarSize::D => "D",
        }
    }
}

impl fmt::Display for StarSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.roman())
    }
}

// ---- Ruby star.rb tables (Classic Traveller + GURPS Space 4e) ----------------

/// Minimum orbital distance (AU) by type and size — Ruby INNER_LIMIT.
fn inner_limit_row(t: StarType) -> &'static [f64] {
    match t {
        StarType::O => &[16.0, 13.0, 10.0, 8.0, 6.0, 1.0, 0.0],
        StarType::B => &[10.0, 6.3, 5.0, 4.0, 3.8, 0.6, 0.0],
        StarType::A => &[4.0, 1.0, 0.4, 0.0, 0.0, 0.0, 0.0],
        StarType::F => &[4.0, 1.0, 0.3, 0.1, 0.0, 0.0, 0.0],
        StarType::G => &[3.1, 1.0, 0.3, 0.1, 0.0, 0.0, 0.0],
        StarType::K => &[2.5, 1.0, 0.3, 0.1, 0.0, 0.0, 0.0],
        StarType::M => &[2.0, 1.0, 0.3, 0.1, 0.0, 0.0, 0.0],
        StarType::D => &[0.0],
    }
}

/// Habitable-zone band (AU) by type and size — Ruby BIOZONE. `None` where the Ruby
/// table has no entry (the star then has no biozone and every orbit reads as inner).
fn biozone_row(t: StarType) -> &'static [(f64, f64)] {
    match t {
        StarType::O => &[(790.0, 1190.0), (630.0, 950.0), (500.0, 750.0), (350.0, 525.0), (235.0, 350.0), (150.0, 225.0), (100.0, 150.0)],
        StarType::B => &[(500.0, 700.0), (320.0, 480.0), (250.0, 375.0), (200.0, 300.0), (180.0, 270.0), (30.0, 45.0)],
        StarType::A => &[(200.0, 300.0), (50.0, 75.0), (20.0, 30.0), (5.0, 7.5), (4.0, 6.0), (3.1, 4.7)],
        StarType::F => &[(200.0, 300.0), (50.0, 75.0), (13.0, 19.0), (2.5, 3.7), (2.0, 3.0), (1.6, 2.4), (0.5, 0.8)],
        StarType::G => &[(200.0, 300.0), (50.0, 75.0), (13.0, 19.0), (2.5, 3.7), (2.0, 3.0), (1.6, 2.4), (0.5, 0.8)],
        StarType::K => &[(125.0, 190.0), (50.0, 75.0), (13.0, 19.0), (4.0, 5.9), (1.0, 1.5), (0.5, 0.6), (0.2, 0.3)],
        StarType::M => &[(100.0, 150.0), (50.0, 76.0), (16.0, 24.0), (5.0, 7.5), (0.0, 0.0), (0.1, 0.2), (0.1, 0.1)],
        StarType::D => &[(0.03, 0.03)],
    }
}

/// Stellar mass (solar masses) by type and size — Ruby MASS. Out-of-range (incl.
/// white dwarfs, Ruby size 500) falls back to 0.3 exactly as `MASS[t][s] || 0.3`.
fn mass_row(t: StarType) -> &'static [f64] {
    match t {
        StarType::O => &[90.0, 60.0, 40.0, 25.0, 20.0, 16.0, 16.0],
        StarType::B => &[50.0, 40.0, 35.0, 30.0, 20.0, 10.0],
        StarType::A => &[30.0, 16.0, 10.0, 6.0, 4.0, 3.0],
        StarType::F => &[15.0, 13.0, 8.0, 2.5, 2.2, 1.9],
        StarType::G => &[12.0, 10.0, 6.0, 2.7, 1.8, 1.1, 0.8],
        StarType::K => &[15.0, 12.0, 6.0, 3.0, 2.3, 0.9, 0.5],
        StarType::M => &[20.0, 16.0, 8.0, 4.0, 0.3, 0.2],
        StarType::D => &[0.8, 0.8, 0.8, 0.8, 0.8, 0.8],
    }
}

/// Valid spectral subtypes per type — Ruby SPECTRAL (keeps every spectral code
/// resolvable in STAR_CHART).
pub fn spectral_subtypes(t: StarType) -> &'static [u8] {
    match t {
        StarType::O => &[9],
        StarType::B => &[0, 2, 5, 8],
        StarType::A => &[0, 2, 5],
        StarType::F => &[0, 2, 5],
        StarType::G => &[0, 2, 5, 8],
        StarType::K => &[0, 2, 5],
        StarType::M => &[0, 2, 4, 6],
        StarType::D => &[0],
    }
}

/// (temperature K, luminosity in Sol units) per spectral code — Ruby STAR_CHART.
fn star_chart(spectral: &str) -> Option<(u32, f64)> {
    Some(match spectral {
        "O9" => (33000, 55000.0),
        "B0" => (30000, 16000.0),
        "B2" => (22000, 8300.0),
        "B5" => (15000, 750.0),
        "B8" => (12500, 130.0),
        "A0" => (9500, 63.0),
        "A2" => (9000, 40.0),
        "A5" => (8700, 24.0),
        "F0" => (7400, 9.0),
        "F2" => (7100, 6.3),
        "F5" => (6400, 4.0),
        "G0" => (5900, 1.45),
        "G2" => (5800, 1.00),
        "G5" => (5600, 0.70),
        "G8" => (5300, 0.44),
        "K0" => (5100, 0.36),
        "K2" => (4830, 0.28),
        "K5" => (4370, 0.18),
        "M0" => (3670, 0.075),
        "M2" => (3400, 0.03),
        "M4" => (3200, 0.0005),
        "M6" => (3000, 0.0002),
        _ => return None,
    })
}

/// GURPS Space 4e p.105 companion separation multipliers — Ruby COMPANION_SEPARATION.
pub const COMPANION_SEPARATION: [f64; 20] = [
    0.05, 0.05, 0.5, 0.5, 0.5, 2.0, 2.0, 10.0, 10.0, 10.0,
    50.0, 50.0, 50.0, 50.0, 50.0, 50.0, 50.0, 50.0, 50.0, 50.0,
];

/// Bode-law orbital spacing constants — Ruby BODE_RATIO (indexed by toss = 2d6-2).
pub const BODE_RATIO: [f64; 11] = [0.3, 0.3, 0.3, 0.3, 0.35, 0.35, 0.35, 0.4, 0.4, 0.4, 0.4];

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Star {
    pub star_type: StarType,
    pub star_size: StarSize,
    pub spectral: String,
    /// White-dwarf subtype letter (Ruby @star_subtype, always 'B'); classification "DB".
    #[serde(default)]
    pub star_subtype: Option<char>,
    pub is_primary: bool,
    /// Companion placement (Ruby @orbit, a fractional orbit number; 0 for primaries).
    #[serde(default)]
    pub orbit_number: f64,
    /// The raw 2d6 size roll — feeds companion size derivation (Ruby @size_dm).
    #[serde(default)]
    pub size_dm: i64,
    pub companions: Vec<Star>,
    pub orbits: Vec<crate::models::OrbitContent>,
    pub mass: f64,
    pub luminosity: f64,
    pub temperature: u32,
    pub bode_constant: f64,
}

impl Star {
    pub fn new(star_type: StarType, star_size: StarSize, is_primary: bool) -> Self {
        let mut s = Star {
            star_type,
            star_size,
            spectral: format!("{}{}", star_type, 5),
            star_subtype: None,
            is_primary,
            orbit_number: 0.0,
            size_dm: 7,
            companions: Vec::new(),
            orbits: Vec::new(),
            mass: 1.0,
            luminosity: 1.0,
            temperature: 5800,
            bode_constant: 0.3,
        };
        s.refresh_physicals();
        s
    }

    /// Recompute mass / luminosity / temperature from the tables.
    pub fn refresh_physicals(&mut self) {
        self.mass = mass_row(self.star_type)
            .get(if self.star_size == StarSize::D { usize::MAX } else { self.star_size.table_index() })
            .copied()
            .unwrap_or(0.3);
        if let Some((temp, lux)) = star_chart(&self.spectral) {
            self.temperature = temp;
            self.luminosity = lux;
        }
    }

    /// Ruby orbit_to_au: inner limit plus the Bode progression (product rounded to 0.1).
    pub fn orbit_to_au(&self, orbit: f64) -> f64 {
        self.inner_limit() + round1(self.bode_constant * 2.0_f64.powf(orbit))
    }

    /// Ruby au_to_orbit: |log2(au / bode)| (rounded to 0.01) minus the inner limit.
    /// Uses the primary's bode constant when called for a companion placement.
    pub fn au_to_orbit(&self, au: f64) -> f64 {
        if au <= 0.0 || self.bode_constant <= 0.0 {
            return 0.0;
        }
        round2((au / self.bode_constant).ln() / 2.0_f64.ln()).abs() - self.inner_limit()
    }

    /// Minimum orbital distance (AU) — Ruby `limit` (0 when the table has no entry).
    pub fn inner_limit(&self) -> f64 {
        inner_limit_row(self.star_type)
            .get(self.star_size.table_index())
            .copied()
            .unwrap_or(0.0)
    }

    pub fn outer_limit(&self) -> f64 {
        40.0 * self.mass // GURPS Space 4e p.107
    }

    /// Habitable zone band, if this type/size has one (Ruby BIOZONE lookup; a star
    /// without an entry has no biozone and all its orbits count as inner).
    pub fn biozone(&self) -> Option<(f64, f64)> {
        biozone_row(self.star_type).get(self.star_size.table_index()).copied()
    }

    /// "F2V" / "DB" — Ruby classification (compact, for the orbit crib).
    pub fn classification(&self) -> String {
        if self.star_type == StarType::D {
            return format!("D{}", self.star_subtype.unwrap_or('B'));
        }
        format!("{}{}", self.spectral, self.star_size.roman())
    }

    /// "F2 V" / "DB" — T5 Second Survey stellar notation.
    pub fn t5_classification(&self) -> String {
        if self.star_type == StarType::D {
            return format!("D{}", self.star_subtype.unwrap_or('B'));
        }
        format!("{} {}", self.spectral, self.star_size.roman())
    }

    /// Primary + companions, space-separated (T5SS Stars column).
    pub fn t5_stars(&self) -> String {
        let mut v = vec![self.t5_classification()];
        for c in &self.companions {
            v.push(c.t5_classification());
        }
        v.join(" ")
    }
}

impl fmt::Display for Star {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.t5_classification())
    }
}

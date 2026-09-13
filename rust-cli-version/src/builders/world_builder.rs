//! World generation, converged onto Traveller 5 via the active ruleset (mirrors the
//! Ruby orbit.rb World/Terrestrial and the Go pkg/builder/world.go). The UWP spine,
//! trade codes, and starport/tech/base tables come from rules/<name>.yml; the
//! algorithmic modules (climate, native, Ix/Ex/Cx) and the genre realism passes are
//! code here.

use crate::error::Result;
use crate::models::world::{Base, Temperature, World};
use crate::models::StarType;
use crate::rng;
use crate::rules::runtime;
use crate::rules::{Context, Value};

pub struct WorldBuilder {
    row: usize,
    col: usize,
    names: Vec<String>,
    orbit_number: u8,
    star_type: Option<StarType>,
}

fn flux() -> i64 {
    (rng::d6().unwrap_or(3) as i64) - (rng::d6().unwrap_or(3) as i64)
}

impl WorldBuilder {
    pub fn new(row: usize, col: usize) -> Self {
        WorldBuilder {
            row,
            col,
            names: Vec::new(),
            orbit_number: 0,
            star_type: None,
        }
    }

    pub fn with_names(mut self, names: Vec<String>) -> Self {
        self.names = names;
        self
    }

    pub fn with_orbit(mut self, orbit_number: u8) -> Self {
        self.orbit_number = orbit_number;
        self
    }

    pub fn with_star_type(mut self, star_type: StarType) -> Self {
        self.star_type = Some(star_type);
        self
    }

    pub fn build(self) -> Result<World> {
        let rs = runtime::ruleset();
        let genre = runtime::genre();
        let mut world = World::new(self.row, self.col);
        let mut ctx: Context = Context::new();

        // UWP spine — Size / Atmo / Hydro from the ruleset's step formulas.
        let size = rs.uwp_step("size", &ctx);
        ctx.insert("size".into(), Value::Int(size));
        let atmo = rs.uwp_step("atmo", &ctx);
        ctx.insert("atmo".into(), Value::Int(atmo));
        let hydro = rs.uwp_step("hydro", &ctx);
        ctx.insert("hydro".into(), Value::Int(hydro));
        world.size = size as u8;
        world.atmosphere = atmo as u8;
        world.hydrographics = hydro as u8;

        // Climate, then the genre realism pass (may thin atmosphere / dry hydro).
        world.temperature = climate(self.orbit_number);
        ctx.insert("temp".into(), Value::Str(world.temperature.to_code().to_string()));
        apply_genre_atmo_hydro(&mut world, &genre);
        ctx.insert("atmo".into(), Value::Int(world.atmosphere as i64));
        ctx.insert("hydro".into(), Value::Int(world.hydrographics as i64));

        // Population — the port-orientation roll is taken now (firm nudges it by pop).
        let mut port_roll = rng::roll_2d6()? as i64;
        let mut pop = rs.uwp_step("pop", &ctx);
        let stripped = firm_pop_strip(&world, &genre, pop, port_roll);
        pop = stripped.0;
        port_roll = stripped.1;
        pop = pop.clamp(0, 15);
        pop = cap_colony_population(&world, pop, self.star_type);
        world.population = pop as u8;
        ctx.insert("pop".into(), Value::Int(pop));

        world.government = rs.uwp_step("gov", &ctx) as u8;
        ctx.insert("gov".into(), Value::Int(world.government as i64));
        world.law_level = rs.uwp_step("law", &ctx) as u8;
        ctx.insert("law".into(), Value::Int(world.law_level as i64));

        world.starport = rs.starport(port_roll).chars().next().unwrap_or('X');
        ctx.insert("port".into(), Value::Str(world.starport.to_string()));

        world.factions = Self::generate_factions(world.population, world.law_level)?;

        let mut tech = (rng::d6()? as i64 + rs.tech_dm(&ctx)).max(0);
        if let Some(cap) = runtime::tech_cap() {
            tech = tech.min(cap);
        }
        world.tech_level = tech.min(15) as u8;
        if world.population == 0 {
            world.law_level = 0;
            world.government = 0;
            world.tech_level = 0;
        }
        ctx.insert("tech".into(), Value::Int(world.tech_level as i64));
        ctx.insert("gov".into(), Value::Int(world.government as i64));
        ctx.insert("law".into(), Value::Int(world.law_level as i64));

        world.trade_codes = rs.trade_codes(&ctx);
        world.bases = generate_bases(world.starport)?;

        if world.population > 0 {
            world.pop_multiplier = 1 + rng::roll_range(9)? as u8;
        }

        world.update_uwp();

        if !self.names.is_empty() {
            let idx = rng::roll_range(self.names.len())?;
            world.name = self.names[idx].clone();
        } else {
            world.name = format!("World-{}", world.coords());
        }

        Ok(world)
    }

    fn generate_factions(population: u8, law_level: u8) -> Result<Vec<String>> {
        if population == 0 {
            return Ok(vec![]);
        }
        let mut num = (rng::roll_range(3)? + 1).min(3);
        if law_level == 0 || law_level == 7 {
            num += 1;
        }
        if law_level > 9 {
            num = num.saturating_sub(1);
        }
        let types = ["O", "O", "O", "O", "F", "F", "M", "M", "N", "N", "S", "S", "P"];
        let rolls: Vec<usize> = (0..5).map(|_| rng::roll_2d6().unwrap_or(7) as usize).collect();
        let mut factions = Vec::new();
        for i in 0..num.min(rolls.len()) {
            factions.push(types[rolls[i].min(types.len() - 1)].to_string());
        }
        Ok(factions)
    }
}

// ---- climate / native modules ------------------------------------------

fn climate(orbit_number: u8) -> Temperature {
    let module = runtime::ruleset().module_for("climate").unwrap_or_else(|_| "t5".into());
    if module == "none" {
        return Temperature::Temperate;
    }
    if orbit_number <= 1 {
        return Temperature::Twilight;
    }
    let variance = [-2, -1, -1, -1, 0, 0, 0, 0, 0, 1, 1, 1, 2][(flux() + 6).clamp(0, 12) as usize];
    if variance <= -1 {
        Temperature::Hot
    } else if variance >= 1 {
        Temperature::Cold
    } else {
        Temperature::Temperate
    }
}

fn native_status(w: &World) -> String {
    let module = runtime::ruleset().module_for("native").unwrap_or_else(|_| "t5".into());
    if module == "none" {
        return String::new();
    }
    let varied = runtime::sophonts() == "varied";
    if varied {
        if w.population >= 7 {
            return if w.atmosphere <= 1 { "Exotic".into() } else { "Native".into() };
        }
        if (1..=6).contains(&w.population) {
            return "Colony".into();
        }
        return String::new();
    }
    if w.population >= 7 {
        "Settled".into()
    } else if (1..=6).contains(&w.population) {
        "Colony".into()
    } else {
        String::new()
    }
}

// ---- genre realism passes ----------------------------------------------

fn apply_genre_atmo_hydro(w: &mut World, genre: &str) {
    if genre != "opera" && genre != "firm" {
        return;
    }
    let size = w.size as i64;
    let atmo = w.atmosphere as i64;
    let new_atmo = if size < 3 || (size < 4 && atmo < 3) {
        0
    } else if (size == 3 || size == 4) && (3..=5).contains(&atmo) {
        1
    } else if (size == 3 || size == 4) && atmo > 5 {
        10
    } else {
        atmo
    };
    w.atmosphere = new_atmo as u8;
    let mut hydro = w.hydrographics as i64;
    let a = w.atmosphere as i64;
    if a < 2 {
        hydro -= 6;
    }
    if a == 2 || a == 3 || a == 11 || a == 12 {
        hydro -= 4;
    }
    w.hydrographics = hydro.max(0) as u8;
}

fn firm_pop_strip(w: &World, genre: &str, mut pop: i64, mut port_roll: i64) -> (i64, i64) {
    if genre != "firm" {
        return (pop, port_roll);
    }
    let size = w.size as i64;
    if size < 3 || size > 9 {
        pop -= 1;
    }
    let atmo_dm = [-1, -1, -1, -1, -1, 1, 1, -1, 1, -1, -1, -1, -1, -1, -1, -1];
    let a = w.atmosphere as usize;
    if a < atmo_dm.len() {
        pop += atmo_dm[a];
    }
    port_roll = (port_roll + 7 - pop.max(0)).max(0);
    (pop, port_roll)
}

const GRAVITY: [f64; 16] = [
    0.0, 0.05, 0.15, 0.25, 0.35, 0.45, 0.7, 0.9, 1.0, 1.25, 1.4, 1.6, 1.9, 2.2, 2.5, 2.8,
];

fn gravity_for(size: u8) -> f64 {
    *GRAVITY.get(size as usize).unwrap_or(&0.0)
}

fn is_hot_star(t: StarType) -> bool {
    matches!(t, StarType::O | StarType::B | StarType::A | StarType::F)
}

fn cap_colony_population(w: &World, mut pop: i64, star_type: Option<StarType>) -> i64 {
    if let Some(t) = star_type {
        if is_hot_star(t) && pop > 6 {
            pop = 6;
        }
    }
    let g = gravity_for(w.size);
    if !(0.4..=1.5).contains(&g) && pop > 6 {
        pop = 6;
    }
    pop
}

// ---- starport bases (ruleset-driven) -----------------------------------

fn generate_bases(port: char) -> Result<Vec<Base>> {
    let rs = runtime::ruleset();
    let p = port.to_string();
    let mut bases = Vec::new();
    for (kind, base) in [
        ("naval", Base::Naval),
        ("scout", Base::Scout),
        ("depot", Base::Depot),
        ("way", Base::Way),
    ] {
        if let Some(th) = rs.base_threshold(kind, &p) {
            if rs.base_meets(rng::roll_2d6()? as i64, th) {
                bases.push(base);
            }
        }
    }
    Ok(bases)
}

// ---- extensions module (post-pass) -------------------------------------

/// Run the ruleset's extensions module (if any), then native status. Called once the
/// system's gas-giant and belt counts are known. Mirrors the Go buildExtensions.
pub fn build_extensions(w: &mut World, gas_giants: i64, belts: i64) {
    let module = runtime::ruleset().module_for("extensions").unwrap_or_else(|_| "t5".into());
    if module != "none" {
        build_extensions_t5(w, gas_giants, belts);
    }
    w.native = native_status(w);
}

fn build_extensions_t5(w: &mut World, gas_giants: i64, belts: i64) {
    let tc: Vec<String> = w.trade_codes.clone();
    let bases = w.bases.clone();
    let pop = w.population as i64;
    let tech = w.tech_level as i64;
    let port = w.starport;
    let has = |c: &str| tc.iter().any(|x| x == c);

    let mut ix = 0i64;
    if port == 'A' || port == 'B' {
        ix += 1;
    }
    if port == 'D' || port == 'E' || port == 'X' {
        ix -= 1;
    }
    if tech >= 10 {
        ix += 1;
    }
    if tech <= 8 {
        ix -= 1;
    }
    ix += ["Ag", "Hi", "In", "Ri"].iter().filter(|c| has(c)).count() as i64;
    if pop <= 6 {
        ix -= 1;
    }
    if bases.contains(&Base::Naval) && bases.contains(&Base::Scout) {
        ix += 1;
    }
    if bases.contains(&Base::Way) {
        ix += 1;
    }

    let mut res = rng::roll_2d6().unwrap_or(7) as i64;
    if tech >= 8 {
        res += gas_giants + belts;
    }
    let lab = (pop - 1).max(0);
    let inf = if ["Ba", "Di", "Lo"].iter().any(|c| has(c)) {
        0
    } else if has("Ni") {
        rng::d6().unwrap_or(3) as i64
    } else {
        (rng::roll_2d6().unwrap_or(7) as i64 + ix).max(0)
    };
    let eff = flux();
    let nz = |v: i64| if v == 0 { 1 } else { v };
    let ru = nz(res) * nz(lab) * nz(inf) * nz(eff);

    let min1 = |v: i64| if v < 1 { 1 } else { v };
    let homo = min1(pop + flux());
    let acc = min1(pop + ix);
    let strange = min1(5 + flux());
    let sym = min1(tech + flux());

    w.ix = ix;
    w.ex = [res, lab, inf, eff];
    w.ru = ru;
    w.cx = [homo, acc, strange, sym];
    w.extended = true;
}

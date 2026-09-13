use crate::models::{Volume, OrbitContent};
use crate::error::Result;
use crate::builders::StarBuilder;
use crate::data::get_planet_names;
use crate::rng;

pub struct VolumeBuilder {
    row: usize,
    col: usize,
}

impl VolumeBuilder {
    pub fn new(row: usize, col: usize) -> Self {
        VolumeBuilder { row, col }
    }

    pub fn build(self) -> Result<Volume> {
        let mut volume = Volume::new(self.row, self.col);

        // A system exists wherever the sector's density roll placed this volume — the
        // density is the single gate (matching Ruby/Go); whether it has a mainworld
        // depends on its biozone.
        let mut star = StarBuilder::build_primary()?;

        // The mainworld is the LAST world in the orbit list (Ruby @world), captured
        // before companion insertion — a companion's forbidden zone may clear its
        // orbit slot, but the system keeps its mainworld (as in Ruby).
        for orbit in &star.orbits {
            if let OrbitContent::World(world_orbit) = orbit {
                volume.world = Some(world_orbit.world.clone());
            }
        }

        // Companion stars (Ruby Volume#initialize): 2d6 — 8-11 one, 12 two.
        const COMPANIONS: [usize; 13] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2];
        let n = COMPANIONS[(rng::roll_2d6()? as usize).min(12)];
        for ternary in 0..n {
            let companion = StarBuilder::build_companion(&star, ternary as i64)?;
            StarBuilder::insert_companion(&mut star, companion);
        }

        volume.star = Some(star);

        // A starful but worldless system still gets a name.
        if volume.world.is_none() && volume.star.is_some() {
            let names = get_planet_names();
            if !names.is_empty() {
                let name_idx = (rng::roll_1d6() as usize * 100 + rng::roll_d100() as usize) % names.len();
                volume.name = names[name_idx].clone();
            }
        }

        Ok(volume)
    }
}

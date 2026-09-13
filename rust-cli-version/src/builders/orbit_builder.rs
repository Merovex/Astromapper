//! Orbit population — a faithful port of the Ruby builder/orbit.rb zone tables and
//! builder/star.rb orbit loop (Classic Traveller + GURPS Space 4e mechanics).
//! Dice notation: `toss(a,b)` in Ruby is (a d6) - b floored at 0.

use crate::models::{OrbitContent, Star, StarSize, StarType};
use crate::models::orbit::{
    EmptyOrbit, WorldOrbit, GasGiant, Belt, Hostile, Rockball, GiantSize, Moon,
};
use crate::rng;
use crate::error::Result;
use crate::builders::WorldBuilder;
use crate::data::get_planet_names;
use crate::rules::runtime;

/// Zone classification of an orbit relative to the star's biozone.
#[derive(Clone, Copy, PartialEq)]
enum Zone {
    Inner,
    Biozone,
    Outer,
}

pub struct OrbitBuilder;

impl OrbitBuilder {
    pub fn populate_orbits(star: &mut Star) -> Result<()> {
        let dm = Self::orbit_dm(star);
        let num_orbits = ((rng::roll_2d6()? as i64) + dm).max(0);

        let names = get_planet_names();
        let mut orbits: Vec<Option<OrbitContent>> = Vec::new();

        for i in 0..num_orbits {
            let au = star.orbit_to_au(i as f64);
            // Ruby skips appending beyond the outer limit; AU grows monotonically,
            // so this is a truncation.
            if au > star.outer_limit() {
                break;
            }
            orbits.push(Some(Self::populate_orbit(star, i as u8, au, &names)?));
        }

        star.orbits = Self::prune(star, orbits);
        Ok(())
    }

    /// Trailing-empty cleanup + renumbering (Ruby Star#prune!): nil slots become
    /// empty orbits, empties are stripped off the far end, and the survivors are
    /// renumbered sequentially with their AU recomputed.
    pub fn prune(star: &Star, orbits: Vec<Option<OrbitContent>>) -> Vec<OrbitContent> {
        let mut filled: Vec<OrbitContent> = orbits
            .into_iter()
            .enumerate()
            .map(|(i, o)| {
                o.unwrap_or(OrbitContent::Empty(EmptyOrbit {
                    orbit_number: i as u8,
                    au: star.orbit_to_au(i as f64),
                }))
            })
            .collect();

        let last_occupied = filled.iter().rposition(|o| o.kid() != '.');
        match last_occupied {
            Some(idx) => filled.truncate(idx + 1),
            None => filled.clear(),
        }

        if filled.len() >= 2 {
            for (i, o) in filled.iter_mut().enumerate() {
                o.set_orbit_number(i as u8);
                o.set_au(star.orbit_to_au(i as f64));
            }
        }
        filled
    }

    /// Orbit-count DM (Ruby Star#initialize): +4 giants (III), +8 anything brighter
    /// (Ia/Ib/II), -4 M-types, -2 K-types. White dwarfs (Ruby size 500) get neither.
    fn orbit_dm(star: &Star) -> i64 {
        let mut dm = 0;
        match star.star_size {
            StarSize::III => dm += 4,
            StarSize::Ia | StarSize::Ib | StarSize::II => dm += 8,
            _ => {}
        }
        match star.star_type {
            StarType::M => dm -= 4,
            StarType::K => dm -= 2,
            _ => {}
        }
        dm
    }

    /// Ruby Orbit#initialize zone logic: no biozone entry means everything reads
    /// as inner (and `distant` never matters there).
    fn zone_of(star: &Star, au: f64) -> (Zone, bool) {
        match star.biozone() {
            None => (Zone::Inner, true),
            Some((b0, b1)) => {
                let z = if au < b0 {
                    Zone::Inner
                } else if au > b1 {
                    Zone::Outer
                } else {
                    Zone::Biozone
                };
                (z, au > b1 * 10.0)
            }
        }
    }

    fn populate_orbit(star: &Star, orbit_num: u8, au: f64, names: &[String]) -> Result<OrbitContent> {
        // Ruby Orbit#populate: beyond the outer limit or inside the inner limit
        // stays an empty orbit.
        if au > star.outer_limit() || au < star.inner_limit() {
            return Ok(OrbitContent::Empty(EmptyOrbit { orbit_number: orbit_num, au }));
        }
        let (zone, distant) = Self::zone_of(star, au);
        match zone {
            Zone::Inner => Self::populate_inner(star, orbit_num, au),
            Zone::Outer => Self::populate_outer(star, orbit_num, au, distant),
            Zone::Biozone => Self::populate_biozone(star, orbit_num, au, names),
        }
    }

    /// Biozone (Ruby populate_biozone): with `always_inhabited` (default) always a
    /// mainworld; otherwise 2d6 < 12 world, 12 gas giant.
    fn populate_biozone(star: &Star, orbit_num: u8, au: f64, names: &[String]) -> Result<OrbitContent> {
        let world_here = runtime::always_inhabited() || rng::roll_2d6()? < 12;
        if world_here {
            let world = WorldBuilder::new(0, 0)
                .with_names(names.to_vec())
                .with_orbit(orbit_num)
                .with_star_type(star.star_type)
                .build()?;
            let moons = Self::make_moons(
                Self::toss(1, 3) as usize,
                world.size as i16,
                None,
                Zone::Biozone,
            );
            Ok(OrbitContent::World(WorldOrbit { orbit_number: orbit_num, au, world, moons }))
        } else {
            Ok(Self::make_gas_giant(orbit_num, au, Zone::Biozone))
        }
    }

    /// Inner zone (Ruby populate_inner): 2d6 — <5 empty, 5-6 hostile, 7-9 rockball,
    /// 10-11 belt, 12 gas giant.
    fn populate_inner(_star: &Star, orbit_num: u8, au: f64) -> Result<OrbitContent> {
        let roll = rng::roll_2d6()?;
        Ok(match roll {
            r if r < 5 => OrbitContent::Empty(EmptyOrbit { orbit_number: orbit_num, au }),
            5..=6 => Self::make_hostile(orbit_num, au, Zone::Inner),
            7..=9 => Self::make_rockball(orbit_num, au, Zone::Inner),
            10..=11 => OrbitContent::Belt(Belt { orbit_number: orbit_num, au }),
            _ => Self::make_gas_giant(orbit_num, au, Zone::Inner),
        })
    }

    /// Outer zone (Ruby populate_outer): 1d6 (+1 if beyond 10x the biozone) —
    /// 1 rockball, 2 belt, 3 empty, 4-7 gas giant.
    fn populate_outer(_star: &Star, orbit_num: u8, au: f64, distant: bool) -> Result<OrbitContent> {
        let mut roll = rng::roll_1d6();
        if distant {
            roll += 1;
        }
        Ok(match roll {
            1 => Self::make_rockball(orbit_num, au, Zone::Outer),
            2 => OrbitContent::Belt(Belt { orbit_number: orbit_num, au }),
            3 => OrbitContent::Empty(EmptyOrbit { orbit_number: orbit_num, au }),
            4..=7 => Self::make_gas_giant(orbit_num, au, Zone::Outer),
            _ => Self::make_rockball(orbit_num, au, Zone::Outer),
        })
    }

    // ---- planet constructors (Ruby Planet subclasses) ----------------------

    fn toss(dice: u32, minus: i64) -> i64 {
        ((rng::roll(dice, 6).unwrap_or(7) as i64) - minus).max(0)
    }

    fn make_rockball(orbit_num: u8, au: f64, zone: Zone) -> OrbitContent {
        let size = Self::toss(2, 2) as i16; // Planet: size = toss
        let moons = Self::make_moons(Self::toss(1, 3) as usize, size, None, zone);
        OrbitContent::Rockball(Rockball { orbit_number: orbit_num, au, size, moons })
    }

    fn make_hostile(orbit_num: u8, au: f64, zone: Zone) -> OrbitContent {
        let size = Self::toss(2, 2) as i16;
        let moons = Self::make_moons(Self::toss(1, 3) as usize, size, None, zone);
        // Exotic/corrosive/insidious atmosphere; (acid) seas capped at A.
        let atmosphere = (10 + rng::roll_range(5).unwrap_or(0)) as u8;
        let hydrographics = Self::toss(2, 4).min(10) as u8;
        OrbitContent::Hostile(Hostile {
            orbit_number: orbit_num,
            au,
            size,
            atmosphere,
            hydrographics,
            moons,
        })
    }

    fn make_gas_giant(orbit_num: u8, au: f64, zone: Zone) -> OrbitContent {
        let size = Self::toss(2, 2) as i16; // preliminary Planet size (feeds nothing here)
        let giant = if rng::roll_1d6() < 4 { GiantSize::Large } else { GiantSize::Small };
        let mut count = Self::toss(2, 0);
        if giant == GiantSize::Small {
            count = (count - 4).max(0);
        }
        let moons = Self::make_moons(count as usize, size, Some(giant), zone);
        OrbitContent::GasGiant(GasGiant { orbit_number: orbit_num, au, size: giant, moons })
    }

    /// Ruby Planet#make_moons: moons key off their orbital radius (duplicates
    /// collapse), sorted by radius.
    fn make_moons(count: usize, planet_size: i16, giant: Option<GiantSize>, zone: Zone) -> Vec<Moon> {
        let mut moons: Vec<Moon> = Vec::new();
        for i in 0..count {
            let m = Self::make_moon(i as i64, planet_size, giant, zone);
            // hash-key semantics: a later moon at the same radius replaces the earlier
            if let Some(existing) = moons.iter_mut().find(|x| x.orbital_radius == m.orbital_radius) {
                *existing = m;
            } else {
                moons.push(m);
            }
        }
        moons.sort_by_key(|m| m.orbital_radius);
        for (i, m) in moons.iter_mut().enumerate() {
            m.orbit = i as u8;
        }
        moons
    }

    /// Ruby Moon#initialize. Radius tables: Close = 1..14, Ring = [1,1,1,2,2,3],
    /// Extreme = Close * 25 (Far = Close * 5 exists in Ruby but is unreachable).
    fn make_moon(index: i64, planet_size: i16, giant: Option<GiantSize>, zone: Zone) -> Moon {
        const CLOSE: [u16; 14] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
        const RING: [u16; 6] = [1, 1, 1, 2, 2, 3];

        let size: i16 = match giant {
            Some(GiantSize::Large) => Self::toss(2, 4) as i16,
            Some(GiantSize::Small) => Self::toss(2, 6) as i16,
            None => planet_size - rng::roll_1d6() as i16, // may go negative (moonlet)
        };

        let orbit_roll = Self::toss(2, index);
        let orbital_radius = if size < 1 {
            RING[(Self::toss(1, 1) as usize).min(RING.len() - 1)]
        } else if orbit_roll == 12 && giant == Some(GiantSize::Large) {
            CLOSE[(Self::toss(2, 0) as usize).min(CLOSE.len() - 1)] * 25
        } else {
            CLOSE[(Self::toss(2, 0) as usize).min(CLOSE.len() - 1)]
        };

        let hydrographics: u8 = match zone {
            Zone::Inner => 0,
            _ if size == 0 => 0,
            Zone::Outer => Self::toss(2, 4) as u8,
            Zone::Biozone => Self::toss(2, 7) as u8,
        };

        let atmo_raw = Self::toss(2, 7) as i16 + size;
        let atmosphere: u8 = if size == 0 {
            0
        } else {
            match zone {
                Zone::Inner | Zone::Outer => (atmo_raw - 4).max(0) as u8,
                Zone::Biozone => 0,
            }
        };

        Moon { orbit: 0, orbital_radius, size, atmosphere, hydrographics }
    }
}

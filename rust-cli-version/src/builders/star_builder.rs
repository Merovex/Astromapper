//! Star generation — port of the Ruby builder/star.rb: genre-driven spectral model,
//! table-driven size/mass/biozone, Bode spacing, companion stars with GURPS Space 4e
//! separation and forbidden zones.

use crate::models::{Star, StarType, StarSize, OrbitContent};
use crate::models::orbit::CompanionOrbit;
use crate::models::star::{spectral_subtypes, BODE_RATIO, COMPANION_SEPARATION};
use crate::error::Result;
use crate::rng;
use crate::rules::runtime;
use crate::builders::OrbitBuilder;
use crate::builders::world_builder;

fn flux() -> i64 {
    (rng::roll_1d6() as i64) - (rng::roll_1d6() as i64)
}

/// Ruby toss(a,b): a d6 minus b, floored at 0.
fn toss(dice: u32, minus: i64) -> i64 {
    ((rng::roll(dice, 6).unwrap_or(7) as i64) - minus).max(0)
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

pub struct StarBuilder;

impl StarBuilder {
    pub fn build_primary() -> Result<Star> {
        let star_type = Self::determine_star_type();

        // Size: 2d6 -> 0 1 2 3 4 5 5 5 5 5 5 6 500 (Ruby); the raw roll is kept as
        // size_dm for companion derivation.
        let size_dm = rng::roll_2d6()? as i64;
        let star_size = Self::size_from_roll(size_dm);

        let mut star = Self::finish_star(star_type, star_size, size_dm, true)?;

        OrbitBuilder::populate_orbits(&mut star)?;
        Self::extend_mainworld(&mut star);
        Ok(star)
    }

    /// Companion star (Ruby Star#initialize with a primary): same class or cooler by
    /// 1D-1 steps, size derived from 2d6 + the primary's size roll, placed by the
    /// GURPS separation table. Companions do not get orbit systems of their own.
    pub fn build_companion(primary: &Star, ternary: i64) -> Result<Star> {
        // Separation: 2d6 x multiplier; index is toss(3) [3d6-2] + 4*ternary - 2.
        // Ruby's index -1 wraps to the last entry (50 AU) — preserved here.
        let mut idx = toss(3, 2) + 4 * ternary - 2;
        if idx < 0 {
            idx += COMPANION_SEPARATION.len() as i64;
        }
        let idx = (idx as usize).min(COMPANION_SEPARATION.len() - 1);
        let separation = round2(rng::roll_2d6()? as f64 * COMPANION_SEPARATION[idx]);

        // Fractional orbit slot around the primary (Ruby au_to_orbit(sep) - 1; the
        // companion has no size yet, so its inner-limit term is 0).
        let orbit_f = if separation > 0.0 && primary.bode_constant > 0.0 {
            round2((separation / primary.bode_constant).ln() / 2.0_f64.ln()).abs() - 1.0
        } else {
            0.0
        };

        // Spectral class: primary's class or cooler by 1D-1 steps down O B A F G K M.
        let seq_idx = (primary.star_type.seq_index() + (rng::roll_1d6() as usize - 1)).min(6);
        let star_type = StarType::from_seq_index(seq_idx);

        // Size: 2d6 + primary size roll, capped at 12, through Ruby's companion
        // size array: 0 1 2 3 4 500 500 5 5 6 500 500 500.
        let sroll = (rng::roll_2d6()? as i64 + primary.size_dm).min(12);
        let star_size = match sroll {
            0 => StarSize::Ia,
            1 => StarSize::Ib,
            2 => StarSize::II,
            3 => StarSize::III,
            4 => StarSize::IV,
            5 | 6 => StarSize::D,
            7 | 8 => StarSize::V,
            9 => StarSize::VI,
            _ => StarSize::D,
        };

        let mut star = Self::finish_star(star_type, star_size, 0, false)?;
        star.orbit_number = orbit_f;
        Ok(star)
    }

    /// Insert a companion into the primary's orbit array, clearing the GURPS
    /// forbidden zone (0.67x to 3x its distance), then re-prune (Ruby companions=).
    pub fn insert_companion(primary: &mut Star, companion: Star) {
        let orbit_f = companion.orbit_number.abs();
        let comp_au = primary.orbit_to_au(orbit_f);

        let mut orbits: Vec<Option<OrbitContent>> =
            primary.orbits.drain(..).map(Some).collect();

        // Forbidden orbits — GURPS Space 4e p.107.
        let inner = primary.au_to_orbit(comp_au * 0.67).floor() as i64;
        let outer = primary.au_to_orbit(comp_au * 3.0).ceil() as i64;
        for x in inner.max(0)..=outer {
            if let Some(slot) = orbits.get_mut(x as usize) {
                *slot = None;
            }
        }

        let insert_idx = ((orbit_f - 1.0).trunc() as i64).max(0) as usize;
        if orbits.len() <= insert_idx {
            orbits.resize_with(insert_idx + 1, || None);
        }
        orbits[insert_idx] = Some(OrbitContent::Companion(CompanionOrbit {
            orbit_number: orbit_f.round().max(0.0) as u8,
            au: comp_au,
            classification: companion.classification(),
        }));

        primary.orbits = OrbitBuilder::prune(primary, orbits);
        primary.companions.push(companion);
    }

    /// Shared tail of star construction: spectral subtype, white-dwarf conversion,
    /// Bode constant, physical characteristics.
    fn finish_star(mut star_type: StarType, star_size: StarSize, size_dm: i64, is_primary: bool) -> Result<Star> {
        let subs = spectral_subtypes(star_type);
        let sub = subs[rng::roll_range(subs.len())?];
        let spectral = format!("{}{}", star_type, sub);

        let mut star_subtype = None;
        if star_size == StarSize::D {
            star_subtype = Some('B');
            star_type = StarType::D;
        }

        // Bode spacing constant; M-dwarf main-sequence stars use the tight 0.2.
        let bode = if star_type == StarType::M && star_size == StarSize::V {
            0.2
        } else {
            BODE_RATIO[(toss(2, 2) as usize).min(BODE_RATIO.len() - 1)]
        };

        let mut star = Star::new(star_type, star_size, is_primary);
        star.spectral = spectral;
        star.star_subtype = star_subtype;
        star.size_dm = size_dm;
        star.bode_constant = bode;
        star.refresh_physicals();
        Ok(star)
    }

    /// Attach the T5 extension block (Ix/Ex/Cx, native) to the mainworld — the LAST
    /// world in the orbit list, as in Ruby (`@world` tracks the most recent one).
    fn extend_mainworld(star: &mut Star) {
        let mut gas_giants = 0i64;
        let mut belts = 0i64;
        for o in &star.orbits {
            match o.kid() {
                'G' => gas_giants += 1,
                'B' => belts += 1,
                _ => {}
            }
        }
        if let Some(OrbitContent::World(wo)) = star
            .orbits
            .iter_mut()
            .filter(|o| matches!(o, OrbitContent::World(_)))
            .next_back()
        {
            wo.world.gas_giant = gas_giants > 0;
            world_builder::build_extensions(&mut wo.world, gas_giants, belts);
        }
    }

    /// Genre = realism<->romance stellar slider. opera (and half of normal) uses the
    /// Sun-like T5 table; firm (and the other half) the realistic M-dwarf-heavy
    /// census with rare hot stars. Mirrors the Ruby/Go implementations.
    fn determine_star_type() -> StarType {
        let genre = runtime::genre();
        if genre == "opera" || (genre == "normal" && rng::roll_1d6() <= 3) {
            let f = flux();
            let mut t = if f <= -4 {
                StarType::A
            } else if f <= -2 {
                StarType::F
            } else if f <= 0 {
                StarType::G
            } else if f <= 2 {
                StarType::K
            } else {
                StarType::M
            };
            if matches!(t, StarType::M) && rng::roll_1d6() <= 3 {
                t = StarType::K;
            }
            t
        } else {
            let natural = rng::roll_2d6().unwrap_or(7) as usize;
            if natural >= 12 {
                let arr = [
                    StarType::A, StarType::A, StarType::A, StarType::A, StarType::A, StarType::A,
                    StarType::A, StarType::A, StarType::A, StarType::A, StarType::B, StarType::B,
                    StarType::O,
                ];
                arr[(rng::roll_2d6().unwrap_or(7) as usize).min(12)]
            } else {
                let arr = [
                    StarType::M, StarType::M, StarType::F, StarType::M, StarType::M, StarType::M,
                    StarType::M, StarType::M, StarType::K, StarType::K, StarType::G, StarType::M,
                ];
                arr[natural.min(11)]
            }
        }
    }

    /// Ruby: %w{0 1 2 3 4 5 5 5 5 5 5 6 500}[2d6].
    fn size_from_roll(roll: i64) -> StarSize {
        match roll {
            0 => StarSize::Ia,
            1 => StarSize::Ib,
            2 => StarSize::II,
            3 => StarSize::III,
            4 => StarSize::IV,
            5..=10 => StarSize::V,
            11 => StarSize::VI,
            _ => StarSize::D,
        }
    }
}

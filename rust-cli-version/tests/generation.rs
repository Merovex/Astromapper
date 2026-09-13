//! Integration tests for ruleset-driven generation: the genre stellar model and
//! cepheus divergence, mirroring the Ruby/Go suites.

use astromapper_core::builders::{StarBuilder, VolumeBuilder};
use astromapper_core::models::{OrbitContent, StarType};
use astromapper_core::rng;
use astromapper_core::rules::{runtime, Ruleset};

fn census(genre: &str) -> (f64, f64) {
    runtime::set_ruleset(Ruleset::load("t5", "").unwrap());
    runtime::set_genre(genre);
    runtime::set_sophonts("human");
    rng::init_rng(&format!("genre-{genre}"));
    let total = 200;
    let (mut fgk, mut m) = (0, 0);
    for _ in 0..total {
        let star = StarBuilder::build_primary().unwrap();
        match star.star_type {
            StarType::F | StarType::G | StarType::K => fgk += 1,
            StarType::M => m += 1,
            _ => {}
        }
    }
    (fgk as f64 / total as f64, m as f64 / total as f64)
}

#[test]
fn genre_stellar_model() {
    let (opera_fgk, _) = census("opera");
    let (normal_fgk, _) = census("normal");
    let (firm_fgk, firm_m) = census("firm");

    assert!(
        opera_fgk > normal_fgk,
        "opera FGK {opera_fgk:.2} should exceed normal {normal_fgk:.2}"
    );
    assert!(
        opera_fgk > firm_fgk,
        "opera FGK {opera_fgk:.2} should exceed firm {firm_fgk:.2}"
    );
    assert!(firm_m > 0.5, "firm should be M-dwarf-heavy, got {firm_m:.2}");
}

fn setup(seed: &str) {
    runtime::set_ruleset(Ruleset::load("t5", "").unwrap());
    runtime::set_genre("normal");
    runtime::set_sophonts("human");
    runtime::set_always_inhabited(true);
    runtime::set_tech_cap(None);
    rng::init_rng(seed);
}

/// Companions appear per the Ruby 2d6 table (8-11 -> 1, 12 -> 2): roughly 44% of
/// systems, and every companion also occupies an 'S' orbit slot or was pruned.
#[test]
fn companion_stars_generated() {
    setup("companions");
    let total = 300;
    let mut with_companions = 0;
    for _ in 0..total {
        let v = VolumeBuilder::new(0, 0).build().unwrap();
        let star = v.star.unwrap();
        if !star.companions.is_empty() {
            with_companions += 1;
            for c in &star.companions {
                assert!(!c.t5_classification().is_empty());
            }
        }
    }
    let frac = with_companions as f64 / total as f64;
    assert!(
        (0.25..=0.60).contains(&frac),
        "companion fraction {frac:.2} outside the 2d6>=8 expectation (~0.42)"
    );
}

/// Ruby zone tables: hostiles only spawn in the inner zone; the outer zone rolls
/// rockballs/belts/gas giants. With a biozone and always_inhabited, orbits past the
/// biozone can never be Hostile.
#[test]
fn no_hostiles_beyond_the_biozone() {
    setup("zones");
    for _ in 0..200 {
        let star = StarBuilder::build_primary().unwrap();
        let Some((_, b1)) = star.biozone() else { continue };
        for o in &star.orbits {
            if let OrbitContent::Hostile(h) = o {
                assert!(
                    h.au <= b1,
                    "hostile at {} au beyond biozone outer edge {} au",
                    h.au,
                    b1
                );
                assert!((10..=14).contains(&h.atmosphere), "hostile atmo {}", h.atmosphere);
                assert!(h.hydrographics <= 10);
            }
        }
    }
}

/// Gas-giant moon counts follow 2d6 (large) / 2d6-4 (small); radii come from the
/// Close (1-14) or Extreme (x25, max 350) tables.
#[test]
fn gas_giant_moons() {
    setup("moons");
    let mut seen = 0;
    for _ in 0..200 {
        let star = StarBuilder::build_primary().unwrap();
        for o in &star.orbits {
            if let OrbitContent::GasGiant(g) = o {
                seen += 1;
                assert!(g.moons.len() <= 12, "GG with {} moons", g.moons.len());
                for m in &g.moons {
                    assert!(
                        m.orbital_radius >= 1 && m.orbital_radius <= 350,
                        "moon radius {}",
                        m.orbital_radius
                    );
                }
            }
        }
    }
    assert!(seen > 0, "no gas giants generated in 200 systems");
}

/// Travel codes: every mainworld reads RZ, AZ, or clear, per the Ruby auto-assign.
#[test]
fn travel_codes_assigned() {
    setup("travel");
    let mut counts = std::collections::HashMap::new();
    for _ in 0..200 {
        let v = VolumeBuilder::new(0, 0).build().unwrap();
        if let Some(w) = v.world {
            *counts.entry(w.travel_code()).or_insert(0) += 1;
            if w.law_level >= 15 || w.government >= 15 {
                assert_eq!(w.travel_code(), "RZ");
            }
        }
    }
    assert!(counts.contains_key(".."), "no clear-zone worlds in 200 systems");
    assert!(counts.contains_key("AZ"), "no amber-zone worlds in 200 systems");
}

/// tech_cap is a hard ceiling on generated TL.
#[test]
fn tech_cap_respected() {
    setup("techcap");
    runtime::set_tech_cap(Some(7));
    for _ in 0..100 {
        let v = VolumeBuilder::new(0, 0).build().unwrap();
        if let Some(w) = v.world {
            assert!(w.tech_level <= 7, "TL {} exceeds tech_cap 7", w.tech_level);
        }
    }
    runtime::set_tech_cap(None);
}

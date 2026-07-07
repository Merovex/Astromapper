//! Optional `_astromapper.yml` config, loaded over built-in defaults. Precedence at
//! the CLI is: explicit flag > config file > defaults. Mirrors the Ruby/Go config.

use serde::Deserialize;
use std::fs;
use std::path::Path;

fn d_type() -> String {
    "sector".into()
}
fn d_name() -> String {
    "Unnamed".into()
}
fn d_density() -> String {
    "scattered".into()
}
fn d_genre() -> String {
    "normal".into()
}
fn d_ruleset() -> String {
    "t5".into()
}
fn d_sophonts() -> String {
    "human".into()
}
fn d_true() -> bool {
    true
}
fn d_jump() -> i64 {
    2
}
fn d_min() -> usize {
    2
}
fn d_opacity() -> f64 {
    0.85
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "d_type")]
    pub r#type: String,
    #[serde(default = "d_name")]
    pub name: String,
    #[serde(default = "d_density")]
    pub density: String,
    #[serde(default)]
    pub seed: String,
    #[serde(default = "d_genre")]
    pub genre: String,
    #[serde(default = "d_ruleset")]
    pub ruleset: String,
    #[serde(default = "d_sophonts")]
    pub sophonts: String,
    #[serde(default = "d_true")]
    pub prune_isolated: bool,
    /// Guarantee a mainworld in every biozone orbit (Ruby always_inhabited).
    #[serde(default = "d_true")]
    pub always_inhabited: bool,
    /// Optional tech-level ceiling.
    #[serde(default)]
    pub tech_cap: Option<i64>,
    #[serde(default = "d_true")]
    pub islands: bool,
    #[serde(default = "d_jump")]
    pub island_jump: i64,
    #[serde(default = "d_min")]
    pub island_min: usize,
    #[serde(default = "d_opacity")]
    pub island_opacity: f64,
}

impl Default for Config {
    fn default() -> Self {
        // `{}` deserializes every field from its #[serde(default)].
        serde_yaml::from_str("{}").unwrap()
    }
}

impl Config {
    /// Load `path` over the defaults. A missing file is fine (returns defaults,
    /// found=false). Keys absent from the file keep their default.
    pub fn load(path: &str) -> Result<(Config, bool), String> {
        if !Path::new(path).exists() {
            return Ok((Config::default(), false));
        }
        let raw = fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
        let cfg: Config = serde_yaml::from_str(&raw).map_err(|e| format!("{path}: {e}"))?;
        Ok((cfg, true))
    }
}

/// A commented `_astromapper.yml` scaffold with the given sector name (`new` command).
pub fn template(name: &str) -> String {
    format!(
        "# Astromapper (Rust) project config. Run `astromapper` in this directory to
# generate the sector. Any CLI flag overrides the value here.

type: sector            # sector | volume
name: \"{name}\"
density: scattered      # extra-galactic | rift | sparse | dunbar | scattered | dense | cluster | core
seed:                   # blank = random (a Crawford code is printed); or a code/string
genre: normal           # firm (realistic, M-dwarf-heavy) | normal | opera (Sun-like)
ruleset: t5             # t5 | cepheus | a custom rules/<name>.yml in this directory
sophonts: human         # human (Settled/Colony) | varied (alien sophonts)
prune_isolated: true    # drop systems with no neighbour within jump-4 (lone dots)
always_inhabited: true  # guarantee a mainworld in the biozone; false lets it roll a gas giant
tech_cap:               # optional tech-level ceiling (blank = uncapped)

# Island borders on the SVG (clusters of nearby systems)
islands: true
island_jump: 2          # systems within this many jumps form one island
island_min: 2           # minimum systems per island
island_opacity: 0.85    # 0.0 (invisible) .. 1.0 (solid)
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let d = Config::default();
        assert_eq!(d.ruleset, "t5");
        assert_eq!(d.density, "scattered");
        assert!(d.islands);
        assert_eq!(d.island_jump, 2);
    }

    #[test]
    fn partial_yaml_keeps_defaults() {
        let cfg: Config =
            serde_yaml::from_str("name: Frontier\nruleset: cepheus\nisland_min: 4\nislands: false\n")
                .unwrap();
        assert_eq!(cfg.name, "Frontier");
        assert_eq!(cfg.ruleset, "cepheus");
        assert_eq!(cfg.island_min, 4);
        assert!(!cfg.islands);
        // omitted keys keep their defaults
        assert_eq!(cfg.density, "scattered");
        assert_eq!(cfg.sophonts, "human");
    }

    #[test]
    fn template_round_trips() {
        let cfg: Config = serde_yaml::from_str(&template("Spinward Marches")).unwrap();
        assert_eq!(cfg.name, "Spinward Marches");
        assert_eq!(cfg.ruleset, "t5");
    }
}

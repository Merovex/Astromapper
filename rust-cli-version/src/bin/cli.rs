use astromapper_core::config::{self, Config};
use astromapper_core::formatters::{AsciiFormatter, JsonFormatter, SvgGenerator};
use astromapper_core::rules::{runtime, Ruleset};
use astromapper_core::{generate_crawford_seed, generate_sector, generate_volume, string_to_crawford};
use chrono::Local;
use clap::Parser;
use std::fs;
use std::path::Path;

/// Generation flags. Each is optional; an unset flag falls back to the config file,
/// then the built-in default. Precedence: flag > _astromapper.yml > default.
#[derive(Parser, Debug)]
#[command(name = "astromapper")]
#[command(about = "Traveller RPG Star Map Generator (`astromapper new <name>` scaffolds a project)")]
struct Args {
    /// Generation type: 'sector' or 'volume'
    #[arg(long)]
    r#type: Option<String>,
    /// Density (extra-galactic|rift|sparse|dunbar|scattered|dense|cluster|core)
    #[arg(long)]
    density: Option<String>,
    /// Seed string for generation
    #[arg(long)]
    seed: Option<String>,
    /// Name for the sector
    #[arg(long)]
    name: Option<String>,
    /// Ruleset: t5, cepheus, or a custom rules/<name>.yml
    #[arg(long)]
    ruleset: Option<String>,
    /// Stellar realism: firm | normal | opera
    #[arg(long)]
    genre: Option<String>,
    /// Native life: 'human' or 'varied'
    #[arg(long)]
    sophonts: Option<String>,
    /// Drop systems with no neighbour within jump-4
    #[arg(long)]
    prune: Option<bool>,
    /// Outline clusters of nearby systems on the SVG
    #[arg(long)]
    islands: Option<bool>,
    /// Systems within this many jumps form one island
    #[arg(long)]
    island_jump: Option<i64>,
    /// Minimum systems per island
    #[arg(long)]
    island_min: Option<usize>,
    /// Island border opacity, 0.0-1.0
    #[arg(long)]
    island_opacity: Option<f64>,
    /// YAML config file (default: _astromapper.yml; flags override it)
    #[arg(long)]
    config: Option<String>,
    /// List available density options
    #[arg(long)]
    list_densities: bool,
}

fn density_value(name: &str) -> Option<f64> {
    Some(match name {
        "extra-galactic" => 0.01,
        "rift" => 0.03,
        "sparse" => 0.17,
        "dunbar" => 0.23,
        "scattered" => 0.33,
        "dense" => 0.66,
        "cluster" => 0.83,
        "core" => 0.91,
        _ => return None,
    })
}

fn main() -> anyhow::Result<()> {
    // Subcommand: `astromapper new <name>` scaffolds a project directory.
    let raw: Vec<String> = std::env::args().collect();
    if raw.len() >= 2 && raw[1] == "new" {
        return run_new(&raw[2..]);
    }

    let args = Args::parse();

    if args.list_densities {
        println!("Available density options:");
        println!("  extra-galactic  (1%)  - Deep space between galaxies");
        println!("  rift           (3%)  - Galactic voids");
        println!("  sparse        (17%)  - Frontier regions");
        println!("  dunbar        (23%)  - ~150 systems (Dunbar's Number)");
        println!("  scattered     (33%)  - Outer rim (default)");
        println!("  dense         (66%)  - Inner systems");
        println!("  cluster       (83%)  - Stellar clusters");
        println!("  core          (91%)  - Galactic core");
        return Ok(());
    }

    // Merge: flag > config file > defaults.
    let cfg_path = args.config.clone().unwrap_or_else(|| "_astromapper.yml".into());
    let (cfg, found) = Config::load(&cfg_path).map_err(|e| anyhow::anyhow!(e))?;
    if found {
        println!("Config: {}", cfg_path);
    }
    let gen_type = args.r#type.unwrap_or(cfg.r#type);
    let density_name = args.density.unwrap_or(cfg.density);
    let name = args.name.unwrap_or(cfg.name);
    let ruleset_name = args.ruleset.unwrap_or(cfg.ruleset);
    let genre = args.genre.unwrap_or(cfg.genre);
    let sophonts = args.sophonts.unwrap_or(cfg.sophonts);
    let prune = args.prune.unwrap_or(cfg.prune_isolated);
    let islands = args.islands.unwrap_or(cfg.islands);
    let island_jump = args.island_jump.unwrap_or(cfg.island_jump);
    let island_min = args.island_min.unwrap_or(cfg.island_min);
    let island_opacity = args.island_opacity.unwrap_or(cfg.island_opacity);
    let seed_arg = args.seed.or_else(|| (!cfg.seed.is_empty()).then(|| cfg.seed.clone()));

    let Some(density) = density_value(&density_name) else {
        eprintln!("Error: Invalid density '{}'", density_name);
        eprintln!("Use --list-densities to see available options");
        std::process::exit(1);
    };

    // Seed generation/conversion.
    let (seed, crawford_code) = if let Some(s) = seed_arg {
        let crawford = string_to_crawford(&s);
        if s == crawford {
            println!("Using seed: {}", crawford);
        } else {
            println!("Input: {}", s);
            println!("Crawford code: {}", crawford);
        }
        (crawford.clone(), crawford)
    } else {
        let crawford = generate_crawford_seed();
        println!("Generated seed: {}", crawford);
        (crawford.clone(), crawford)
    };

    // Load the active ruleset and set the generation runtime before building.
    let rs = match Ruleset::load(&ruleset_name, ".") {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error loading ruleset {:?}: {}", ruleset_name, e);
            std::process::exit(1);
        }
    };
    let ruleset_title = rs.title();
    println!("Ruleset: {}  Genre: {}", ruleset_title, genre);
    runtime::set_ruleset(rs);
    runtime::set_genre(&genre);
    runtime::set_sophonts(&sophonts);
    runtime::set_always_inhabited(cfg.always_inhabited);
    runtime::set_tech_cap(cfg.tech_cap);

    fs::create_dir_all("output")?;

    match gen_type.as_str() {
        "sector" => {
            println!(
                "Generating sector with {} density ({}%)...",
                density_name,
                (density * 100.0) as u32
            );

            let mut sector = generate_sector(name, seed.clone(), density)?;
            if prune {
                sector.prune_isolated(4);
            }
            sector.ruleset_title = ruleset_title.clone();

            let ascii_content = AsciiFormatter::format_sector(&sector);
            let svg_content = SvgGenerator::new(sector.name.clone())
                .with_islands(islands, island_jump, island_min, island_opacity)
                .generate(&sector);
            let json_content = JsonFormatter::format_sector(&sector)?;
            let tab_content = sector.to_tab("");

            let timestamp = Local::now().format("%Y%m%d-%H%M%S");
            let base = format!("output/sector_{}_{}", crawford_code, timestamp);
            fs::write(format!("{base}.txt"), ascii_content)?;
            fs::write(format!("{base}.svg"), svg_content)?;
            fs::write(format!("{base}.json"), json_content)?;
            fs::write(format!("{base}.tab"), tab_content)?;

            println!("ASCII saved to: {base}.txt");
            println!("SVG saved to:   {base}.svg");
            println!("JSON saved to:  {base}.json");
            println!("TAB saved to:   {base}.tab");
            println!("Generated {} star systems in sector", sector.system_count());
        }
        "volume" => {
            println!("Generating single volume...");
            let volume = generate_volume(seed.clone(), 0, 0)?;
            let ascii_content = AsciiFormatter::format_volume(&volume);
            let json_content = JsonFormatter::format_volume(&volume)?;

            let timestamp = Local::now().format("%Y%m%d-%H%M%S");
            let ascii_path = format!("output/volume_{}_{}.txt", crawford_code, timestamp);
            let json_path = format!("output/volume_{}_{}.json", crawford_code, timestamp);
            fs::write(&ascii_path, ascii_content)?;
            fs::write(&json_path, json_content)?;

            println!("ASCII saved to: {}", ascii_path);
            println!("JSON saved to:  {}", json_path);
            if let Some(world) = &volume.world {
                println!("Generated system: {}", world.name);
            } else {
                println!("Generated empty hex");
            }
        }
        other => {
            eprintln!("Error: Invalid type '{}'. Must be 'sector' or 'volume'", other);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Scaffold a project directory (like Ruby's `astromapper new <name>`): the dir, an
/// _astromapper.yml seeded with the name, and an output/ folder.
fn run_new(args: &[String]) -> anyhow::Result<()> {
    let raw = args.join(" ");
    let raw = raw.trim();
    if raw.is_empty() {
        eprintln!("Usage: astromapper new <name>");
        std::process::exit(1);
    }
    let dir = raw.replace(' ', "-");
    let title = Path::new(&dir)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&dir)
        .replace('-', " ");

    if Path::new(&dir).exists() {
        eprintln!("Error: {:?} already exists", dir);
        std::process::exit(1);
    }
    fs::create_dir_all(format!("{dir}/output"))?;
    let cfg_path = format!("{dir}/_astromapper.yml");
    fs::write(&cfg_path, config::template(&title))?;

    println!("Created project {:?}", dir);
    println!("  {}", cfg_path);
    println!("  {}/output/", dir);
    println!("\nNext:  cd {} && astromapper", dir);
    Ok(())
}

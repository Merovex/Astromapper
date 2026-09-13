package main

import (
	"astromapper/pkg/builder"
	"astromapper/pkg/config"
	"astromapper/pkg/data"
	"astromapper/pkg/rng"
	"astromapper/pkg/rules"
	"astromapper/pkg/svg"
	"astromapper/pkg/writer"
	"crypto/rand"
	"encoding/json"
	"flag"
	"fmt"
	"hash/fnv"
	"math/big"
	"os"
	"path/filepath"
	"strings"
)

// densityMap matches the Ruby version's per-hex probabilities (builder/sector.rb).
// scattered is the default. There is no inflated "standard" (50%) — it produced far
// too many systems.
var densityMap = map[string]float64{
	"extra-galactic": 0.01,
	"rift":           0.03,
	"sparse":         0.17,
	"dunbar":         0.23, // ~150 systems (Dunbar's Number)
	"scattered":      0.33,
	"dense":          0.66,
	"cluster":        0.83,
	"core":           0.91,
}

func generateRandomSeed() string {
	// Excluding I, O, 0, 1 to avoid confusion
	const charset = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789"
	seed := make([]byte, 10)
	for i := range seed {
		n, _ := rand.Int(rand.Reader, big.NewInt(int64(len(charset))))
		seed[i] = charset[n.Int64()]
	}
	// Format as XXXXX-XXXXX for readability
	return string(seed[:5]) + "-" + string(seed[5:])
}

// Convert any string to a CRAWFORD-style 10-character code
func stringToCrawford(input string) string {
	// Excluding I, O, 0, 1 to avoid confusion
	const charset = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789"
	
	// Use FNV-1a hash to get a deterministic value from the input
	h := fnv.New64a()
	h.Write([]byte(input))
	hash := h.Sum64()
	
	// Convert hash to 10-character string
	result := make([]byte, 10)
	for i := 0; i < 10; i++ {
		// Use different parts of the hash for each character
		index := (hash >> (uint(i) * 6)) % uint64(len(charset))
		result[i] = charset[index]
	}
	
	// Format as XXXXX-XXXXX
	return string(result[:5]) + "-" + string(result[5:])
}

func main() {
	// Subcommand: `astromapper new <name>` scaffolds a project directory.
	if len(os.Args) >= 2 && os.Args[1] == "new" {
		runNew(os.Args[2:])
		return
	}

	// Define command-line flags
	var (
		configPath = flag.String("config", "_astromapper.yml", "Path to a YAML config file (optional; flags override it)")
		genType = flag.String("type", "sector", "Generation type: 'sector' or 'volume'")
		density = flag.String("density", "scattered", "Density for sector generation: extra-galactic, rift, sparse, dunbar, scattered, dense, cluster, core")
		seed    = flag.String("seed", "", "Seed string for generation (if not provided, generates random seed in format XXXXX-XXXXX)")
		name    = flag.String("name", "Unnamed", "Name for the sector (default: Unnamed)")
		genre   = flag.String("genre", "normal", "Stellar realism slider: firm (M-dwarf-heavy), normal, or opera (Sun-like)")
		ruleset = flag.String("ruleset", "t5", "Ruleset: t5, cepheus, or a custom rules/<name>.yml")
		sophonts = flag.String("sophonts", "human", "Native life: 'human' (Settled/Colony) or 'varied' (alien sophonts)")
		prune    = flag.Bool("prune", true, "Drop systems with no neighbour within jump-4 (lone, unreachable stars)")
		islands  = flag.Bool("islands", true, "Outline clusters of nearby systems on the SVG")
		islandJump = flag.Int("island-jump", 2, "Systems within this many jumps form one island")
		islandMin = flag.Int("island-min", 2, "Minimum systems per island to draw a border")
		islandOpacity = flag.Float64("island-opacity", 0.85, "Island border opacity, 0.0-1.0")
		help    = flag.Bool("help", false, "Show help message")
		listDensities = flag.Bool("list-densities", false, "List available density options")
	)

	flag.Parse()

	// Merge configuration: built-in defaults < _astromapper.yml < explicit flags.
	cfg, found, cfgErr := config.Load(*configPath)
	if cfgErr != nil {
		fmt.Fprintf(os.Stderr, "Error reading config %q: %v\n", *configPath, cfgErr)
		os.Exit(1)
	}
	set := map[string]bool{}
	flag.Visit(func(f *flag.Flag) { set[f.Name] = true })
	if !set["type"] {
		*genType = cfg.Type
	}
	if !set["name"] {
		*name = cfg.Name
	}
	if !set["density"] {
		*density = cfg.Density
	}
	if !set["seed"] {
		*seed = cfg.Seed
	}
	if !set["genre"] {
		*genre = cfg.Genre
	}
	if !set["ruleset"] {
		*ruleset = cfg.Ruleset
	}
	if !set["sophonts"] {
		*sophonts = cfg.Sophonts
	}
	if !set["prune"] {
		*prune = cfg.PruneIsolated
	}
	if !set["islands"] {
		*islands = cfg.Islands
	}
	if !set["island-jump"] {
		*islandJump = cfg.IslandJump
	}
	if !set["island-min"] {
		*islandMin = cfg.IslandMin
	}
	if !set["island-opacity"] {
		*islandOpacity = cfg.IslandOpacity
	}
	if found {
		fmt.Printf("Config: %s\n", *configPath)
	}

	// Show help if requested
	if *help {
		showHelp()
		os.Exit(0)
	}

	// List densities if requested
	if *listDensities {
		fmt.Println("Available density options:")
		fmt.Println("  extra-galactic  (1%)  - Deep space between galaxies")
		fmt.Println("  rift           (3%)  - Galactic voids")
		fmt.Println("  sparse        (17%)  - Frontier regions")
		fmt.Println("  dunbar        (23%)  - ~150 systems (Dunbar's Number)")
		fmt.Println("  scattered     (33%)  - Outer rim (default)")
		fmt.Println("  dense         (66%)  - Inner systems")
		fmt.Println("  cluster       (83%)  - Stellar clusters")
		fmt.Println("  core          (91%)  - Galactic core")
		os.Exit(0)
	}

	// Validate generation type
	if *genType != "sector" && *genType != "volume" {
		fmt.Fprintf(os.Stderr, "Error: Invalid type '%s'. Must be 'sector' or 'volume'\n", *genType)
		os.Exit(1)
	}

	// Get density value
	densityValue, ok := densityMap[*density]
	if !ok && *genType == "sector" {
		fmt.Fprintf(os.Stderr, "Error: Invalid density '%s'\n", *density)
		fmt.Println("Use --list-densities to see available options")
		os.Exit(1)
	}

	// Handle seed generation/conversion
	var seedStr string
	var crawfordCode string
	
	if *seed == "" {
		// No seed provided, generate random one
		crawfordCode = generateRandomSeed()
		seedStr = crawfordCode
		fmt.Printf("Generated seed: %s\n", crawfordCode)
	} else if len(*seed) == 11 && strings.Contains(*seed, "-") && len(strings.Split(*seed, "-")) == 2 {
		// Already in CRAWFORD format (XXXXX-XXXXX)
		crawfordCode = *seed
		seedStr = *seed
		fmt.Printf("Using seed: %s\n", crawfordCode)
	} else {
		// Convert any other string to CRAWFORD format
		crawfordCode = stringToCrawford(*seed)
		seedStr = crawfordCode
		fmt.Printf("Input: %s\n", *seed)
		fmt.Printf("Crawford code: %s\n", crawfordCode)
	}

	// Initialize RNG with seed
	r := rng.New(seedStr)

	// Load the active ruleset (project rules/<name>.yml overrides the built-in).
	rs, err := rules.Load(*ruleset, ".")
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error loading ruleset %q: %v\n", *ruleset, err)
		os.Exit(1)
	}
	builder.SetRuleset(rs)
	builder.SetSophonts(*sophonts)
	builder.SetGenre(*genre)
	builder.SetAlwaysInhabited(cfg.AlwaysInhabited)
	builder.SetTechCap(cfg.TechCap)
	fmt.Printf("Ruleset: %s  Genre: %s\n", rs.Title(), *genre)

	// Load planet names
	planetNames := data.GetPlanetNames()

	// Create file writer
	fileWriter := writer.New("output")

	// Generate based on type
	if *genType == "sector" {
		fmt.Printf("Generating sector with %s density (%.0f%%)...\n", *density, densityValue*100)
		
		// Generate sector
		sector := builder.BuildSector(*name, 32, 40, densityValue, planetNames, r)
		if *prune {
			sector.PruneIsolated(4)
		}
		sector.RulesetTitle = rs.Title()

		// Generate ASCII content
		asciiContent := sector.ToASCII()
		
		// Generate SVG content
		svgGen := svg.NewSVGGenerator(*name)
		svgGen.ShowIslands = *islands
		svgGen.IslandJump = *islandJump
		svgGen.IslandMin = *islandMin
		svgGen.IslandOpacity = *islandOpacity
		svgContent := svgGen.GenerateSector(sector)
		
		// Generate JSON content
		jsonData, err := json.MarshalIndent(sector, "", "  ")
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error generating JSON: %v\n", err)
			os.Exit(1)
		}
		jsonContent := string(jsonData)
		
		// Write files
		asciiPath, svgPath, jsonPath, err := fileWriter.WriteFiles(seedStr, asciiContent, svgContent, jsonContent, "sector")
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error writing files: %v\n", err)
			os.Exit(1)
		}
		
		fmt.Printf("ASCII saved to: %s\n", asciiPath)
		fmt.Printf("SVG saved to:   %s\n", svgPath)
		fmt.Printf("JSON saved to:  %s\n", jsonPath)

		// T5 Second Survey .tab (TravellerMap), alongside the other outputs.
		tabPath := strings.TrimSuffix(asciiPath, ".txt") + ".tab"
		if err := os.WriteFile(tabPath, []byte(sector.ToTab("")), 0o644); err != nil {
			fmt.Fprintf(os.Stderr, "Error writing tab file: %v\n", err)
			os.Exit(1)
		}
		fmt.Printf("TAB saved to:   %s\n", tabPath)

		// Count systems
		systemCount := 0
		for r := 0; r < sector.Height; r++ {
			for c := 0; c < sector.Width; c++ {
				if vol := sector.Volumes[r][c]; vol != nil && !vol.IsEmpty() {
					systemCount++
				}
			}
		}
		fmt.Printf("Generated %d star systems in sector\n", systemCount)
		
	} else {
		fmt.Println("Generating single volume...")
		
		// Generate volume
		volume := builder.BuildVolume(1, 1, planetNames, r)
		
		// Generate ASCII content
		asciiContent := volume.ToASCII()
		
		// Generate JSON content
		jsonData, err := json.MarshalIndent(volume, "", "  ")
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error generating JSON: %v\n", err)
			os.Exit(1)
		}
		jsonContent := string(jsonData)
		
		// Write files (no SVG for single volume)
		asciiPath, _, jsonPath, err := fileWriter.WriteFiles(seedStr, asciiContent, "", jsonContent, "volume")
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error writing files: %v\n", err)
			os.Exit(1)
		}
		
		fmt.Printf("ASCII saved to: %s\n", asciiPath)
		fmt.Printf("JSON saved to:  %s\n", jsonPath)
		
		if !volume.IsEmpty() {
			fmt.Printf("Generated system: %s\n", volume.Name)
		} else {
			fmt.Println("Generated empty hex")
		}
	}
}

// runNew scaffolds a project directory (like Ruby's `astromapper new <name>`):
// the dir, an _astromapper.yml seeded with the name, and an output/ folder.
func runNew(args []string) {
	raw := strings.TrimSpace(strings.Join(args, " "))
	if raw == "" {
		fmt.Fprintln(os.Stderr, "Usage: astromapper new <name>")
		os.Exit(1)
	}
	dir := strings.ReplaceAll(raw, " ", "-")
	title := strings.ReplaceAll(filepath.Base(dir), "-", " ")

	if _, err := os.Stat(dir); err == nil {
		fmt.Fprintf(os.Stderr, "Error: %q already exists\n", dir)
		os.Exit(1)
	}
	if err := os.MkdirAll(filepath.Join(dir, "output"), 0o755); err != nil {
		fmt.Fprintf(os.Stderr, "Error creating %q: %v\n", dir, err)
		os.Exit(1)
	}
	cfgPath := filepath.Join(dir, "_astromapper.yml")
	if err := os.WriteFile(cfgPath, []byte(config.Template(title)), 0o644); err != nil {
		fmt.Fprintf(os.Stderr, "Error writing %q: %v\n", cfgPath, err)
		os.Exit(1)
	}

	fmt.Printf("Created project %q\n", dir)
	fmt.Printf("  %s\n", cfgPath)
	fmt.Printf("  %s\n", filepath.Join(dir, "output")+string(os.PathSeparator))
	fmt.Printf("\nNext:  cd %s && astromapper\n", dir)
}

func showHelp() {
	fmt.Println("Astromapper - Traveller RPG Star Map Generator")
	fmt.Println()
	fmt.Println("Usage: astromapper new <name>     Scaffold a project directory")
	fmt.Println("       astromapper [options]      Generate (reads _astromapper.yml if present)")
	fmt.Println()
	fmt.Println("Options:")
	fmt.Println("  --config <path>      YAML config file (default: _astromapper.yml; flags override it)")
	fmt.Println("  --type <type>        Generation type: 'sector' or 'volume' (default: sector)")
	fmt.Println("  --density <density>  Density for sector generation (default: scattered)")
	fmt.Println("                       Options: extra-galactic, rift, sparse, dunbar,")
	fmt.Println("                                scattered, dense, cluster, core")
	fmt.Println("  --seed <string>      Seed string for generation")
	fmt.Println("                       If not provided, generates random seed (format: XXXXX-XXXXX)")
	fmt.Println("  --name <string>      Name for the sector (default: Unnamed)")
	fmt.Println("  --genre <genre>      Stellar realism: firm | normal (default) | opera")
	fmt.Println("  --ruleset <name>     Ruleset: t5 (default), cepheus, or a custom rules/<name>.yml")
	fmt.Println("  --sophonts <mode>    Native life: 'human' (default) or 'varied' (alien sophonts)")
	fmt.Println("  --prune              Drop systems with no neighbour within jump-4 (default true)")
	fmt.Println("  --islands            Outline clusters of nearby systems on the SVG (default true)")
	fmt.Println("  --island-jump <n>    Systems within n jumps form one island (default 2)")
	fmt.Println("  --island-min <n>     Minimum systems per island (default 2)")
	fmt.Println("  --island-opacity <f> Island border opacity 0.0-1.0 (default 0.85)")
	fmt.Println("  --list-densities     List available density options with descriptions")
	fmt.Println("  --help               Show this help message")
	fmt.Println()
	fmt.Println("Examples:")
	fmt.Println("  astromapper                                    # Generate scattered sector with random seed")
	fmt.Println("  astromapper --seed MYSEED123                  # Generate sector with specific seed")
	fmt.Println("  astromapper --density sparse --seed FRONTIER  # Generate sparse sector")
	fmt.Println("  astromapper --type volume --seed ALPHA7       # Generate single star system")
	fmt.Println("  astromapper --name \"Spinward Marches\"         # Generate sector with custom name")
	fmt.Println()
	fmt.Println("Output:")
	fmt.Println("  Files are saved to the 'output' directory:")
	fmt.Println("  - ASCII text file with system data")
	fmt.Println("  - SVG vector graphic (sector only)")
	fmt.Println("  - JSON data file")
}
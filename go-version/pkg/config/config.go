// Package config loads an optional _astromapper.yml so the Go build can be driven
// by a project file (like the Ruby version) instead of repeating flags. Precedence
// is: explicit CLI flag > config file > built-in defaults.
package config

import (
	"os"

	"gopkg.in/yaml.v3"
)

// Config is the on-disk generation settings. Keys mirror the Ruby _astromapper.yml
// where the feature exists in the Go build.
type Config struct {
	Type          string  `yaml:"type"`
	Name          string  `yaml:"name"`
	Density       string  `yaml:"density"`
	Seed          string  `yaml:"seed"`
	Genre         string  `yaml:"genre"`
	Ruleset       string  `yaml:"ruleset"`
	Sophonts      string  `yaml:"sophonts"`
	PruneIsolated bool    `yaml:"prune_isolated"`
	// AlwaysInhabited guarantees a mainworld in the biozone (Ruby always_inhabited).
	AlwaysInhabited bool `yaml:"always_inhabited"`
	// TechCap is an optional tech-level ceiling (nil = uncapped).
	TechCap *int `yaml:"tech_cap"`
	Islands       bool    `yaml:"islands"`
	IslandJump    int     `yaml:"island_jump"`
	IslandMin     int     `yaml:"island_min"`
	IslandOpacity float64 `yaml:"island_opacity"`
}

// Defaults returns the built-in configuration (used when no file/flag overrides it).
func Defaults() Config {
	return Config{
		Type:          "sector",
		Name:          "Unnamed",
		Density:       "scattered",
		Seed:          "",
		Genre:         "normal",
		Ruleset:       "t5",
		Sophonts:      "human",
		PruneIsolated:   true,
		AlwaysInhabited: true,
		Islands:       true,
		IslandJump:    2,
		IslandMin:     2,
		IslandOpacity: 0.85,
	}
}

// Template returns a commented _astromapper.yml scaffold with the given sector name,
// written by `astromapper new`. Values are the built-in defaults.
func Template(name string) string {
	return `# Astromapper (Go) project config. Run ` + "`astromapper`" + ` in this directory
# to generate the sector. Any CLI flag overrides the value here.

type: sector            # sector | volume
name: "` + name + `"
density: scattered      # extra-galactic | rift | sparse | dunbar | scattered | dense | cluster | core
seed:                   # blank = random (a Crawford code is printed); or a code/string
genre: normal           # firm (realistic, M-dwarf-heavy) | normal | opera (Sun-like)
ruleset: t5             # t5 | cepheus | a custom rules/<name>.yml in this directory
sophonts: human         # human (Settled/Colony) | varied (alien sophonts)

prune_isolated: true     # drop systems with no neighbour within jump-4 (lone dots)
always_inhabited: true   # guarantee a mainworld in the biozone; false lets it roll a gas giant
tech_cap:                # optional tech-level ceiling (blank = uncapped)

# Island borders on the SVG (clusters of nearby systems)
islands: true
island_jump: 2          # systems within this many jumps form one island
island_min: 2           # minimum systems per island
island_opacity: 0.85    # 0.0 (invisible) .. 1.0 (solid)
`
}

// Load reads path over the defaults. yaml.Unmarshal only sets keys present in the
// file, so omitted keys keep their default. Returns found=false (no error) when the
// file does not exist, so a missing config is fine.
func Load(path string) (cfg Config, found bool, err error) {
	cfg = Defaults()
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return cfg, false, nil
		}
		return cfg, false, err
	}
	if err := yaml.Unmarshal(data, &cfg); err != nil {
		return cfg, false, err
	}
	return cfg, true, nil
}

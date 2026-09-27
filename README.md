# Astromapper

A multi-language implementation of a Traveller RPG star map generator that creates random sectors, star systems, and worlds following Traveller5 and Classic Traveller rules. Generate detailed astronomical data including orbits, moons, factions, and trade codes for your science fiction RPG campaigns.

## Features

- Generate full 32x40 hex sectors or individual star systems
- Multiple density options from extra-galactic to galactic core
- Deterministic generation using seed values (any string, converted to XXXXX-XXXXX format)
- Output formats: ASCII text, SVG graphics, and JSON data
- Detailed orbital mechanics with AU distances
- Moon generation with orbital radius calculations
- Faction generation for inhabited worlds
- Trade code classification
- Travel zones and starport ratings
- Web-based display using Stimulus.js

## Implementations

### Go Version (Recommended)

The Go implementation is the recommended version for production use, offering the best performance and most complete feature set.

#### Installation

```bash
cd go-version
go build
```

#### Usage

```bash
# Generate a standard sector with random seed
./astromapper

# Generate a sector with any string as seed (automatically converted to XXXXX-XXXXX format)
./astromapper --seed "My Campaign World"

# Generate a sparse frontier sector
./astromapper --density sparse --seed "FRONTIER"

# Generate a single star system
./astromapper --type volume --seed "Alpha Centauri"

# Generate a named sector
./astromapper --name "Spinward Marches" --density standard
```

#### Options

- `--type`: Generation type: 'sector' or 'volume' (default: sector)
- `--density`: Sector density options:
  - `extra-galactic` (1%) - Deep space between galaxies
  - `rift` (3%) - Galactic voids
  - `sparse` (17%) - Frontier regions
  - `scattered` (33%) - Outer rim
  - `standard` (50%) - Typical space
  - `dense` (66%) - Inner systems
  - `cluster` (83%) - Stellar clusters
  - `core` (91%) - Galactic core
- `--seed`: Any string for deterministic generation (converted to XXXXX-XXXXX Crawford format internally)
- `--name`: Name for the sector (default: Unnamed)
- `--list-densities`: Show all density options
- `--help`: Show help message

### Rust Version

A high-performance Rust implementation with identical features to the Go version.

#### Installation

```bash
cd rust-cli-version
cargo build --release
```

#### Usage

```bash
# Generate a sector with any seed string
cargo run --release -- --type sector --seed "My Rust Sector"

# Or use the built binary
./target/release/astromapper --type sector --density dense --seed "Core Worlds Beta"
```

The Rust version supports the same command-line options as the Go version.

### Ruby Version (Deprecated)

The original Ruby implementation is currently deprecated but remains in the repository for reference. It includes the original algorithms and game mechanics that were ported to Go and Rust.

```bash
cd ruby-version
bundle install
# See ruby-version/README.md for historical usage
```

## Output Formats

### ASCII Text

Traditional text-based sector maps showing:

- Location coordinates
- UWP (Universal World Profile) codes
- Temperature zones
- Bases (Naval, Scout, Gas Giant, Consulate, Pirate)
- Trade codes
- Faction presence
- Star classifications
- Orbital layouts

### SVG Graphics

Scalable vector graphics showing:

- Hexagonal sector grid
- System presence indicators
- Star classifications by color
- Trade route potential

### JSON Data

Structured data with:

- Volumes as a hash with zero-padded coordinate keys (e.g., "0801")
- Complete star system data
- Nested orbit structures with type and data fields
- Moon arrays with orbital radius in planetary radii
- World details including factions and trade codes

## How to Display in Jekyll

The repository includes a web-based viewer for displaying generated sectors using Jekyll and Stimulus.js.

### Setup

1. Generate sector data using the Go version (recommended for best compatibility):

```bash
cd go-version
./astromapper --seed "My Web Campaign" --name "My Sector"

# Files generated in output/ directory:
# - sector_*.txt  (ASCII format)
# - sector_*.svg  (Visual map)
# - sector_*.json (Data for interactive display)
```

2. Copy the display files to your Jekyll site:

```bash
# Copy the HTML template
cp jekyll-display/astromap.html /path/to/jekyll/site/

# Copy the Stimulus controller
cp jekyll-display/astromap-controller.js /path/to/jekyll/site/assets/js/

# Copy your generated SVG files
cp go-version/output/*.svg /path/to/jekyll/site/assets/

# Copy your sector JSON files
cp jekyll-display/assets/astromaps/*.json /path/to/jekyll/site/assets/astromaps/
```

3. Include in your Jekyll page:

```html
<!-- In your Jekyll layout or page -->

<!-- SVG Sector Map Display -->
<div class='w-full border-2 border-brand dark:border-brand-light' style='height: 800px; overflow: hidden;'>
  <embed id="map" src="/assets/sector_ORBITS-TEST_20250818-090851.svg" type="image/svg+xml" width="100%" height="100%" />
</div>

<!-- Interactive System Browser -->
<div data-controller="astromap" 
     data-astromap-data-url-value="/assets/astromaps/mysector.json">
  <!-- Astromap display elements -->
  <input data-astromap-target="coordinates" type="text" placeholder="0801" />
  <div data-astromap-target="name"></div>
  <div data-astromap-target="uwp"></div>
  <div data-astromap-target="orbits"></div>
  <!-- See astromap.html for complete structure -->
</div>

<script type="module" src="/assets/js/astromap-controller.js"></script>
```

### Features

The Jekyll display provides:

- Visual SVG sector map with embedded display
- Interactive sector browsing
- Click on coordinates to view system details
- Orbital displays with moon information
- Trade route calculations
- UWP translations with detailed descriptions
- Support for both old and new JSON formats

### Customization

The Stimulus controller (`astromap-controller.js`) can be customized to:

- Change display formatting
- Add custom trade codes
- Modify distance calculations
- Integrate with your Jekyll theme

## JSON Format

The current JSON format uses a hash structure for volumes with zero-padded coordinate keys:

```json
{
  "name": "Sector Name",
  "volumes": {
    "0801": {
      "name": "System Name",
      "column": 8,
      "row": 1,
      "star": {
        "star_type": "G",
        "star_size": 2,
        "orbits": [
          {
            "type": "world",
            "data": {
              "orbit_number": 3,
              "au": 1.0,
              "starport": "B",
              "size": 7,
              "atmosphere": 6,
              "hydrographics": 6,
              "population": 8,
              "government": 4,
              "law_level": 5,
              "tech_level": 9,
              "temperature": "T",
              "factions": ["N", "F", "M"],
              "trade_codes": ["Ag", "Ni"],
              "moons": [
                {
                  "orbit": 0,
                  "orbital_radius": 11,
                  "size": 2,
                  "atmosphere": 0,
                  "hydrographics": 0
                }
              ]
            }
          }
        ]
      }
    }
  }
}
```

## Seed System

Seeds can be any string of any length. The system will:

1. Accept any string as input (e.g., "My Campaign", "Spinward Marches", "Test123")
2. Convert it deterministically to the XXXXX-XXXXX Crawford format
3. Use this converted seed for reproducible generation
4. Display both the original input and Crawford code in output

This ensures that:

- The same input string always generates the same sector
- Seeds are human-readable and memorable
- Generation remains deterministic across all implementations

## Development

### Project Structure

```
Astromapper/
├── go-version/          # Go implementation (recommended)
├── rust-cli-version/    # Rust implementation
├── ruby-version/        # Original Ruby version (deprecated)
├── jekyll-display/      # Web display components
├── old-version.json     # Legacy format example
├── new-version.json     # Current format example
└── CLAUDE.md           # Development notes and learnings
```

### Contributing

When contributing, please:

1. Maintain compatibility between Go and Rust implementations
2. Preserve the deterministic generation using seeds
3. Follow Traveller RPG rules for world generation
4. Update both implementations when adding features
5. Test with consistent seeds across implementations

## License

The software is released under the **MIT License**. Game rules (Traveller, GURPS) are used under their respective fair-use policies. See [LICENSE.md](LICENSE.md) for the full text, including the Far Future Enterprises Traveller Fair Use Policy and the Steve Jackson Games / GURPS notice.

## Acknowledgments

Based on Traveller5 (FFE) and Classic Traveller RPG rules for world and system generation, with GURPS Space 4e for stellar/orbital physics.
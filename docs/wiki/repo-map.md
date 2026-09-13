# Repo Map

Updated: 2026-07-07

Astromapper generates random Traveller RPG star sectors (ASCII, SVG, `.tab`, JSON).
Three implementations share the same data-driven rulesets and seeding scheme.

## Top level

| Path | What it is |
|---|---|
| `ruby-version/` | The original gem. **Reference implementation** — richest feature set. |
| `go-version/` | Go port. Ruleset engine mirrored (`pkg/rules`), rules YAML embedded via `go:embed`. Go toolchain via `mise`, not on PATH. |
| `rust-cli-version/` | Rust port. Library crate `astromapper_core` + `astromapper` CLI binary. As of 2026-07-07 the stellar/orbital math is converged on Ruby (see [parity.md](parity.md)). |
| `jekyll-display/` | Stimulus/Jekyll viewer for generated JSON (coordinate lookup, orbit tables, jump-3 routes, svg-pan-zoom). Consumes the Rust/Go JSON format. |
| `docs/` | `generation-pipeline.md` (math provenance) + this wiki. |
| `teradoma-astromapper.yml` | Project config for the Teradoma canon sector. |

## ruby-version/ highlights

- `lib/astromapper/builder/` — sector/volume/star/orbit generation (the heart).
- `lib/astromapper/rules/` — `Expr` (sandboxed evaluator) + `Ruleset` (YAML loader).
- `rules/t5.yml`, `rules/cepheus.yml` — the rulesets (byte-identical copies live in
  `rust-cli-version/src/rules/builtin/` and `go-version/pkg/rules/builtin/`).
- `lib/astromapper/seed.rb` — Crawford codes + FNV-1a.
- `lib/astromapper/islands.rb` — shared island-border geometry (SVG + tools).
- `tools/` — Ruby-only post-processors: `json2sector/svg/tab`, `sector2svg`,
  `enrich` (layer T5 extensions onto lean JSON), `canon` (overlay canon names),
  `island-borders`, `island-conflicts`.
- `macropedia/`, `teradoma-canon/` — canon reference data and pre-built exports.
- `test/` — Minitest: golden masters (T5 + Cepheus fixtures), T5 unit tests, Expr tests.

## rust-cli-version/ highlights

- `src/models/` — `star.rs` (stellar tables: INNER_LIMIT/BIOZONE/MASS/STAR_CHART,
  Bode, companion separation), `orbit.rs` (orbit variants incl. Companion, Moon),
  `world.rs` (UWP, travel_code, extensions), `sector.rs` (prune_isolated, to_tab).
- `src/builders/` — `star_builder.rs` (genre spectral model, companions, forbidden
  zones), `orbit_builder.rs` (zone tables, moons, prune), `world_builder.rs`
  (ruleset-driven UWP + T5 modules), `sector_builder.rs`, `volume_builder.rs`.
- `src/rules/` — `expr.rs`, `ruleset.rs`, `runtime.rs` (thread-local genre /
  sophonts / always_inhabited / tech_cap).
- `src/rng.rs` — ChaCha8 seeded via FNV-1a; Crawford code helpers in `lib.rs`.
- `tests/generation.rs` — behavioral tests (genre census, companions, zones, moons,
  travel codes, tech_cap).

## Commands (Rust CLI)

`astromapper new <name>` scaffolds; bare `astromapper` generates using
`_astromapper.yml` + flags (`--seed`, `--density`, `--ruleset`, `--genre`,
`--config`, `--list-densities`, island flags). One run emits `.txt`, `.svg`,
`.json`, `.tab` into `output/`.

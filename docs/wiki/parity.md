# Cross-Implementation Parity

Updated: 2026-07-07

## Identical by construction (verified 2026-07-07)

- **Ruleset YAMLs**: `rules/t5.yml` and `rules/cepheus.yml` are byte-identical
  across ruby-version, rust-cli-version (`src/rules/builtin/`), go-version
  (`pkg/rules/builtin/`). If you edit one, copy to all three.
- **Rules engine semantics**: expression grammar/precedence, dice (`NdM`, `flux` =
  1d6−1d6), `extends:` deep-merge with trailing-`!` wholesale replacement,
  project-rules override, validation.
- **Seeding**: Crawford charset (`ABCDEFGHJKLMNPQRSTUVWXYZ23456789`), XXXXX-XXXXX
  format, FNV-1a 64-bit (offset `0xcbf29ce484222325`, prime `0x100000001b3`).
  Same input → same code + same seed integer in all three.
- **eHex**: `0-9 A-H J-N P-Z` (skips I/O) everywhere.
- **`.tab` (T5 Second Survey)**: same columns, order, and semantics.
- **T5 world modules**: Ix/Ex/Cx + RU (zero→1 rule), HZ-variance climate, native
  status, genre realism passes, gravity/hot-star population caps, travel zones.

## Rust ↔ Ruby stellar/orbital convergence (ported 2026-07-07)

The Rust port originally had a simplified stellar layer. It now carries Ruby's:
INNER_LIMIT / BIOZONE / MASS / STAR_CHART tables, spectral subtype arrays, Bode
table (+ M-dwarf V = 0.2), `orbit_to_au = inner_limit + round1(bode·2^n)`,
`outer_limit = 40·mass`, orbit-count DMs, zone placement tables (inner 2d6:
&lt;5 empty / 5-6 Hostile / 7-9 Rockball / 10-11 Belt / 12 GG; biozone:
always_inhabited → World, else 2d6&lt;12; outer 1d6+distant: 1 R / 2 B / 3 empty /
4-7 GG), no-biozone ⇒ all-inner, companion stars (count/separation/type/size) with
GURPS forbidden zones, trailing-empty prune + renumber, gas-giant-size-aware moons
(2d6, small −4; radius tables Close/Ring/Extreme), Hostile atmo 10-14, hydro
2d6−4 capped A, `tech_cap`, `always_inhabited`, travel zones.

### Deliberate divergences from Ruby (bugs not replicated)

| Ruby behavior | Rust behavior | Why |
|---|---|---|
| `GasGiant#make_moons` returns `nil` (no moons) when the preliminary Planet moon array had <2 entries (`Integer#size == 8` accident) | GG always gets its rolled 2d6(−4) moons | Ruby accident, not a rule |
| Wasted preliminary moon roll in `Planet#initialize` for GGs | Skipped | Only affects RNG draw order; cross-language draw parity is a non-goal |
| Negative forbidden-zone / insertion indices wrap to the array tail | Clamped to 0 | Ruby indexing accident |
| Companion stars generate their own (never displayed) orbit systems | Companions carry no orbits | Pure wasted draws |
| `Moon` "Far" radius table (Close×5) defined but unreachable | Not ported | Dead code |

## Not parity goals (accepted)

- **Cross-language byte-identical maps.** Ruby (MT19937) / Go (math/rand) / Rust
  (ChaCha8) use different RNGs and draw orders. Same seed ⇒ same Crawford code and
  integer, different maps. Per-language reproducibility only.
- **ASCII `.sector` line format.** Ruby is tab-delimited; Rust is space-aligned
  fixed-width. Interchange happens via `.tab` and JSON, which do match.

## Known gaps (as of 2026-07-07)

- **Go stellar layer**: go-version's world gen was converged on T5 rulesets, but
  its stellar/orbital layer has NOT been audited against the 2026-07-07 Ruby↔Rust
  convergence. Assume it still has the simplified model until checked.
- **Ruby-only**: `about <hex>` command, canon-override pipeline
  (`apply_overrides!` / `apply_star_override!` / `ensure_gas_giants!`), the
  `tools/` post-processors. Rust has none of these yet (see [roadmap.md](roadmap.md)).
- **Rust-only**: single-run multi-format output, `--config`/`--list-densities`,
  JSON output format consumed by jekyll-display.

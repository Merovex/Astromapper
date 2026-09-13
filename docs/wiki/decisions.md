# Decisions

Updated: 2026-07-07

Standing decisions with rationale. Don't re-litigate without new information.

- **Per-language reproducibility, not cross-language byte parity** (2026-06).
  All three implementations derive the same Crawford code + seed integer (FNV-1a),
  but keep their native RNGs. Identical cross-language maps would require one
  shared RNG plus identical draw order — deliberately not pursued.

- **Rulesets are data, not code** (2026-06-05, Ruby "phases 1-8"; ported to Go and
  Rust 2026-06-06). Tabular rules live in `rules/<name>.yml`; algorithmic steps
  (Ix/Ex/Cx, climate, native) are named code modules selected by the YAML
  `modules:` block. The Expr evaluator is sandboxed by construction (no
  eval/send); YAML can never execute arbitrary code.

- **The T5 golden master is sacred** (Ruby). The phased ruleset extraction kept
  the Ruby T5 fixture byte-identical by preserving roll formulas and dice draw
  order. Any intentional generation change requires `UPDATE_GOLDEN=1 rake test`
  and a commit message saying why.

- **GURPS orbital math is not modified** (project-wide). Stellar/orbital physics
  (separation, limits, spacing, forbidden zones) come from GURPS Space 4e
  pp. 104-107 and are ported as-is, even where odd (e.g. `au_to_orbit` subtracting
  an AU inner limit from a log2 orbit number).

- **Ruby accidents are not spec** (2026-07-07, Rust port). Where Ruby behavior is
  demonstrably accidental (`Integer#size` moon bug, negative-index wraparound),
  the Rust port implements the evident intent and the divergence is recorded in
  [parity.md](parity.md). Where behavior is odd-but-deliberate (biozone moons get
  atmo 0, F radius table unreachable), the observable outcome is preserved.

- **Genre is a stellar-model slider, not a die modifier** (2026-06). `STAR_BIAS`
  is all-zero; firm/normal/opera select different spectral tables, plus
  population-realism passes and the F-and-hotter colony cap (pop ≤ 6).

- **Cepheus rule values need spot-checking** (2026-06). They were transcribed from
  the SRD from memory. Verify against the SRD before relying on them.

- **Tauri app builds on `astromapper_core`** (2026-07-07, direction). The Rust
  crate is the engine for the planned Tauri app; Ruby stays the reference for
  behavior questions. See [roadmap.md](roadmap.md).

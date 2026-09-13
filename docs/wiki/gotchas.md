# Gotchas

Updated: 2026-07-07

Traps that have burned (or nearly burned) contributors. Read before porting or
comparing implementations.

## Ruby extension semantics (the big one)

`ruby-version/lib/astromapper/extensions/integer.rb` redefines core-sounding
methods **backwards**:

- `Integer#max(n)` returns the **smaller** of self and n — it's a **cap**.
- `Integer#min(n)` returns the **larger** — it's a **floor**.
- `Integer#whole` floors at 0.

So `@popx.whole.max(15)` means clamp(0, 15), and `@tek.max(config['tech_cap'])`
means "cap TL at tech_cap". Any naive reading of Ruby code inverts these.

## Dice notation

`toss(a, b)` in `builder.rb` is `(a d6) − b`, **floored at 0** — and `b` defaults
to 2, so bare `toss` = 2d6−2 (0..10). `flux` = 1d6−1d6 (−5..+5, NOT floored).
`toss(2,0)` = plain 2d6. `toss(1,3)` = 0..3. Misreading `toss(2,4)` as "2d6+4"
(instead of 2d6−4) has already produced a wrong analysis once.

## Ruby quirks that shape output

- `Planet#make_moons` keys moons by orbital radius in a Hash — moons at the same
  radius **collapse** (later wins). Ported to Rust.
- Biozone moons always get atmosphere 0 (the `else 0` arm). Deliberately preserved.
- Multiple Worlds per system are possible (every biozone orbit rolls one);
  Ruby `@world` — and therefore the mainworld — is the **last** one.
- `Star#prune!` renumbers all orbits sequentially and recomputes AU after
  stripping trailing empties — orbit numbers in output are post-prune.
- A companion's forbidden zone can delete the mainworld's orbit slot; the Volume
  keeps its (already captured) mainworld anyway. Mirrored in Rust.
- The Ruby `.sector` ASCII line is tab-delimited between the trade-code and later
  columns; parsers key on `/^\d{4}/`.

## Agent/analysis traps

- Exploration agents have produced **confidently wrong parity claims** (said
  prune_isolated and named densities were missing from Rust when both existed;
  misread `toss`). Verify load-bearing claims by reading the code yourself.
- The Cepheus golden fixture is internally reproducible but NOT comparable to T5
  output; the Cepheus values themselves are unverified against the SRD.

## Environment

- Go is installed via `mise`, not on PATH.
- Ruby needs `YAML.unsafe_load` (Psych 4+) for the config's `!ruby/range`;
  `bin/astromapper` prepends `lib/` so a source checkout runs without install.
- Rust crate: models are `Serialize` but mostly not round-trip-tested for
  `Deserialize` (needed for the Tauri read path — see roadmap).

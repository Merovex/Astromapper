# Roadmap

Updated: 2026-07-07

## Goal: Tauri desktop app

Generate a sector (or read an existing one) and emit **Jekyll-style documents**
(YAML frontmatter + markdown body) the user can read AND edit — files are the
data store. Built on the `astromapper_core` Rust crate.

### Done

- ✅ Rust stellar/orbital layer converged on Ruby (2026-07-07): companions +
  forbidden zones, full stellar tables, Ruby zone/moon math, travel zones,
  tech_cap, always_inhabited. See [parity.md](parity.md).

### Remaining, in dependency order

1. **Round-trip**: add `Deserialize` load path for the sector JSON (models already
   derive it — needs a `Sector::from_json` + tests). Optional `.sector` ASCII
   importer for old files.
2. **`describe_volume()`**: port Ruby's `about <hex>` logic (UWP prose breakdown,
   orbit/moon tables, jump-3 routes) into the crate — one implementation feeding
   both the Tauri detail view and generated document bodies.
   (jekyll-display's `astromap-controller.js` has a JS version to crib from.)
3. **Canon-override merge**: port `apply_overrides!` / `apply_uwp!` /
   `apply_star_override!` / `ensure_gas_giants!` / `repatch!` from Ruby. This is
   the mechanism by which user edits to frontmatter survive regeneration.
4. **Jekyll document emitter**: fan a Sector out into e.g. `_worlds/0101-name.md`
   — generated data in frontmatter, user prose in the body. Merge discipline:
   frontmatter regenerable, body never touched, user frontmatter edits = canon
   overrides. Seed + config stored so the sector is reproducible from files alone.
5. **Tauri shell**: commands wrapping generate/load/save/regenerate-svg; frontend
   viewer adapted from `jekyll-display/` + a frontmatter/markdown editor.

### Also worth doing

- Audit **go-version**'s stellar layer against the new Ruby↔Rust convergence
  (it predates the 2026-07-07 port and likely still has the simplified model).
- Ruby cleanup: dead exporter stubs (PDF/EPUB), `String#to_coords`,
  `Array#overlaps?`, stray `# exit` in star.rb.
- Spot-check Cepheus rule values against the SRD.

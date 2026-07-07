# Astromapper LLM Wiki

A working memory for humans **and** LLM agents contributing to this repo. Read this
index first; each page is small, factual, and dated so a fresh session can rebuild
context without re-deriving it from the code.

## Pages

| Page | What it answers |
|---|---|
| [repo-map.md](repo-map.md) | What lives where — the three implementations, tools, data dirs |
| [parity.md](parity.md) | Feature/math parity across Ruby, Go, Rust — what matches, what deliberately doesn't |
| [decisions.md](decisions.md) | Standing decisions and their rationale (don't re-litigate) |
| [gotchas.md](gotchas.md) | Traps: Ruby extension semantics, dice notation, quirks preserved or dropped |
| [roadmap.md](roadmap.md) | Where this is going (Tauri app, Jekyll-style documents) |
| [../generation-pipeline.md](../generation-pipeline.md) | Full walkthrough of the generation math and its rule-system provenance |

## Maintenance rules

- **Update, don't append-only.** When a fact changes, fix the page and refresh its
  `Updated:` line. Stale facts are worse than missing ones.
- **Record decisions with the "why".** A decision without rationale gets re-argued.
- **Date everything.** Every page carries `Updated: YYYY-MM-DD` at the top.
- **Cross-check before trusting.** Pages describe code; verify file paths still
  exist before acting on them. If a page and the code disagree, the code wins —
  then fix the page.
- **Keep CLAUDE.md the entry point.** CLAUDE.md holds instructions for working in
  the repo; this wiki holds knowledge about it. Don't duplicate — link.

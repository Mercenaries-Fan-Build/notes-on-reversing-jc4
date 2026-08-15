# Reversing Just Cause 4

A reverse-engineering & modding knowledge base for **Just Cause 4** (Avalanche Studios, 2018),
built on the **Apex ("Avalanche Open World") engine**. Third in a series of the same effort —
after [Mercenaries 2](../notes-on-the-released-game) (the most built-out) and
[The Saboteur](../notes-on-reversing-the-sabetour). We reuse the *methodology* from those
projects, not their byte-level facts: JC4 is a different engine lineage (Apex, not Pandemic),
so almost nothing transfers below the level of "how we work."

## What this is

The goal is to bootstrap a modding scene the way the two prior games got one: crack the archive
and asset formats, build small round-trip-verified Rust tools that read **and write** them, find
the engine's built-in patch/override layer so mods ship as additive reversible packs, and capture
every finding as evidence-graded notes indexed by a local RAG server.

## Orientation — read in this order

1. [`AGENTS.md`](AGENTS.md) — working conventions; the prime directive (verify lineage before reuse).
2. [`docs/lineage_and_divergence.md`](docs/lineage_and_divergence.md) — **what carries over from JC2/Apex
   and the prior projects, and what must be re-derived.** Read before reusing anything.
3. [`docs/binary_recon.md`](docs/binary_recon.md) — the install survey: engine, archives, shipped
   debug tooling, the format stack we have to crack.
4. [`docs/tools/README.md`](docs/tools/README.md) — task-oriented "I want to…" index of our tools.

## Ground truth (in priority order)

1. **The game binary** — `JustCause4.exe` (x64, DirectX 11). No DRM analysis needed for RE; disassemble
   directly. A full Ghidra decomp is the specification (see `docs/binary_recon.md` for how to generate it).
2. **The retail install** — `C:\Program Files (x86)\Steam\steamapps\common\Just Cause 4` (assets in
   `archives_win64/*.tab`+`.arc`). Read-only; never modify game files in place.
3. **JC2/JC3 Apex community prior art** — Gibbed's Just Cause tools, ApexToolsLauncher / RustyApex,
   and our own `jc2_arc`/`jc2_rbm` crates in the Mercs 2 repo. Cross-check, never trust blindly:
   JC4 is a **later** Apex revision than JC2.

## Layout

- `docs/` — the RE record (formats, recon, lineage). Evidence-graded (see `AGENTS.md`).
- `tools/` — small Rust crates, each round-trip-verified. Start: [`tools/jc4_arc`](tools/jc4_arc).
- `memory/` — durable cross-session notes, one fact per file; `memory/MEMORY.md` is the index.
  (Lives under `~/.claude/projects/<slug>/memory/`, outside the repo tree.)
- `.mcp.json` — wires the `corpus` RAG server (shared from the Mercs 2 repo, re-targeted here).

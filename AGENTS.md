# Working in this repository (humans and AI agents)

This is a reverse-engineering knowledge base for **Just Cause 4** (Avalanche, 2018), on the
**Apex engine**. Read this before making changes. It defines conventions and — importantly — how
NOT to over-import from the Mercenaries 2 and Saboteur work this effort descends from.

## Prime directive: verify lineage before you reuse

The two prior projects ([Mercenaries 2](../notes-on-the-released-game),
[The Saboteur](../notes-on-reversing-the-sabetour)) are **Pandemic-engine** games. JC4 is a
**completely different engine family — Avalanche's Apex** (shared with Just Cause 3, theHunter:
Call of the Wild, RAGE 2, Generation Zero). This means **less transfers than between Mercs 2 and
Saboteur, not more.** The Pandemic hash, `sges`/FFCS containers, Havok packfiles, UCFX/megapack —
none of it applies here.

Where JC4 *does* have relatives, it's **Just Cause 2/3** (same Apex archive lineage: TAB/ARC, SARC,
RenderBlockModel). Even there JC4 is a **later revision** — its TAB is v2, not JC2's v1; it adds
Oodle compression, ADF, AAF, and DDSC. Before stating that a JC2/JC3 fact applies, check
[`docs/lineage_and_divergence.md`](docs/lineage_and_divergence.md). If it isn't in the "shared"
column, treat it as **unverified** and confirm against the JC4 binary or assets. Never copy a JC2
offset, struct, or magic into a JC4 doc without re-deriving it.

## Ground truth, in priority order

1. **The game binary** — `JustCause4.exe` (x64, DX11, unpacked — no DRM to unwrap). A full Ghidra
   decomp is the oracle for engine behavior. It parses/produces every format; read it before guessing.
   (Generation is a headless-Ghidra task; see `docs/binary_recon.md`. The decomp is large/regenerable
   — gitignored, indexed by the corpus server, not committed.)
2. **The retail install** — `C:\Program Files (x86)\Steam\steamapps\common\Just Cause 4`
   (`archives_win64/*.tab`+`.arc`, shaders, FMOD banks). **Read-only.** Never modify game files in place.
3. **JC2/JC3 Apex community prior art** — Gibbed Just Cause tools, ApexToolsLauncher / RustyApex, plus
   our own `tools/jc2/jc2_arc` and `jc2_rbm` in the Mercs 2 repo. Cross-check, don't trust blindly.

## Conventions

- **Prefer Rust for tooling.** Small crates under `tools/`, dependency-light (hand-rolled byte readers;
  `flate2` for zlib, the game's own `oo2core_7_win64.dll` for Oodle). Every reader is validated by
  **round-trip byte-equality** before it's trusted. Document how to run it.
- **Novel format cracking → double-blind studies.** ≥2 independent investigators (isolated worktrees,
  blind to each other) derive the same layout; a third adjudicates divergences against a hard oracle
  (round-trip equality, cursor-consumes-file, offsets 0x1000-aligned & monotonic, count matches).
- **Verify artifacts by hash**, not size/mtime, when you produce or deploy anything.
- **Every format claim cites its evidence**: a decomp function VA, a byte offset in a named retail
  file, or a cross-reference to a named community tool's source. Grade it (see below).
- **Ask, don't assume** on unverified setup/intent facts. State the defaults you pick.
- **Minimal, faithful edits** to others' files; don't reword working prose.
- **Don't commit game assets or huge regenerated outputs** — they're gitignored. Commit the *method*
  to regenerate them, plus small reference dumps.

### Evidence grading (doc frontmatter)

Front every format/recon doc with YAML: `status: current|superseded`, `evidence: proven|inferred|
speculative`, `verified_on:`, `witness:` (the exact command + counts that prove the claim),
`supersedes:`. "proven" means a hard oracle passed — not "it looked right."

## What is deliberately NOT carried over from the prior projects

- The **Pandemic hash / `sges` / FFCS-WAD / megapack** machinery — wrong engine entirely.
- **Havok packfile** readers (Mercs 2 = 5.5, Saboteur = 6.5) — JC4's physics/animation stack is
  Apex-native; re-derive from scratch.
- **SecuROM devirtualization** — JC4 has no such DRM; disassemble directly.
- Any **byte offset, struct, chunk tag, or hash** from a Pandemic-engine doc.

Bring over *methodology* — double-blind cracking, the Ghidra-headless + x32dbg-MCP oracle loop,
hash-name recovery, evidence-graded notes, the Rust round-trip discipline, the corpus RAG backbone,
and the "ship mods as an additive override pack, never touch the base archive" law — not *facts*.

## Memory

`memory/` holds durable cross-session notes (one fact per file, YAML frontmatter, `[[wikilinks]]`);
`memory/MEMORY.md` is the one-line router index with standing mandates at the top. Add a note when you
establish something non-obvious and reusable; keep index lines short.

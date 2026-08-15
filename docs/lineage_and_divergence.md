---
status: current
evidence: mixed — see per-row grading
verified_on: 2026-08-15
witness: initial install survey (xxd of boot/game0.tab + boot/game0.arc; archives_win64 tree; shipped DLL inventory)
---

# Lineage & Divergence: Just Cause 4 vs the prior projects & vs JC2/Apex

**Read this before reusing anything.** JC4 runs Avalanche's **Apex engine** — a *different engine
family* from the Pandemic-engine games this effort descends from (Mercenaries 2, The Saboteur). The
correct relatives to mine are the other **Apex / RenderWare-Avalanche** titles: **Just Cause 2 & 3**,
theHunter: COTW, RAGE 2, Generation Zero. Anything not in a **SHARED** row must be re-verified against
the JC4 binary/assets before you rely on it.

## Two separate lineage questions

1. **vs the prior projects (Pandemic engine)** — essentially *nothing* transfers at the byte level.
   Different hash, compression, containers, model/anim/texture formats, physics stack. Only
   **methodology** carries over (see `AGENTS.md`).
2. **vs Just Cause 2/3 (same Apex family)** — the *concepts* transfer (TAB/ARC archives, SARC bundles,
   RenderBlockModel, name-hash addressing), but JC4 is a **later revision**: TAB v2 not v1, adds Oodle,
   ADF, AAF, DDSC. Re-derive every offset.

## SHARED with JC2/Apex — concept carries, VERIFY the layout

| Thing | JC2 (prior art) | JC4 (this game) | Evidence / status |
|---|---|---|---|
| **Archive pair** | `.tab` TOC + `.arc` blob | `.tab` + `.arc` (159 pairs, ~70 GB) | ✅ **proven** — same scheme; layout differs (below) |
| **Sub-bundle** | `SARC` small-archive carrying real names | expected inside `.arc` entries | ⚠️ inferred from lineage — confirm in JC4 |
| **Model** | `RBMDL` RenderBlockModel → glTF | RBM/RBN render-block model (newer block set) | ⚠️ inferred — block IDs differ, re-derive |
| **Name-hash addressing** | 32-bit name hash keys | hash-keyed (width/algorithm unconfirmed) | ⚠️ **unverified** — do NOT assume JC2's hash |

## DIFFERENT / NEW in JC4 — treat as from-scratch

| Thing | JC2 | JC4 | Impact |
|---|---|---|---|
| **TAB version** | v1: 16-byte header, 12-byte entries `{nameHash,offset,size}`, 2048-align | **v2**: magic `TAB\0`, version `2.1`, alignment `0x1000`. Entry stride does NOT match JC2's 12 bytes (interleaved `0xFFFFFFFF` fields observed) | ✅ header **proven**; ⚠️ entry layout **UNVERIFIED** — first crack target (`tools/jc4_arc`) |
| **Compression** | zlib/deflate (per-file `0x78` streams) | **Oodle Kraken** (`oo2core_7_win64.dll` shipped) | New. Link the game's own DLL (can't redistribute); `flate2` won't decode arc payloads |
| **Structured data** | Avalanche property container `01 04 00 01` | **ADF** (Avalanche Data Format, magic `ADF ` / ` FDA` LE) — first entry of `boot/game0.arc` is `AICoverSettings.adf` | ✅ presence **proven**; format to crack |
| **Container** | SARC | **AAF** and/or SARC nesting inside ADF/arc | ⚠️ inferred — confirm |
| **Textures** | `DDS ` inline | **DDSC / HMDDSC** (streamed-mip Avalanche texture) | New; expect header + external hi-res mip stream |
| **Audio** | FSB / FEV (FMOD) | **FMOD Studio** (`fmod_studio_F.dll`) — `.bank` | Newer FMOD; re-derive |
| **Address width** | 32-bit throughout | **64-bit** engine — offsets/hashes/pointers widen | Every reader/writer field width re-derived; the user's "larger address buffers" point |
| **DRM** | — | none relevant to RE (unpacked x64 exe) | Skip the whole SecuROM playbook |

## Confirmed JC4 header facts (from the install survey)

- **TAB v2 header** (`archives_win64/boot/game0.tab`, offset 0): `54 41 42 00` = `TAB\0`,
  `02 00` `01 00` = version 2.1, `00 10 00 00` = alignment `0x1000`, then `00 00 00 00`.
  Entries begin at offset `0x10`; their exact struct is the first double-blind task.
- **ARC payload**: `boot/game0.arc` offset 0 is an ADF file (` FDA` = `ADF ` LE), named
  `AICoverSettings.adf` — so the arc stores name-prefixed, typed records; ADF is the structured-data
  workhorse.
- **Oodle**: `oo2core_7_win64.dll` present ⇒ arc entry payloads are Oodle-compressed, not zlib.

## Rule of thumb

- From the **prior Pandemic projects**: reuse *methodology only* — never a byte.
- From **JC2/JC3 Apex**: reuse *concepts and tool shapes* (`jc2_arc`, `jc2_rbm`), re-derive *every layout*.
- When in doubt, the **clean JC4 decomp is the oracle** — the exe is unpacked; the code that parses the
  format is right there to read.

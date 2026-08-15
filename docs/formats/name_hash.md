---
status: current
evidence: proven
verified_on: 2026-08-15
witness: reproduces all 12,130 (string→hash) pairs from ADF string-hash tables + instance names; and 225 real resource-path strings match TAB entry name_hashes across boot+main archives; algorithm independently confirmed from the decomp as lookup3 hashlittle
---

# JC4 name hash — Bob Jenkins lookup3 `hashlittle` (seed 0)

The single 32-bit string hash used everywhere in JC4: **Bob Jenkins lookup3 `hashlittle`, `initval = 0`,
hashing the raw bytes as-is (no lowercasing applied by the hash, no trailing NUL)**. This is the
canonical `lookup3.c` `hashlittle()` verbatim.

Proven two ways, both hard oracles:
- **ADF**: reproduces **all 12,130** `(string → u32)` pairs harvested from ADF string-hash tables and
  instance `name_hash`↔name — e.g. `hashlittle("BloomContrast") = 0x2f6ea6e9`. (12,130 exact 32-bit
  matches ⇒ not a coincidence.)
- **TAB/ARC**: **225 real resource-path strings** (recovered from extracted asset payloads) match TAB
  entry `name_hash`es across the boot + main archives — e.g.
  `hashlittle("environment/presets/aerial.environc") = 0x0ec6df48`,
  `hashlittle("settings/spawn_prop_defs.bin")`, `hashlittle("textures/roads/jc4/asphalt/asphalt_edge_03_mpm.ddsc")`.
- **Decomp**: independently identified as standard lookup3 `hashlittle` (seed 0). ⚠️ No clean VA to
  cite — the hash is **compiled inline** at its call sites and this build's decomp is obfuscated around
  it (broken jumptables / register artifacts). The algorithm ID is authoritative from the ground-truth
  match; the inlined VA is not recoverable from the static dump (use x64dbg on the ADF name-resolution
  path if a VA is ever needed). ADF anchors nearby: `FUN_14aafceb0` (header), `FUN_140f2c220` (name walker).

## What gets hashed

| Use | Input string | Example |
|---|---|---|
| **TAB entry `name_hash`** | full resource **path**: lowercase, forward slashes, **with extension**, no leading/trailing slash, no prefix | `environment/presets/aerial.environc` → `0x0ec6df48` |
| **ADF string-hash / type / name** | the string as-is (property names, type names, etc.) | `BloomContrast` → `0x2f6ea6e9` |

The retail paths are already stored lowercase with forward slashes, so the hash is applied directly —
no separate normalization step was needed to reproduce them. (Whether the engine *also* lowercases at
runtime is moot for reproduction; feed it the lowercase forward-slash path.)

## Why this matters (the modding payoff)

This is the gate to **registering brand-new assets**, not just overriding existing ones:
- To **override** an existing asset in an additive patch archive, reuse its known `name_hash`.
- To **add a new** asset, compute `name_hash = hashlittle("your/resource/path.ext")` — now the engine
  can find it by path. Combined with the proven TAB/ARC writer format (future), this enables real
  additive-override mods.

## Reference implementation

`tools/jc4_arc hash <string>` prints `hashlittle(string, 0)`. Rust impl in
`tools/jc4_arc/src/main.rs` (`mod hashlittle`). Standard lookup3: `a=b=c=0xdeadbeef+len`; 12-byte
mix rounds (rotates 4/6/8/16/19/4); final rounds (rotates 14/11/25/16/4/14/24); return `c`.

## Filelist (external dictionary) — validated at scale

gibbed's `Gibbed.JustCause4` ships per-archive **filelists** (plaintext, lowercase, forward-slash resource
paths, using *cooked* extensions — e.g. `ai/aicoversettings.aicoversettingsc`, `*.environc`, `*.bin`).
Running our `hashlittle(0)` over them (`jc4_arc names <tab> <filelist>`): **every one of the 1,311 boot
paths + 850 main/game1 paths maps to a real TAB entry hash, 0 collisions, 0 mismatches** — matching
gibbed's own stated coverage (boot 89%). This is a second, external, 2,161-pair confirmation of the hash
(on top of the 12,130 ADF pairs) and gives us a ready un-hashing dictionary. Fetch from
`github.com/gibbed/Gibbed.JustCause4/.../files/archives_win64/**/game*.filelist` (+ `jc4-mp/jc4-file-lists`).
Coverage is partial (~72–89% per archive); the remainder still needs path recovery.

## Tool
`jc4_arc hash <string>` (compute) and `jc4_arc names <tab> <filelist>` (un-hash a .tab via a filelist).

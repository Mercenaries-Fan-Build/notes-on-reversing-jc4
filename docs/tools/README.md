# Tools — task index

A small cargo **workspace** under [`../../tools/`](../../tools): the shared codec library
[`jc4_formats`](../../tools/jc4_formats) (TAB/ARC+Oodle, ADF, name hash) plus thin CLIs over it — and
(soon) `jc4_workshop`, the wgpu+egui model/texture inspector. Mirrors the `mercs2_formats`/`sab_formats`
pattern. Every format claim ships with a passing oracle.

| I want to… | Tool | Status |
|---|---|---|
| the shared format library | [`jc4_formats`](../../tools/jc4_formats) | ✅ `tab` / `adf` / `hash` / `oodle` modules |
| list / verify a `.tab`+`.arc` archive | [`jc4_arc`](../../tools/jc4_arc) | ✅ `header`/`verify`/`list`/`hex` — TAB v2 proven on all 159 tabs |
| extract + decompress arc payloads (raw/Oodle) | [`jc4_arc`](../../tools/jc4_arc) `extract` | ✅ 0 failures on 4477 entries; runtime-binds `oo2core_7_win64.dll` |
| un-hash entries via a filelist | [`jc4_arc`](../../tools/jc4_arc) `names` | ✅ `hash` + `names` (lookup3 hashlittle) |
| decode ADF structured data → JSON | [`jc4_adf`](../../tools/jc4_adf) | ✅ `info`/`dump`/`verify` — 90/90 corpus decode |
| AVTX texture → DDS / render bytes | [`jc4_tex`](../../tools/jc4_tex) + `jc4_formats::avtx` | ✅ `info`/`dds`/`verify` — 1193/1193 |
| inspect a textured model | *(planned, next)* `jc4_workshop` | wgpu+egui viewer; AVTX ready, needs RBM/RBN |
| RBM/RBN model → mesh | *(planned)* in `jc4_formats` | fork pattern from `jc2_rbm` |
| write an additive override pack | *(planned)* | after the override-mount layer is reversed |

## Prior-art crates to fork (in the Mercs 2 repo)

- `../../../notes-on-the-released-game/tools/jc2/jc2_arc` — JC2 ARC/TAB reader; the direct ancestor of
  `jc4_arc` (JC4 is TAB **v2**, so header + entry decode diverge).
- `../../../notes-on-the-released-game/tools/jc2/jc2_rbm` — JC2 RenderBlockModel → glTF, with a
  heuristic vertex-stride scanner; the pattern for `jc4_rbm`.

## Conventions

Build the whole workspace: `cargo build --release` from `tools/` (binaries land in `tools/target/release/`).
`cargo test` runs the unit oracles (e.g. the hashlittle known-pairs test). Keep it dependency-light
(hand-rolled byte readers; `flate2` for zlib; the game's `oo2core_7_win64.dll` for Oodle; `serde_json`
for ADF output). No format claim ships without a passing round-trip / consumes-file / alignment oracle.

# Tools — task index

"I want to…" → which tool. Every tool is a small Rust crate under [`../../tools/`](../../tools),
validated by round-trip byte-equality.

| I want to… | Tool | Status |
|---|---|---|
| list / probe a `.tab`+`.arc` archive | [`jc4_arc`](../../tools/jc4_arc) | WIP — TAB v2 header parse + entry-stride cracking probe |
| decompress an arc entry (Oodle) | `jc4_arc` (planned) | needs `oo2core_7_win64.dll` binding |
| decode ADF structured data | *(planned)* `jc4_adf` | — |
| RBM/RBN model → glTF | *(planned)* `jc4_rbm` | fork from Mercs 2 repo's `tools/jc2/jc2_rbm` |
| DDSC texture ⇄ DDS | *(planned)* `jc4_dtex` | — |
| write an additive override pack | *(planned)* | after the mount/override layer is reversed |

## Prior-art crates to fork (in the Mercs 2 repo)

- `../../../notes-on-the-released-game/tools/jc2/jc2_arc` — JC2 ARC/TAB reader; the direct ancestor of
  `jc4_arc` (JC4 is TAB **v2**, so header + entry decode diverge).
- `../../../notes-on-the-released-game/tools/jc2/jc2_rbm` — JC2 RenderBlockModel → glTF, with a
  heuristic vertex-stride scanner; the pattern for `jc4_rbm`.

## Conventions

Build: `cargo build --release --manifest-path tools/<crate>/Cargo.toml`. Keep crates dependency-light
(hand-rolled byte readers; `flate2` for zlib; the game's `oo2core_7_win64.dll` for Oodle). No format
claim ships without a passing round-trip / consumes-file / alignment oracle.

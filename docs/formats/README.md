# JC4 format specs

Byte-level format documentation, each doc evidence-graded (see [`../../AGENTS.md`](../../AGENTS.md)).
Crack order and status track [`../binary_recon.md`](../binary_recon.md).

| Format | What | Status |
|---|---|---|
| TAB/ARC v2 | archive TOC + blob | ✅ **PROVEN** — [`tab_arc_v2.md`](tab_arc_v2.md); verified on all 159 tabs (`../../tools/jc4_arc`) |
| Oodle | arc payload compression (`oo2core_7_win64.dll`) | ✅ integrated in `jc4_arc extract` |
| name hash | 32-bit string hash (TAB paths + ADF names) | ✅ **PROVEN** — [`name_hash.md`](name_hash.md); lookup3 `hashlittle(0)`, 12130+225 pairs |
| ADF | Avalanche Data Format — structured data | ✅ **PROVEN** — [`adf.md`](adf.md); `jc4_adf` decodes 90/90 to JSON |
| AAF / SARC | container nesting | to confirm |
| RBM / RBN | RenderBlockModel meshes → glTF | to crack (JC2 `rbm` is the ancestor pattern) |
| DDSC / HMDDSC | streamed-mip textures → DDS | to crack |
| FMOD `.bank` | audio | to crack |

Add one file per format as it's cracked (`tab_arc_v2.md`, `adf.md`, …). Each must cite a decomp VA, a
byte offset in a named retail file, or a named community-tool cross-reference, and pass a hard oracle.

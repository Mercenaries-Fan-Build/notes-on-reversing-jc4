---
status: current
evidence: proven
verified_on: 2026-08-15
witness: double-blind crack (2 independent investigators converged) + jc4_formats::avtx verify PASS=1193/1193 (header sanity + inline-region == Σ mip sizes); decomp writer FUN_14aa1bfd0, magic FUN_14aa1c0a0
---

# JC4 AVTX texture

AVTX ("Avalanche texture") is the JC4 texture container — a 0x80-byte header followed by inline mip
data. It carries a raw **DXGI format**, so payloads are standard BCn / uncompressed surfaces (a GPU can
consume them directly). Larger textures keep their **top mips in an external hi-res stream** (`.hmddsc`
/ a `hires` archive); the AVTX holds the smaller mips. Cracked double-blind (both investigators
converged, 1193/1193). Decomp: writer `FUN_14aa1bfd0`, magic check `FUN_14aa1c0a0` (`== 0x58545641`).

## Header (0x80 bytes, little-endian)

| Off | Type | Field | Notes |
|---|---|---|---|
| 0x00 | char[4] | magic `AVTX` (`0x58545641`) | |
| 0x04 | u16 | version | 1 |
| 0x06 | u8 | unknown | varies 0–59; not size-critical (open) |
| 0x07 | u8 | dimension | raw D3D: 2=Texture2D/array, 3=Texture3D |
| 0x08 | u32 | **dxgi_format** | see census |
| 0x0C | u16 | **width** (base mip) | |
| 0x0E | u16 | **height** (base mip) | |
| 0x10 | u16 | **depth** / array-slice count | multiplies data size |
| 0x12 | u16 | **flags** | `0x40`=cubemap (×6), `0x01`=has external mips, `0x08`=? (open) |
| 0x14 | u8 | **mip_total** | full mip-chain length |
| 0x15 | u8 | **mip_inline** | mips present in THIS file (`< total` ⇒ largest mips external) |
| 0x20 | u32 | size_header = 0x80 | data start |
| 0x24 | u32 | size_body | inline byte count = `filelen − 0x80` (all 1193) |
| 0x28 | u32 | = 0x10 | constant; alignment granularity? (open) |
| 0x2C–0x7F | | zero pad | writer memsets 0x20→0x80 |

**No per-mip descriptor array** — mip sizes are computed from base dims + format + level.

## DXGI format census (boot corpus, 1193 files)

| DXGI | Name | block | count | decode |
|---|---|---|---|---|
| 77 (0x4D) | BC3_UNORM | 4×4, 16 B | 819 | BCn |
| 71 (0x47) | BC1_UNORM | 4×4, 8 B | 214 | BCn |
| 80 (0x50) | BC4_UNORM | 4×4, 8 B | 64 | BCn |
| 87 (0x57) | B8G8R8A8_UNORM | 1×1, 4 B | 45 | raw |
| 98 (0x62) | BC7_UNORM | 4×4, 16 B | 25 | BCn |
| 83 (0x53) | BC5_UNORM | 4×4, 16 B | 13 | BCn |
| 10 | R16G16B16A16_FLOAT | 1×1, 8 B | 6 | raw |
| 26 | R11G11B10_FLOAT | 1×1, 4 B | 3 | raw |
| 74 (0x4A) | BC2_UNORM | 4×4, 16 B | 3 | BCn |
| 28 (0x1C) | R8G8B8A8_UNORM | 1×1, 4 B | 1 | raw |

Block size: BCn → `ceil(w/4)·ceil(h/4)·blockBytes`; uncompressed → `w·h·bpp`.

## Mip layout

- Full texture has `mip_total` mips (mip 0 = `width×height`). The file stores the smallest `mip_inline`
  mips, i.e. levels `[start .. mip_total)` where `start = mip_total − mip_inline`, **largest-inline
  first** from 0x80 (per slice/face).
- `mip_inline < mip_total` (flag `0x01`) ⇒ the top `start` mips are external (`.hmddsc`/hires archive).
  In the boot corpus: **7 files** (all 1024×1024, `total=11`, inline 8–9).
- Cubemaps (`0x40`): every mip stored ×6 faces. Arrays/volumes (`depth>1`): ×depth. (True 3D volume
  depth-halving per mip is untested — none multi-mip in corpus.)
- **Inline byte total** = `slices · Σ_{i=start}^{total−1} mip_size(w>>i, h>>i, fmt)` == `size_body` ==
  `filelen − 0x80` on **all 1193**.

## Best available mip (for rendering, from the AVTX alone)

`start = mip_total − mip_inline`; `w = max(1, width>>start)`, `h = max(1, height>>start)`; the largest
inline mip (slice/face 0) is the first `mip_size(w,h,fmt)` bytes at offset 0x80. When mips are external,
this is the best obtainable without the hi-res stream. ⚠️ Ordering (largest-inline-first) is asserted
from convention + size-consistency (sizes are order-invariant); **the workshop's first render is the
visual proof** — a coherent image confirms it.

## Tool
`tools/jc4_tex` — `info` / `dds <avtx> <out.dds>` (DX10-wrapped, opens in any DDS viewer) / `verify <dir>`.
Library: `jc4_formats::avtx` (`parse_header`, `best_inline_mip → {width,height,dxgi_format,data}`,
`to_dds`, `expected_inline_size`). The BCn bytes + DXGI format upload directly to a wgpu compressed
texture — no CPU BCn decode needed for the renderer.

## Open
- `unknown@0x06`, `flags 0x08`, `field@0x28` semantics.
- Inline mip ordering (visual confirm pending). External `.hmddsc` payload layout (no samples in corpus).

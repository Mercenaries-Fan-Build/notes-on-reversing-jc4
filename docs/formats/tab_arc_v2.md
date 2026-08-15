---
status: current
evidence: proven
verified_on: 2026-08-15
witness: double-blind crack (2 independent investigators converged) + jc4_arc verifier exact-consume/aligned/monotonic/in-bounds on ALL 159 retail .tab files (PASS=159 FAIL=0); decomp parser FUN_140f92be0
supersedes: the "entry table UNVERIFIED" note in lineage_and_divergence.md
---

# JC4 TAB v2 (`.tab` + `.arc`) archive format

The retail archive is 159 `.tab`+`.arc` pairs under `archives_win64/`. `.tab` is the table of contents
(header + compression-block table + entry table); `.arc` is the packed blob entries point into. Cracked
double-blind on 2026-08-15 — two investigators independently derived an identical layout; `tools/jc4_arc`
then verified it exact-consumes **all 159** `.tab` files.

**Decomp oracle:** v2 parser `FUN_140f92be0 @ 0x140f92be0`; entries are hash-sorted for binary search
(`FUN_140f92890`); codec dispatch `FUN_14ad55350 @ 0x14ad55350`. (A legacy JC2-style v1 path
— 24-byte header + 12-byte entries — also exists in the binary at `FUN_140f90…` but is a different,
unused-by-retail format.)

## Header — 24 bytes

| Off | Type | Field | Notes |
|---|---|---|---|
| 0x00 | char[4] | magic | `54 41 42 00` = `"TAB\0"` |
| 0x04 | u16 | version_major | `2` (parser checks `==2`) |
| 0x06 | u16 | version_minor | `1` (parser checks `==1`) |
| 0x08 | u32 | alignment | `0x1000` — all entry offsets are multiples of this |
| 0x0C | u32 | reserved | `0` in every sample |
| 0x10 | u32 | max_compressed_block_size | scratch/decompress-buffer hint; `0` when the arc has no real blocks. ⚠️ exact use not isolated in decomp (open) |
| 0x14 | u32 | block_uncompressed_size | `0x80000` (512 KB) — the streaming block size; `0` when no real blocks |

## Compression-block table

| Off | Type | Field |
|---|---|---|
| 0x18 | u32 | block_count |
| 0x1C | Block[block_count] | 8 bytes each: `{ u32 compressed_size; u32 uncompressed_size }` |

- `(0xFFFFFFFF, 0xFFFFFFFF)` records are **sentinels/separators** (index 0 is always a sentinel; e.g.
  boot has 229 sentinel gaps between multi-block entries' ranges). Uncompressed archives carry exactly
  two sentinels and no real blocks (`block_count == 2`, `block_uncompressed_size == 0`).
- **Proven:** for a 44.7 MB multi-block entry, the 86 blocks it spans sum *exactly* to its compressed
  (`0x66e85e`) and uncompressed (`0x2aaac80`) sizes; each full block is `0x80000` uncompressed. All 243
  multi-block entries in `boot` are individually consistent.
- These sentinel `0xFFFFFFFF` records are what broke the naïve 12-byte-stride guess — they sit *before*
  the entry table, not inside it.

## Entry table — 20 bytes/entry, to EOF

Starts at `0x1C + block_count*8`. Count = `(file_size − entries_off) / 20` (remainder is always 0).
Entries are **sorted ascending by name_hash** (for binary search) — a writer MUST preserve this.

| Off | Type | Field | Notes |
|---|---|---|---|
| +0x00 | u32 | name_hash | 32-bit name hash; sort key. ⚠️ hash algorithm not yet derived (open) |
| +0x04 | u32 | offset | physical byte offset into `.arc`; multiple of `alignment`; monotonic non-decreasing |
| +0x08 | u32 | compressed_size | bytes stored in `.arc` at `offset` |
| +0x0C | u32 | uncompressed_size | size after decompression; `== compressed_size` ⇒ stored raw |
| +0x10 | u32 | flags | packed bitfield (below) |

### `flags` bitfield
- `bits[15:0]` = **first_block_index** into the compression-block table (multi-block entries).
- `bits[23:16]` = **codec_id**.
- `bit[24]` = **multi_block** (payload spans the shared 512 KB block stream).

Proof: single-block Oodle `flags=0x00040000` (codec 4, blk 0, single); a 44.7 MB entry `0x01040001`
(codec 4, multi, first_block 1); the next such `0x01040057` → first_block 87 = 1 + 86 (cumulative).
Corroborated by the big-endian fixup loop, which byte-swaps entry `u32[0..3]` but deliberately leaves
`u32[4]` (flags) untouched — consistent with a packed bitfield, not a plain int.

## Codec dispatch (`FUN_14ad55350`)

| codec_id | codec | evidence |
|---|---|---|
| 0 | none (raw memcpy) | `param==0` → copy |
| 1 | zlib/deflate | inits with version string `"1.2.8"` |
| 2 | lz4-style | single-shot decompress thunk |
| 3 | zstd | ZSTD ctx create/decompress chain |
| 4 | **Oodle** | `OodleLZ_Decompress` (`oo2core_7`), decomp line 4966763 |

Retail arcs observed use only **0 (raw)** and **4 (Oodle)**. Payload extraction therefore needs the
game's `oo2core_7_win64.dll` for codec 4 (can't redistribute — link the game's copy).

## Community cross-reference (audit, 2026-08-15)

Matches gibbed's `Gibbed.JustCause4/ArchiveTableFile.cs` **bit-for-bit** (the authoritative reference)
and DECA's `ff_arc_tab.py`. Field-name aliases so this doc is self-contained:
- Our packed `flags` = gibbed's separate `{CompressedBlockIndex u16, CompressionType u8, CompressionFlags u8}`
  (LE, so identical bits): our `first_block_index` = `CompressedBlockIndex`, `codec_id` = `CompressionType`
  (enum `None=0, ZLib=1, Oodle=4`), `multi_block` bit = `CompressionFlags` bit 0 (gibbed labels it
  `Unknown` and, like us, does **not** use it for the decode decision — our `multi_block` name is
  **inferred, not proven**; don't wire logic to it).
- Our `0xFFFFFFFF,0xFFFFFFFF` sentinel = DECA's **`no_block`**. This also *explains* our
  `first_block==0 ⇒ single stream` rule mechanistically: block index 0 is always a `no_block`, so
  "index 0" ⟺ "points at a sentinel" ⟺ single continuous stream (gibbed keys on `index==0`, DECA on
  `block != no_block` — same decision).
- ⚠️ EonZeNx's `tab_v02.hexpat` models the **legacy JC3** TAB (12-byte header, 12-byte uncompressed
  entries, no block table) and misparses real JC4 v2.1 — do not use it as the JC4 spec.

## Oracle results

`tools/jc4_arc verify` on every retail `.tab`: **PASS = 159 / FAIL = 0** (exact-consume + 100%
aligned + monotonic + in-bounds where the `.arc` was checked). Representative:

| File | size | block_count | entries | remainder |
|---|---|---|---|---|
| `boot/hires/game0.tab` | 544 | 2 (sentinels) | 25 | 0 |
| `boot/game0.tab` | 39920 | 1294 | 1477 | 0 |
| `main/eng/game0.tab` | 232784 | 2 (sentinels) | 11637 | 0 |

Sanity: `boot/game0.arc` entry 0 (`offset=0, comp=uncomp=0x51d, codec 0`) is raw `ADF ` (` FDA` LE) —
`AICoverSettings.adf`. `boot/hires/game0.tab` is a degenerate **hash-only manifest** (25 hashes, all
offset/size 0; no `.arc` in `hires/`) — an override/patch list.

## Payload extraction / decompression (PROVEN)

An entry's arc payload decodes by one of three modes; the discriminator is `comp_size==uncomp_size` then
`first_block_index`. Established empirically (ctypes probe against `oo2core_7`) and confirmed by extracting
**all 1477 `boot` entries + 3000 `main/eng` entries with 0 failures**, each decompressing to exactly its
declared `uncompressed_size`:

1. **`comp_size == uncomp_size` ⇒ stored raw** — copy `comp_size` bytes (ignore the codec bits; some
   stored entries still carry `codec_id=4` in flags).
2. **`first_block_index == 0` ⇒ one continuous stream** — `OodleLZ_Decompress(comp[..comp_size]) → uncomp_size`
   in a single call. Oodle's internal seek-chunks are handled for you.
3. **`first_block_index != 0` ⇒ per-block** — the payload is a run of **independently** Oodle-compressed
   blocks. From `first_block_index`, decode `block[i]: comp_size → uncomp_size` sequentially (advancing the
   arc read position by each block's `comp_size`), skipping sentinel blocks, until the entry's
   `uncomp_size` is reached. Individual blocks may be near-incompressible (`comp ≳ uncomp`) but are still
   valid Oodle streams — this is *why* single-shot fails on them and per-block is required.

`OodleLZ_Decompress` params used: `fuzzSafe=1, checkCRC=0, verbosity=0, threadPhase=3`, rest NULL/0. The
DLL is loaded at runtime from the game dir (`oo2core_7_win64.dll`) — not redistributed.

Observed `boot` content types after extraction: **AVTX** textures (1193), **ADF** (90), **RTPC** runtime
property containers (80), plus SARC/WAV/DDS. (AVTX/RTPC are the next formats after ADF.)

## Open questions (do not block a reader)
1. ~~name_hash algorithm~~ ✅ **SOLVED** — `name_hash = hashlittle(lowercase forward-slash resource path)`;
   lookup3 seed 0. See [`name_hash.md`](name_hash.md). Registering a new asset = hash its path.
2. Exact use of header `@0x10` (`max_compressed_block_size`) — inferred as a scratch/buffer hint.
3. The sentinel-allocation rule (why N sentinel gaps) — doesn't affect ToC parsing.

## Tool
`tools/jc4_arc` — `header` / `verify` / `list` / `hex` / **`extract <tab> <arc> <outdir> [limit]`**
(decodes raw + Oodle payloads to `outdir`, named `{name_hash:08x}.{ext}` with ext by magic sniff).
Oodle DLL path defaults to the game dir; override via `$JC4_OODLE_DLL`.

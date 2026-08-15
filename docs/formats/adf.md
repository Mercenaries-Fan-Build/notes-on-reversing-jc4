---
status: current
evidence: proven
verified_on: 2026-08-15
witness: double-blind crack (2 independent investigators converged, incl. byte-identical hand-decode of the smallest file) + jc4_adf Rust decoder PASS=90/90 corpus files (total_size==filelen, all tables consume exactly, every type_hash resolves, sane scalars); decomp parser FUN_14aafceb0
---

# JC4 ADF (Avalanche Data Format)

ADF is Avalanche's **reflection-based typed binary container** — the backbone data format: most JC4
assets are ADF (world/terrain settings, environment params, entity configs, …). A file embeds the
reflected **type library** it needs, so it is self-describing. Cracked double-blind on 2026-08-15;
`tools/jc4_adf` decodes **90/90** corpus files to typed JSON, reproducing the investigators' values
exactly. All integers little-endian (the loader also accepts a byte-swapped BE variant, magic
`0x20464441`).

**Decomp oracle:** header parse/validate `FUN_14aafceb0`; instance-table stride `FUN_14aaf6c70`
(0x18 v4 / 0x30 v<4); core decoder + relocation `FUN_140f2e0d0`; metatype dispatch `FUN_140f2c440`;
name-table walker `FUN_140f2c220`/`FUN_14aaf60b0`. Versions 2/3/4 supported; retail is v4.

## Header (64 bytes, then a NUL-terminated comment)

| Off | Type | Field |
|---|---|---|
| 0x00 | u32 | magic `0x41444620` (` FDA`) |
| 0x04 | u32 | version (4) |
| 0x08 | u32 | instance_count |
| 0x0C | u32 | instance_offset |
| 0x10 | u32 | typedef_count |
| 0x14 | u32 | typedef_offset |
| 0x18 | u32 | stringhash_count |
| 0x1C | u32 | stringhash_offset |
| 0x20 | u32 | name_count |
| 0x24 | u32 | name_offset |
| 0x28 | u32 | **total_size == file length** (oracle) |
| 0x2C | u32 | metadata_offset (0; EonZeNx name) |
| 0x30 | u32 | header_flags (=1; gates load-time pointer/relocation fixup in `FUN_140f2e0d0`) |
| 0x34 | u32 | included_libraries (0; EonZeNx name) |
| 0x38–0x3C | u32×2 | reserved (0) |
| 0x40 | cstr | comment (provenance, e.g. `"WorldSettingsType.adf in TerrainSystemTypes 12.0.0"`) |

Tables may appear in any order; only the offsets are authoritative.

## Type system

**MetaType** (first u32 of each typedef): `0`=Primitive/Scalar · `1`=Structure · `2`=Pointer ·
`3`=Array · `4`=InlineArray · `5`=String · `6`=Recursive · `7`=BitField · `8`=Enumeration ·
`9`=StringHash · `10`=Deferred. (6/10 unobserved in corpus.)

**Type-def record — 40-byte (0x28) header:**

| Off | Type | Field |
|---|---|---|
| 0x00 | u32 | MetaType |
| 0x04 | u32 | Size (byte size of an instance of this type) |
| 0x08 | u32 | Alignment |
| 0x0C | u32 | TypeHash (this type's id) |
| 0x10 | u64 | NameIndex |
| 0x18 | u16 | Flags |
| 0x1A | u16 | ScalarType (0=signed, 1=unsigned, 2=float) |
| 0x1C | u32 | SubTypeHash (element type for Array/InlineArray/Pointer) |
| 0x20 | u32 | ElementLength (InlineArray count; BitField bit-width) |
| 0x24 | u32 | MemberCount |

Tail: Structure → `MemberCount` × 32-byte members; Enumeration → `MemberCount` × 12-byte
`{NameIndex:u64, Value:i32}`; others → no tail.

**Struct member — 32 bytes:** `NameIndex:u64 @0x00`, `TypeHash:u32 @0x08`, `Alignment:u32 @0x0C`,
`Offset:u32 @0x10`, flag byte `@0x14`, `Default:u64 @0x18`. ★**Offset is packed**:
`byte_offset = Offset & 0xFFFFFF`, `bit_position = Offset >> 24` (bitfield members share a byte;
both investigators found this via `VegetationPhysics` — 7 `uint8:1` at byte 0x50, bits 0–6).

**Built-in scalar type_hashes** (referenced by hash, NOT stored as typedefs):
int8 `0x580D0A62` · uint8 `0x0CA2821D` · int16 `0xD13FCF93` · uint16 `0x86D152BD` ·
int32 `0x192FE633` · uint32 `0x075E4E4F` · int64 `0xAF41354F` · uint64 `0xA139E01F` ·
float `0x7515A207` · double `0xC609F663` · String `0x8955583E` (8-byte inline offset ref).
0 unresolved hashes across all 90 corpus files.

## Instance / name / string-hash tables

- **Instance record — 24 bytes (v4 stride 0x18):** `name_hash:u32, type_hash:u32, payload_offset:u32,
  payload_size:u32, name_index:u64`. `payload_size` includes the inline struct **plus** the appended
  array/string heap.
- **Name table:** `name_count` length bytes (u8 each), then that many NUL-terminated strings.
- **String-hash table:** `count` × `{cstring, u32 hash, u32 pad(=0)}`. Maps a 32-bit hash → source
  string; StringHash-typed (meta 9) fields store only the u32 hash and resolve here.

## Instance decoding

LE throughout. Decode `type_hash` at `payload_offset`; **all Array/String/Pointer offsets are relative
to the root instance's `payload_offset`** (`base`) — nested arrays resolve against the root base, not
the parent element.

- **Scalar/builtin:** read fixed width per ScalarType.
- **Structure:** each member at `base_off + (member.Offset & 0xFFFFFF)`; pass `Offset >> 24` as bit pos.
- **InlineArray:** `ElementLength` elements of SubType, contiguous (stride = element Size).
- **Array (dynamic):** 16-byte inline ref `{offset:u32 @+0, reloc-link:u32 @+4, count:u32 @+8, _}`;
  `count` elements of SubType at `base + offset`.
- **String / String-builtin:** 8-byte inline `{offset:u32, reloc-link:u32}`; NUL string at
  `base + offset` (0 ⇒ empty).
- **Pointer:** u32 offset → one SubType at `base + offset` (0 ⇒ null).
- **BitField:** read the Size-byte cell, value = `(cell >> bit) & ((1 << ElementLength) - 1)`.
- **Enumeration:** i32 → matched entry name. **StringHash:** u32 → resolve via string-hash table.

**Relocation:** array/string/pointer inline refs are 8 bytes = `{low u32 = offset (base-relative), high
u32 = intrusive relocation-linked-list link}`; at load the engine rewrites the low dwords into absolute
pointers (header flag @0x30). A type-directed reader needs only the low dword + count.

## Oracle results

`tools/jc4_adf verify` over the 90-file boot corpus: **PASS = 90 / FAIL = 0** — `total_size==filelen`,
all tables consume exactly, root instance decodes with every type_hash resolving, no overrun. 8 files
carry 1–7 bytes of benign trailing alignment padding. Smallest file `9fdec90b.adf` (WorldSettings)
decodes to `WorldSize=[32768,4096,32768]`, `PatchBaseLod=9`, `PatchLodRange=[9,12]`,
`StaticPatchMemoryRequirements=512`. Env files decode strings/floats/string-hashes
(`gust-demo`→`WeatherGustMapStrengthMax`=0.954/20.0, etc.).

## Community cross-reference (audit, 2026-08-15)

Field-for-field correct vs DECA `ff_adf.py`, gibbed JC3 `AdfFile.cs`, and EonZeNx `adf_v04.hexpat` on
every construct in the JC4 corpus — and *more complete than DECA* on Pointer/Array/BitField/Enum/String
(DECA prints "Not Implemented" on some of those). Built-in scalar hashes byte-identical to DECA/gibbed.
The **Deferred** type (metatype 10 / builtin `0xDEFE88ED`, `{offset, flags, type_hash, _}` → recurse) is
now handled to match DECA, though it's unobserved in the 90-file JC4 corpus. An ADF **writer** blueprint
(from gibbed's encoder): positional back-patching (no relocation table — write refs as `{offset, 0}`,
engine builds the intrusive list at load), section alignment 16/8/4, inline arrays emitted in reverse
member order, deduped string pool.

## Open questions (do not block decoding)
- MetaType 6 (Recursive) unobserved in corpus; nobody public decodes it.
- Array `count` read as u32 (u64-safe); no corpus array exceeds 2³².
- Header `@0x34–0x3C` reserved words; member `@0x14` flag + `@0x18` default semantics not fully traced.
- BE (`0x20464441`) and v<4 (0x30 instance stride) handled by the engine but absent from this corpus.

## Tool
`tools/jc4_adf` — `info <adf>` / `dump <adf>` (→ JSON) / `verify <dir>`. Feed it files from
`jc4_arc extract`. Write path (ADF encode) is a future task, gated by the same reflection model.

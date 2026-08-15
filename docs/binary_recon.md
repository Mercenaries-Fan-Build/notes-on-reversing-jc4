---
status: current
evidence: proven
verified_on: 2026-08-15
witness: filesystem survey of the retail Steam install; xxd of boot/game0.{tab,arc}; DLL inventory
---

# Binary & install recon — Just Cause 4

Retail install: `C:\Program Files (x86)\Steam\steamapps\common\Just Cause 4` (Steam AppID **517630**,
build `1711159`). ~70 GB. This is the ground-truth asset + binary source.

## Engine

**Apex ("Avalanche Open World") engine** — same family as Just Cause 3, theHunter: Call of the Wild,
RAGE 2, Generation Zero. Implication: the JC3-era Apex community toolchain is the transfer target, not
a from-scratch build. See [`lineage_and_divergence.md`](lineage_and_divergence.md).

## The executable

`JustCause4.exe` — **268 MB, x64, DirectX 11**, unpacked (no DRM to unwrap; disassemble directly).
Avalanche shipped it with a reverse-engineer's gift basket — do not overlook these:

- **`renderdoc.dll`** — RenderDoc is built into the shipping build. GPU frame-capture mesh/texture
  ripping is viable on day one, independent of cracking the on-disk formats.
- **`udis86.dll`** — an x86 disassembler embedded in the binary.
- **`WinPixEventRuntime.dll`** + **`GFSDK_Aftermath_Lib.x64.dll`** — named PIX GPU markers and Nvidia
  crash-replay; named GPU events to anchor render analysis.
- **`CacheSim.dll`**, **`CrashRpt1403`/`dbghelp`/`dbgcore`/`symsrv`** — crash + symbol infrastructure.

Third-party middleware (each a known format to lean on):
- **Oodle** — `oo2core_7_win64.dll` (Kraken). Archive payload compression. Link the game's own copy.
- **FMOD Studio** — `fmod_studio_F.dll` (audio; `.bank`).
- **Bink2** — `bink2w64.dll` (video).
- **RAD Telemetry** — `rad_tm_win64.dll` (profiling).
- **Steam** — `steam_api64.dll`, `sdkencryptedappticket64.dll`.

## Archives — `archives_win64/` (~70 GB, 159 `.tab`+`.arc` pairs)

**Format: TAB/ARC v2.** `.tab` = table of contents, `.arc` = packed blob.

- **TAB v2 header** (proven, `boot/game0.tab` @0): `TAB\0`, version `2.1`, alignment `0x1000`, then a
  zero dword; entries begin at `0x10`. Entry struct is **not** JC2's 12-byte `{hash,offset,size}`
  (interleaved `0xFFFFFFFF` fields present) — **first double-blind crack target**; see `tools/jc4_arc`.
- **ARC payload** (proven): name-prefixed typed records; `boot/game0.arc` first entry is
  `AICoverSettings.adf` (` FDA` = `ADF ` little-endian).
- **Compression**: Oodle Kraken (from the shipped `oo2core_7_win64.dll`), not zlib.

Layout of `archives_win64/`:
- `boot/`, `boot_patch/`, `boot/hires/` — startup + hi-res stream.
- `main/` — the bulk: `game0..N.arc` at ~890 MB each, plus language packs `main/{ara,bra,eng,fre}`.
- `main_patch/` — patch/override layer (candidate for the additive-mod mount point — verify).
- DLC bundles: `agency`, `daredevil`, `demonios`, `wpn_904_dragon`, `cp_deathstalker`,
  `cp_digitaldeluxe`, `cp_goldengear`, `cp_neonracer`, `cp_renegade`, `market1..6`.

## Inner format stack to crack (in dependency order)

1. **TAB v2 entry table** → locate/decompress every arc entry (needs Oodle). *(jc4_arc — in progress)*
2. **ADF** (Avalanche Data Format) — the structured-data layer; nearly everything is described by it.
3. **AAF / SARC** — container nesting inside arc entries (confirm which JC4 uses).
4. **RBM/RBN** — RenderBlockModel meshes → glTF.
5. **DDSC / HMDDSC** — streamed-mip textures → DDS.
6. **FMOD `.bank`** — audio.

## RE workflow to stand up

- **Ghidra headless decomp** of `JustCause4.exe` as the oracle (regenerable, gitignored, corpus-indexed).
- **x32dbg/x64dbg MCP** for the last inferred gaps — note this is a **64-bit** target, so the x64dbg
  build/bridge, not the 32-bit one used on the Pandemic games. Standing rule from prior projects:
  inspect read-only while PAUSED; the user drives execution; never resume; no conditional bp on a hot
  per-frame function.
- **Hash-name recovery** once the addressing hash is identified (rainbow table + GPU brute-force pattern
  from the Mercs 2 repo).

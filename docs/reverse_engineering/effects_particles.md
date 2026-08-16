# Effects & Particles (Pfx / VFX) — GPU-simulated particle system, mesh effects & decals

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

## Overview

JC4's visual-effects layer is a **fully GPU-driven particle system**. The CPU side (`CEffectSystem` and a
family of emitter classes) declares emitters and pushes new-particle records into GPU buffers; the actual
simulation — emission finalize, per-particle *modifiers* (the "affectors"/force fields), depth-sort and
lighting — runs entirely as **compute-shader passes over structured buffers** that are allocated once at
device-init time. The heart of the system is `FUN_1402f77a0` (proven): a single init routine that creates
every GPU buffer, names each one with a `"VFX_…"` debug string, records the **element stride (= the C struct
size) and element count**, and binds them into the compute resource tables. Because the export is
functions-only, those `"VFX_…"` names and their struct sizes survive verbatim — giving a complete, byte-level
catalog of the affector param blocks. Mesh effects (`CMeshEffectManager`) and destruction decals
(`CHavokDestructionDecalManager`) are separate managers that ride the same engine spine. (proven)

Lineage note: this is Apex-engine VFX (JC2/JC3 relatives), a *later* revision — treat names/sizes as
JC4-specific, verified against this binary only, not carried from prior projects.

## Key classes & functions

| Name string | FUN_ / DAT_ | Role |
|---|---|---|
| `CEffectSystem` | registered `FUN_148f960c0` @ line 4138638, type-tag **`0x41ceb158`** | The VFX game-system/manager singleton |
| `CMeshEffectManager` | alloc'd (0x1810 B) in `FUN_14025f000` (`thunk_FUN_14ad68360("CMeshEffectManager")`), stored `DAT_142cafd98` | Mesh-based (geometry) effects manager |
| `CPfxBodyPropsSystem` | registered `FUN_148f958e0` @ line 4138487, type-tag = string ptr | Pfx body-props (per-body cloth/prop physics) system |
| `CPfxWindPhysicsSystem` | registered `FUN_148f958e0` @ line 4138470, type-tag **`0x41cb1660`** | Pfx wind-physics (see `wind_and_weather.md`) |
| `CHavokDestructionDecalManager` | registered `FUN_148f958e0` @ line 4138402, tag **`0x41cad740`**; installed `FUN_1400b8...` @ line 18567 | Destruction decal manager (see `destruction.md`) |
| `CProjectEffectToTerrain` | registered `FUN_148f960c0` @ line 4138655 | Projects effects onto terrain |
| **GPU affector/buffer registry** | **`FUN_1402f77a0`** (size 8672), caller `FUN_140310680` | Allocates & names every VFX GPU buffer, records struct sizes, binds compute resource tables |
| GPU per-frame sim dispatch | `FUN_1402feaa0` (size 6358) | Runs the compute passes (noise, emplace, modifiers, sort) each frame |
| GPU lightmap pass | `FUN_140300380` | `VFX_Lighting` → `VFX_RenderToLightmap` → `VFX_BlurLightmap` |
| Billboard trim/render | `FUN_147e832b0` | `VFX_TrimVertices`, billboard vertex build (`"VFXBillboard"`) |
| Emitter shape classes | `FUN_140795300…140795810` etc. (each a one-shot RTTI-register via `FUN_140f27f60`) | The emitter kinds (table below) |

### Emitter kinds (CPU-side, each RTTI-registered via `FUN_140f27f60`)

| Emitter class | Register FUN_ | name-len arg |
|---|---|---|
| `CEffectBoxEmitter` | `FUN_140795390` | 0x11 |
| `CEffectCollectibleEmitter` | `FUN_140795420` | 0x19 |
| `CEffectLayerEmitter` | `FUN_1407954b0` | 0x13 |
| `CEffectLineEmitter` | `FUN_140795540` | 0x12 |
| `CEffectPlaneEmitter` | `FUN_1407955d0` | 0x13 |
| `CEffectPointEmitter` | `FUN_140795660` | 0x13 |
| `CEffectSphereEmitter` | `FUN_140795780` | 0x14 |
| `CEffectVolumeEmitter` | `FUN_140795810` | 0x14 |
| `CEffectUIEmitter` | (str @ line 900787, `FUN_140845bd0`) | — |
| `CCameraEffectEmitter` | (string-referenced) | — |

(proven — all read directly from the register-thunk bodies)

## How it works (from the decomp)

### 1. One-time GPU buffer allocation — `FUN_1402f77a0` (proven)

Called once (via `FUN_140310680`), this routine builds the whole GPU particle state. Pattern per buffer: a
local descriptor is filled — `local_1fc` = **element stride (struct size in bytes)**, `local_1f8` = element
count, `local_1f4` = alignment (0x100), `local_1e0` = the `"VFX_…"` debug name — then
`thunk_FUN_14ad70540(device, &desc)` creates the structured buffer and the handle is stored at a fixed offset
in the `CEffectSystem` object (`param_1 + 0x6b0 …`). First the compute **kernels** are bound by name
(`thunk_FUN_14aa0d100`):

```
VFXClearNewPages, VFXEmplaceNewParticles, VFXModCommon, VFXInitSortBuffers,
VFXBatchedBitonicSort{64,128,256,512,1024,2048,4096}Descending   (7 sort kernels)
```

Then the buffers. The **core pool** (single instance):

| Buffer | stride (B) | count | stored at obj+ |
|---|---|---|---|
| `VFX_Particles` | 0x54 (84) | 0x20000 (131072) | 0x6b0 |
| `VFX_SortIndicesBuffer` | 4 | 0x20000 | 0x6b8 |
| `VFX_SortKeysBuffer` | 8 | 0x20000 | 0x6c0 |
| `VFX_SplineBuffer` | 4 | 0x40000 | 0x6c8 |

So the live particle capacity is **131,072 particles × 84-byte records** (proven). Then a **double-buffered**
set (`local_170` loop, 2 iterations) of per-page bookkeeping buffers, each count 0x800 (2048) unless noted:
`VFX_LivePages`(4) · `VFX_PageToEmitterGuid`(4) · `VFX_PageToContainerGuid`(4) · `VFX_EmitterInfo`(**0x13c**) ·
`VFX_ParticleModFlags`(4) · `VFX_SplineBufferOffsets`(4) · `VFX_MergedLocalParamsBuffer`(0x10, ×0x200) ·
`VFX_MergedLocalParamsOffsets`(4, ×0x200) · `VFX_NewParticles`(**0x68 / 104**, ×0x4000) · `VFX_SortPageMap`(4) ·
`VFX_SortBatches %d %d`(8, ×0x800, **7 batches** — one per bitonic size). (proven)

### 2. The force-field affector catalog (proven — the core deliverable)

Inside the same double-buffer loop, `FUN_1402f77a0` allocates one param-buffer per **modifier / affector
kind** — each an array of 2048 (`0x800`) structs. The **stride is the exact GPU/C struct size** of that
affector's parameter block. After allocation the routine binds them into the compute resource descriptor
table at `obj+0x968` via `thunk_FUN_147ec4ed0(table, slot, handle, 0)`, re-stating the same stride — giving a
second, independent confirmation of both size and the shader **binding slot**.

| Affector (`VFX_…`) | struct size | buffer obj+ | bind slot | PTR symbol | line |
|---|---|---|---|---|---|
| `VFX_CommonParams` | 0x34 (52) | 0x840 | 0x15 | `PTR_s_VFX_CommonParams_0_141cfa440` | 230780 |
| `VFX_DampingParams` | 0x0c (12) | 0x850 | 0x16 | `…_141cfa480` | 230793 |
| `VFX_GravityPointParams` | 0x2c (44) | 0x860 | 0x17 | `…_141cfa4c0` | 230806 |
| `VFX_GlobalWind` | 0x24 (36) | 0x870 | 0x18 | `…_141cfa510` | 230819 |
| `VFX_AxialOrbitParams` | 0x48 (72) | 0x880 | 0x19 | `…_141cfa550` | 230832 |
| `VFX_DistanceScaleParams` | 0x10 (16) | 0x890 | 0x1a | `…_141cfa590` | 230845 |
| `VFX_AngleFadeParams` | 0x10 (16) | 0x8a0 | 0x1b | `…_141cfa5e0` | 230858 |
| `VFX_TimeOfDayOpacity` | 0x08 (8) | 0x8b0 | 0x1c | `…_141cfa620` | 230871 |
| `VFX_HueSatValModulate` | 0x20 (32) | 0x8c0 | 0x1d | `…_141cfa660` | 230884 |
| `VFX_DepthCollisionParams` | 0x28 (40) | 0x8d0 | 0x1e | `…_141cfa6a0` | 230897 |
| `VFX_DepthProjectionParams` | 0x10 (16) | 0x8e0 | 0x1f | `…_141cfa6f0` | 230910 |
| `VFX_ContinuesProjectToTerrainParams` | 0x40 (64) | 0x8f0 | 0x20 | `…_141cfa740` | 230923 |
| `VFX_OnBirthProjectToTerrainParams` | 0x30 (48) | 0x900 | 0x21 | `…_141cfa7a0` | 230936 |
| `VFX_InheritVelocityEmitterParams` | 0x10 (16) | 0x910 | 0x22 | `…_141cfa800` | 230949 |
| `VFX_SimplexCurlNoise` | 0x28 (40) | 0x920 | 0x23 | `…_141cfa860` | 230962 |
| `VFX_ParticleFeedback` | 0x1c (28) | 0x930 | (final) | `…_141cfa8a0` | 230975 |

All 16 modifier buffers are 2048-element arrays (`local_1f8 = 0x800`), 0x100-aligned. (proven — every row is a
distinct `local_1fc = <size>` … `thunk_FUN_14ad70540` … `thunk_FUN_147ec4ed0(…, slot, …)` pair in the block.)

The seven affectors named in the task brief (`VFX_GlobalWind`, `DampingParams`, `GravityPointParams`,
`AxialOrbitParams`, `DistanceScaleParams`, `AngleFadeParams`, `SimplexCurlNoise`) are a **subset** of this
16-entry catalog — the full set also covers colour/opacity (`HueSatValModulate`, `TimeOfDayOpacity`), depth
interaction (`DepthCollisionParams`, `DepthProjectionParams`), terrain projection (two `…ProjectToTerrain…`
kinds), velocity inheritance and a feedback channel. (proven)

### 3. Emission-shape param blocks (proven)

Also allocated in the loop — the **spawn-volume** descriptors (how particles are seeded), 2048 each except
kill-volumes (20):

| Shape (`VFX_…`) | struct size | obj+ | count |
|---|---|---|---|
| `VFX_KillVolumeParams` | 0x4c (76) | 0x6d0 | 0x14 (20) |
| `VFX_EmissionBoxParams` | 0x44 (68) | 0x6e0 | 0x800 |
| `VFX_EmissionSphereParams` | 0x40 (64) | 0x6f0 | 0x800 |
| `VFX_EmissionCylinderParams` | 0x40 (64) | 0x700 | 0x800 |
| `VFX_EmissionBoxVolumeParams` | 0x20 (32) | 0x710 | 0x800 |
| `VFX_PagesToClear` | 4 | 0x830 | 0x800 |

These map to the CPU emitter classes (box/sphere/line/plane/volume/point) that stage particles into
`VFX_NewParticles` (104-byte records). (box/sphere/cylinder shapes = proven; the emitter-class→shape mapping is
inferred by name)

### 4. Per-frame compute dispatch — `FUN_1402feaa0` (proven)

The simulation runs each frame as an ordered chain of compute passes (each preceded by its debug marker
string, dispatched through the render command list). Order read from the body:

```
VFX_NoiseGenerateTexture   (curl-noise volume, line 234550)
VFX_DynamicBuffersUpdate   (line 234615)
VFX_UpdateConstants        (line 235038)
VFX_ClearNewPages          → dispatch = livePageCount            (line 235131)
VFX_EmplaceNewParticles    → dispatch = (newParticleCount+0x3f)>>6   [64 threads/group]  (235142)
VFX_Modifiers              → dispatch = liveParticleGroups        (235151)  ← runs the affector catalog
VFX_InitSortBuffer         (235158)
VFX_BatchedBitonicSort     → loop over the 7 batch sizes         (235166)
```

`VFX_EmplaceNewParticles` finalizes staged particles from `VFX_NewParticles` into the pool; `VFX_Modifiers`
is the pass that consumes the 16 affector param buffers (bound at slots 0x15–0x23) to integrate forces,
damping, colour, depth-collision etc.; then the pool is sorted back-to-front by the bitonic sort for correct
alpha blending. Dispatch group counts confirm 64-lane emplace groups. (proven)

### 5. Lighting / billboard render (proven)

`FUN_140300380` runs the lightmap passes (`VFX_Lighting` → `VFX_RenderToLightmap` → `VFX_BlurLightmap`) into
`VFXLightmapAtlas`/`VFXLightmapAtlasBlurred` render targets (allocated back in `FUN_1402f77a0`, offsets
0x19a8/0x1b20). Billboard geometry is trimmed/built in `FUN_147e832b0` (`VFX_TrimVertices`, `"VFXBillboard"`),
with refraction (`VFXRefraction`) and depth-stencil (`VFXDepthStencil`) states also created in the init
routine. (proven)

### 6. Mesh effects & decals (separate managers)

- **`CMeshEffectManager`** — a 0x1810-byte manager object allocated in the global-VFX init `FUN_14025f000`
  (alongside a 0x90-byte pool with two 0x100000-byte / 1 MB scratch allocations and two 0x400-capacity
  reserve lists), stored at `DAT_142cafd98`. Handles geometry-based (non-particle) effects. (proven that it is
  allocated/named here; the per-mesh update path is not yet traced — inferred)
- **Decals** — `CHavokDestructionDecalManager` (installed at line 18567, tag `0x41cad740`) plus a general decal
  render path. The decal **material/shader family** is rich and survives as strings: `DecalSimple`,
  `DecalSkinned`, `DecalSkinnedDestruction`, `DecalSkinnedGeneralMKIII(Destructible)`, `DecalDeformable`,
  `DecalMap`/`DecalMapData`/`DecalMapRuntime`, `DecalMask`, plus GPU passes `GPU_DECAL_CLIPPER`,
  `CopyDecalVertices`, `MergeDecalMaps`, `PullDecalFromQueue`, `Render_Decals`, `DecalCopyBarrier`,
  `CStaticDecalObject`. Full decal mechanism → see `destruction.md`. (proven that the strings/managers exist;
  decal pipeline detail deferred)

## Data & config integration

- `pfxwindcapsulesc` (extension `.pfxwindcapsulesc`, name-hash **`0x138e4211`**) is an ADF payload — referenced
  in `FUN_14008d5b0` @ line 12432 and `FUN_147e832b0`-area @ lines 3648585/3648588. It is the wind-capsule
  config consumed by the Pfx wind path; resolve its typed contents with `jc4_adf`. Cross-ref
  `wind_and_weather.md` and memory `[[composite-assets]]`. (proven that the asset+hash exist; field layout via
  `jc4_adf` — not yet decoded here)
- Emitter/effect *instances* are RTPC entity components (the CPU emitter classes above are the component types);
  their tunable magnitudes live data-side, matching the project-wide "mechanism in code, magnitudes in data"
  finding. Tie each `CEffect*Emitter` to its component hash via `[[rtpc-entity-assembly]]`. (inferred)
- The affector param structs (sizes tabled above) are the GPU-side image of ADF/RTPC "modifier" config blocks;
  a modeller/mod tool can use the stride table as the record layout to author or read them. (inferred)

## Notable constants / tunables

- Live particle pool: **131,072** (`0x20000`) particles × **84 B** (`VFX_Particles`, proven).
- Spline buffer: **262,144** (`0x40000`) entries × 4 B (proven).
- New-particle staging: **16,384** (`0x4000`) × **104 B** (`VFX_NewParticles`, proven).
- Per-page tables: **2,048** (`0x800`) pages; `VFX_EmitterInfo` = **316 B** (`0x13c`) per emitter (proven).
- Modifier/affector arrays: **2,048** structs each, 0x100-aligned (proven).
- 7 bitonic-sort kernels sized 64…4096 (descending) for the back-to-front alpha sort (proven).
- Emplace compute group size = 64 lanes (`(n+0x3f)>>6`, proven).
- Type-tags: `CEffectSystem` `0x41ceb158`, `CHavokDestructionDecalManager` `0x41cad740`,
  `CPfxWindPhysicsSystem` `0x41cb1660` (proven).

## Call-graph highlights

- `FUN_140310680` → `FUN_1402f77a0` (build all GPU buffers + bind compute tables). (proven)
- Per frame: render/effect tick → `FUN_1402feaa0` (compute sim chain) → `FUN_140300380` (lightmap) →
  billboard build `FUN_147e832b0`. (dispatch order proven; the top-level tick caller is `callers=[]` in the
  export — inferred)
- System install: `FUN_148f958e0` / `FUN_148f960c0` register `CEffectSystem`, `CPfxBodyPropsSystem`,
  `CPfxWindPhysicsSystem`, `CHavokDestructionDecalManager`, `CProjectEffectToTerrain` into the manager table
  (same registrar family as the rest of the engine — see `README.md` "shared engine spine"). (proven)
- Emitter classes register via `FUN_140f27f60` (the global name→type factory, lookup3-hash keyed). (proven)

## Open questions / lower-confidence

- Internal field layout of each affector struct (e.g. what the 44 bytes of `VFX_GravityPointParams` are) — only
  the *total size* is proven from the stride; per-field decode needs the compute-shader disassembly or the ADF
  modifier schema. (open)
- Exact CPU→GPU emission path: which function writes `VFX_NewParticles` records and how emitter transforms are
  fed in (the `CEffect*Emitter::Emit` bodies are `callers=[]`/unnamed). (open)
- `CMeshEffectManager` and `CPfxBodyPropsSystem` per-frame update loops not yet traced. (open)
- `pfxwindcapsulesc` field layout — decode with `jc4_adf`. (open)

## Appendix — decomp anchors

- **`FUN_1402f77a0`** @0x1402f77a0 (size 8672), caller `FUN_140310680` — the affector/buffer registry. Kernel
  binds lines 230451–230461; core pool 230475–230533; per-page loop 230542–230700; emission shapes 230701–230779;
  affector catalog 230780–230987; compute-table binds (slots 0x15–0x23) 231086–231132.
- Affector PTR symbols: `PTR_s_VFX_CommonParams_0_141cfa440` … `PTR_s_VFX_ParticleFeedback_0_141cfa8a0`
  (see table for each address).
- **`FUN_1402feaa0`** @0x1402feaa0 — per-frame compute dispatch; markers at lines 234550, 234615, 235038,
  235131, 235142, 235151, 235158, 235166.
- **`FUN_140300380`** @0x140300380 — lightmap passes (lines 235222–235257).
- **`FUN_147e832b0`** @0x147e832b0 — billboard trim (`VFX_TrimVertices` line 3638477).
- System registration: `FUN_148f958e0` (lines 4138402/4138470/4138487), `FUN_148f960c0` (line 4138638).
- Emitter registers: `FUN_140795390`…`FUN_140795810` (lines 804118–804279); `CEffectUIEmitter`
  `FUN_140845bd0` (line 900787).
- `CMeshEffectManager` alloc: `FUN_14025f000` (line 172071).
- Decal manager install: line 18567; `pfxwindcapsulesc`/`0x138e4211`: line 12432.
- Decal shader/pass strings: `DecalSimple`, `DecalSkinned*`, `DecalDeformable`, `DecalMap*`, `DecalMask`,
  `GPU_DECAL_CLIPPER`, `CopyDecalVertices`, `MergeDecalMaps`, `PullDecalFromQueue`, `Render_Decals`.

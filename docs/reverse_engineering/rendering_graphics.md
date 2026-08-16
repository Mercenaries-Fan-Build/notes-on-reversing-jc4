# Rendering & graphics core — render blocks, material constants, the deferred/HDR/composite pipeline

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

> "proven" = read directly from the decomp (`output/_ghidra_jc4/jc4_all_functions_decomp.txt`).
> All `FUN_`/`DAT_`/`PTR_` addresses below are from that dump. Names in `"quotes"` are the engine's own
> C++ class / GPU-pass / constant-buffer strings that survive as string constants.
>
> **Scope.** This doc covers the **render-block + material-constants system**, **model instancing**, and the
> **frame / deferred / HDR / composite pipeline** (named GPU passes). Lighting/shadows, environment/TOD, and
> the volumetric weather (tornado) pipeline are **separate docs** — see `wind_and_weather.md` for the tornado
> compute/raymarch passes, and cross-reference them here rather than duplicating. The CarPaint *shader*
> reconstruction lives in the workshop / memory `[[model-amf]]`; this doc documents the CarPaint render
> **block** (its C++ constant-buffer builder), not the shader math.

## Overview

JC4 (Avalanche **Apex** engine) draws every mesh through a **render block**: a C++ object that is the
material/shader unit for one draw. A model's AMF material (an ADF struct inlined in the model unit, memory
`[[model-amf]]`) carries a **type-id** (an ADF type-hash) that selects *which* `CRenderBlock*` subclass owns
that material. The render block's job at bind time is to (1) validate the material blob's type-id, (2) decode
the inline material fields into a packed GPU **constant-buffer** struct, and (3) allocate/update that constant
buffer and stash its handle on the render-block object. The engine ships a fixed family of block types:
`CRenderBlockGeneral` (the default), `CRenderBlockCharacter`, `CRenderBlockCarPaint`, `CRenderBlockCarLight`,
`CRenderBlockBark`, `CRenderBlockHologram`, `CRenderBlockEnergyShield`, `CRenderBlockDemonDome`,
`CRenderBlockDemonOrganic`, `CRenderBlockTypeWeather`, `CTerrainRenderBlockForest`, plus several unnamed
entries (proven — each has its own constant-buffer builder in a consecutive dispatch table; see below).

The scene is composited through a **deferred + HDR + checkerboard** pipeline whose passes survive by name:
frame stages `PreDraw → GBuffer → PostGBuffer → PostEffects → PostDraw` (proven, `FUN_140d50450`) and
draw-list buckets `Shadow / ReflectiveShadowMap / PreZ / Velocity / Vegetation / Transparancy` (proven,
`FUN_140d50550`), feeding a post chain of `SSAO_* → ScreenSpaceReflection* → CompositeBloom → Tonemapping/
Histogram → TAA/FXAA → DepthOfField/MotionBlur → Checkerboard resolve/upscale → Resolve BackBuffer` (all
proven as named pass strings; ordering partly inferred from stage membership — see the pipeline section).

Two engine primitives underpin everything:

- **The constant-buffer creator `thunk_FUN_14ad70540(device, &desc)`** (`FUN_14ad70540` @0x14ad70540) — takes
  a small descriptor (element size × count, name, bind flags, optional init data) and returns a GPU buffer
  handle. Every render block and every weather CB calls it (proven).
- **The shader-for-pass binder family `thunk_FUN_14aa0d100 / …d480 / …e400 / …`** — each acquires a compiled
  shader for a pass *by name* (name → hash → resource lookup `FUN_140f215d0`) and binds it into a slot. The
  functions differ only by a shader-stage tag `DAT_141ecb27c / …e400=DAT_141cf8f48 / …` (proven).

## Key classes & functions

### Render-block constant-buffer builders (the material/shader unit)

Each `CRenderBlock*` subclass has a "prepare material/instance constant buffers" method. They sit in a
**consecutive dispatch table** — the `callers=[0x140d2xxxx]` addresses are the per-type thunk slots, i.e. the
render-block vtable/registry region `0x140d211e0 … 0x140d23160` (proven — contiguous, one thunk per builder).

| Render block | Builder `FUN_` | Dispatch thunk | Material type-id gate | CB name(s) → size | CB handle slot |
|---|---|---|---|---|---|
| **General** (default) | `FUN_14a346d30` @0x14a346d30 | (separate thunk) | none — reads blob raw | `CRenderBlockGeneral::SMaterialConstants` → 0x70; `…::SInstanceConstants` → 0x60 | obj `param_1[0x5e]`/`[0x5f]` (=+0x2f0/+0x2f8) |
| **Bark** | `FUN_14a383d90` @0x14a383d90 | 0x140d211e0 | `param_3[1]==0x60a5c450` | `CRenderBlockBark_MaterialConstants` → 0x30 (+ 0x50-byte CPU struct @obj+0x138) | obj+0x128 |
| **CarLight** | `FUN_14a384370` @0x14a384370 | 0x140d21490 | `param_5+0x60==0x1c363162` | `CRenderBlockCarLight_StaticMaterialConstants` | — |
| **CarPaint** | `FUN_14a384a00` @0x14a384a00 | 0x140d21710 | `param_3[1]==0x73375a05` (+sub `0x1c363162`) | `CRenderBlockCarPaint_StaticMaterialConstants` → 0x100; `…_DynamicMaterialConstants` → 0x30 | obj+0x300 / obj+0x308 |
| **Character** | `FUN_14a3855f0` @0x14a3855f0 | 0x140d21d50 | `param_3[1]==0x25970695` (also `0x1bac0639`, `0x342303ce`) | `CRenderBlockCharacter` constants | — |
| **DemonDome** | `FUN_14a385dc0` @0x14a385dc0 | 0x140d223b0 | (reads blob) | `CRenderBlockDemonDome::SMaterialConstants` | — |
| **DemonOrganic** | `FUN_14a386020` @0x14a386020 | 0x140d22580 | `param_5+0x60==0x15eec9de` | `CRenderBlockDemonOrganic::SMaterialConstants` | — |
| **EnergyShield** | `FUN_14a386a50` @0x14a386a50 | 0x140d22880 | `param_3[1]==0x0589d3e0` | `CRenderBlockEnergyShield::SMaterialConstants` | — |
| **Hologram** | `FUN_14a387620` @0x14a387620 | 0x140d22e00 | `param_3[1]==0x075af78a` | `CRenderBlockHologram::SMaterialConstants` | — |
| (unnamed, light-like) | `FUN_14a387280`/`FUN_14a387430` @0x14a387430 | 0x140d22ce0 | `param_3[1]==0x1b12d103` | 4-byte fields → obj+0x300/+0x304 | obj+0x300/+0x304 |
| (unnamed) | `FUN_14a386d50` @0x14a386d50 | 0x140d22960 | — | — | — |
| (unnamed) | `FUN_14a387a30` @0x14a387a30 | 0x140d22fc0 | — | — | — |
| (unnamed) | `FUN_14a388220` @0x14a388220 | 0x140d23160 | — | — | — |
| **TypeWeather** | (CB setup) @~line 148584 | — | — | `CRenderBlockTypeWeather` → three 0x40 CBs (size 5000 vtx grid) | obj+0xa0/+0xa8/+0xb0 |

(All rows proven: the class/CB name string is passed to `thunk_FUN_14ad70540` in the cited function, and the
type-id gate is the literal `param_3[1]==0x…` / `param_5+0x60==0x…` comparison read from the decomp.)

### Engine primitives

| Name | `FUN_` | Role |
|---|---|---|
| CB creator | `FUN_14ad70540` @0x14ad70540 (`thunk_FUN_14ad70540`) | Build/upload a GPU (constant/structured) buffer from a descriptor; returns handle (proven) |
| Shader-for-pass (stage A) | `FUN_14aa0d100` @0x14aa0d100 | Acquire compute shader for pass name; stage tag `DAT_141ecb27c` (proven) |
| Shader-for-pass (stage B) | `FUN_14aa0d480` @0x14aa0d480 | Acquire pixel/PS shader for pass name; tag `DAT_141cf8f4c` (proven) |
| Shader-for-pass (stage C) | `FUN_14aa0e400` @0x14aa0e400 | Acquire shader for pass name; tag `DAT_141cf8f48` (proven) |
| Resource lookup by hash | `FUN_140f215d0` | Name-hash → shader/pipeline resource (called by the binders) (proven) |
| Pass/resource registrar | `FUN_140c9cad0` @0x140c9cad0 (384 B, 6.1k-style callers) | Register a named render pass with a `PTR_s_<Pass>` resource-descriptor (proven) |
| Frame-stage interner | `FUN_140d50450` @0x140d50450 | Interns `PreDraw/GBuffer/PostGBuffer/PostEffects/PostDraw` via the name-hash factory `FUN_140f27f60` (proven) |
| Draw-bucket interner | `FUN_140d50550` @0x140d50550 | Interns `Shadow/ReflectiveShadowMap/PreZ/Velocity/Vegetation/Transparancy` (proven) |
| Name-hash factory | `FUN_140f27f60` | The global name→type factory (memory `[[name-cracking-engine]]`); passes bind here too (proven) |

### Managers

| Class string | Where | Role |
|---|---|---|
| `CModelInstanceManager` | registered @line 3200055 (`FUN_14008f5d0(..,"CModelInstanceManager")`); manager table `FUN_148f960c0` @line 4138604 | Owns model-instance batches; feature-flag registry (proven registration) |
| `CModelInstance` | @line 3200058 | Per-instance object; 12 named feature flags (proven) |
| `CMeshEffectManager` | `thunk_FUN_14ad68360("CMeshEffectManager")` @line 172071 | Mesh-effect (decal/overlay) system (proven registration) |
| `ChromaManager` | `thunk_FUN_148f6e780(world,"ChromaManager",..)` @line 3209730 / 4139212; `ChromaManagerUpdateThread` @line 4380308 | Razer Chroma RGB peripheral lighting — **not** core scene rendering (proven; role inferred) |

## How it works (from the decomp)

### 1. A render block is a material-constant compiler

Take the archetype, **`CRenderBlockGeneral`** (`FUN_14a346d30`). Its builder receives the render-block object
(`param_1`), a device/context (`param_2`), the AMF material blob (`param_3`), a per-frame/double-buffer flag
(`param_5`), and a mesh-attribute record (`param_4`). It:

1. **Remaps material flag bits.** A long sequence rewrites bits of the source material flag word
   `*(param_3+0x13)` into the block's runtime flag word `*(param_1+0x2cc)` at engine-specific bit positions
   (e.g. src bit 5 → bit 0x1a, src bit 0x14 → bit 9). This is the material→pipeline-state translation (proven).
2. **Packs `SMaterialConstants` (0x70 = 112 bytes).** Copies material scalars/vectors out of the blob
   (`param_3+0x0…+0x6c`) into a stack struct, tags it `local_170="CRenderBlockGeneral::SMaterialConstants"`,
   and calls `thunk_FUN_14ad70540(*param_2,&desc)` → handle stored at `param_1[0x5e]` (obj+0x2f0) (proven).
3. **Packs `SInstanceConstants` (0x60 = 96 bytes).** Builds a second struct that folds in per-instance world
   data from `param_4` and global render params from `DAT_142cca630` (e.g. `+0x15c`, `+0x160`, `+0x14c`), plus
   a wrapped/periodic value chain over `DAT_141ca6c98…cdc` (a fract/wrap approximation — likely a time or
   phase animation term; **speculative** as to exact meaning), and a computed LOD/detail byte
   `(int)(DAT_141ebfa94 / bboxExtent)` clamped to 0xff. Handle → `param_1[0x5f]` (obj+0x2f8), then
   `thunk_FUN_14a4a3680(param_1)` finalizes (proven for structure; math semantics partly inferred).

**Bark** (`FUN_14a383d90`) is the clearest end-to-end example: it gates on `param_3[1]==0x60a5c450`, heap-allocates
a 0x50-byte CPU material struct at obj+0x138, fills it from the blob with unit conversions
(`(float)puVar6[8] * DAT_141cfb42c`, `* DAT_141cae160`), then builds the 0x30-byte
`CRenderBlockBark_MaterialConstants` CB (handle → obj+0x128) (proven).

**CarPaint** (`FUN_14a384a00`) gates on `param_3[1]==0x73375a05`, copies ~40 material fields into a **256-byte
static** CB and a **48-byte dynamic** CB (`obj+0x300`/`obj+0x308`), and additionally scans the mesh-attribute
list `param_5` (records of stride 0x14; `type==2` entries) to gather up to three texture/UV slots
(`local_58[0..2]`). A sub-gate `param_5+0x60==0x1c363162` enables extra "car light" bits and computes a
material-instance index into a table at `DAT_142cca5f8` (stride 0x4e0) (proven). The paint shader math itself
is not in this function — see the workshop reconstruction (`[[model-amf]]`).

**Character** (`FUN_14a3855f0`) accepts three material variants by type-id (`0x25970695`, `0x1bac0639`,
`0x342303ce`) — i.e. one render block, several authored material layouts (skin/cloth variants inferred) — and
branches its instance-constant packing on `(*(param_1+0x4c) & 0x30000)` (a per-instance mode select) (proven).

**The type-id at `param_3[1]` is the render-block selector** and is an **ADF type-hash** of the material-constants
struct: the same reflection identity documented in `docs/formats/adf.md`. This is the concrete bridge from the
data side (an AMF material blob) to the code side (which `CRenderBlock*` compiles it) (proven mechanism;
mapping each hash → a named ADF struct is open, see below).

### 2. The constant-buffer creator

`FUN_14ad70540(device, &desc)` reads the descriptor: element size at `desc[2]` (+0x10), element count at
`desc+0xc`, per-instance flags at `desc+0x31` (bit0=…, bit1=…, bit2 forces 16-byte size alignment
`size = size+0xf & ~0xf`, bit3=…), a usage index at `desc+0x15` into table `DAT_141f07d08`, and an optional
init-data pointer / name. It computes `byteSize = elem * count`, sets up bind/usage bitfields (`local_5c`),
and allocates the GPU buffer, returning its handle (proven). Every render-block CB and the three
`CRenderBlockTypeWeather` CBs (each 0x40 bytes, sized for a 5000-vertex procedural grid, obj+0xa0/a8/b0)
route through it (proven).

### 3. Shaders are bound to passes by name

Post/compute setup functions bind shaders with the `thunk_FUN_14aa0…` family, e.g.
`thunk_FUN_14aa0e400(device, &slot, "SSAO_SAO")`. Internally (`FUN_14aa0e400`) the pass name is hashed
(`thunk_FUN_14aa0f4a0(name, DAT_141cf8f48)` — the `DAT_` is the shader-stage/permutation tag), looked up via
`FUN_140f215d0`, and the resulting shader stored into the slot (proven). So **pass name → shader** is a hash
lookup keyed by our cracked name hash (`docs/formats/name_hash.md`), and the pass-name inventory below *is*
the shader inventory recoverable from this dump. The compiled **bytecode is not present** (walled — data side).

### 4. Frame stages & draw buckets

`FUN_140d50450` builds pass-descriptor objects tagged by stage — an array
`["PreDraw","GBuffer","PostGBuffer","PostEffects","PostDraw"]` indexed by `param_2`, each interned through the
name-hash factory `FUN_140f27f60` (proven). `FUN_140d50550` does the same for draw-list buckets
`["Shadow","ReflectiveShadowMap","PreZ","Velocity"] + "Vegetation" + "Transparancy"` (proven — note the
shipped typo "Transparancy"). These are the top-level frame graph: geometry is bucketed (shadow/prez/velocity/
vegetation/transparency) and composited across stages (predraw→gbuffer→postgbuffer→posteffects→postdraw).

### 5. Model instancing

`CModelInstance` registers a **12-bit feature-flag vocabulary** by name via `thunk_FUN_147620a00(obj, bit, name)`
(all proven, @lines 3200059-3200071):

| Bit | Name | Bit | Name |
|---|---|---|---|
| 0x0 | (base) | 0x40 | `DecalMask` |
| 0x1 | `Deform` | 0x80 | `EnvFX` |
| 0x2 | `AnimateUV` | 0x100 | `Billboard` |
| 0x4 | `RelativePos` | 0x200 | `Window` |
| 0x8 | `EffectPoints` | 0x400 | `VegUpdate` |
| 0x10 | `DynUI` | 0x800 | `MaterialTune` |
| 0x20 | `Instancing` | | |

`CModelInstanceManager` registers flag `0xf = "ModelInstanceBatch"` (proven). The vegetation-instancing path is
a set of **GPU compute passes** bound through `PTR_s_RenderBlockVegSamplingShared_141ccf608` and
`PTR_s_RenderBlockVegUpdateShared_141cd27e0`: `PreSamplePositions`, `SamplePositionsOpt(Detail)`,
`VegEstimateNumPositions`, `VegSamplesFilterCellsAlt`, `VegSamplePointsToInstances`,
`VegSamplesCompactInstances`, `VegSysUpdateInstances`, `VegSysUpdateSortByIndex`, `VegSysUpdateProcessPass`,
`VegSysUpdateWriteInstPass`, `VegSysUpdateCalcClosestDist`, `VegSysUpdateClearInstBuffers` (all proven, @lines
117245-117605). This is a GPU-driven vegetation placement/instancing system.

## The deferred / HDR / composite pipeline (named passes)

Every entry below is a **proven** pass-name string bound via the `thunk_FUN_14aa0…` shader binders or
registered via `FUN_140c9cad0`/`FUN_140c91e30` with a `PTR_s_<Pass>` descriptor. Grouping into stages is
inferred from the stage/bucket enums (§4); exact intra-stage order is not asserted where not evident.

**Geometry / GBuffer (stage `GBuffer`/`PostGBuffer`, buckets PreZ/Velocity/…):**
`GBuffer`, `PostGBuffer`, `PreZ`, `Velocity`, `Emissive` (@line 3638548), `CompositeVelocity`,
`CompositeHighPrecisionVelocityFilter` (@line 4593813). Terrain uses its own deferred blocks:
`TerrainDeferredDetail/Near/Near_B/Far/Far_B` (@lines 118101-118108).

**Ambient occlusion (SSAO, scalable-AO family):** `SSAO_SAO`, `SSAO_Apply`, `SSAO_ApplyIrradiance`,
`SSAO_Blur`, `SSAO_Minify`, `SSAO_MipCopy`, `SSAO_ReconstructZ`, `SSAO_TemporalFilter`, `SSAO_5x5Blur`
(@lines 1551867-1551877).

**Screen-space reflections:** `ScreenSpaceReflection`, `CompositeScreenSpaceReflectionProxies`,
`CompositeWaterProxies`(`_nobranching`), `ScreenSpaceReflectionBlur`(`ReducedFireFlies`),
`SSReflectionDilation_3x3`, `SSReflectionErosion_3x3`, `GraphicsSSReflection` (@lines 1540429-1541869); resource
pointers `PTR_s_ScreenSpaceReflection_141ec6d70`, `PTR_s_ScreenSpaceReflectionBlur_141ec1318` (proven).

**Image-based lighting / cubemaps:** `CompositeEnvCube`, `CompositeCubemap`, `CompositeUnfilteredCubemap`,
`CompositeUnfilteredIBLCubemap` (@lines 4607131-4607138, 1526333-1526546).

**Bloom:** `CompositeBloom` (`PTR_s_CompositeBloom_141e81210`), `BloomSeparateBlur`, `BloomAccumulatedBlur`,
`BloomDilation_3x3`, `BloomErosion_3x3`, `CompositeBloom::LocalResources` (@lines 1510793-1514723).

**Exposure / tonemap:** `TonemappingHistogram`, `TonemappingHistogramClear`, `HistogramCompute`,
`HistogramGeneration`, `HistogramTexture` (@lines 1515060-1519434, 4585444).

**Anti-aliasing:** TAA — `TAAResolve`, `TAAResolveHorz`, `TAAResolveRef`, `TAA::LocalResources`; FXAA —
`FXAA`/`FXAAHdrTv`, `FXAAPass1RGBCS`, `FXAAResolveWorkQueueCS`, `FXAAPass2H/VCS`, `FXAACopyBuffer(Hdr)`,
`FXAAComputeConstants` (@lines 1512077-1516714).

**Lens post:** `DepthOfField`, `MotionBlur`, `CompositeMotionBlurFilter`, `MotionBlurRandomJitterBuffer`,
`MotionBlurStrength` (@lines 1510382-1517884, 4593817).

**Checkerboard resolve (the console-style half-res reconstruction):** `Checkerboard`, `CheckerboardApply`,
`CheckerboardApplyUpscale`, `CheckerboardApplySharpen`, `CheckerboardResolve`/`CheckerboardResolveHDR`,
`CheckerboardHistory`/`…HDR`, and inputs `CheckerboardResolveTexture`, `CheckerboardPrevColorTexture`,
`CheckerboardPrevDepthTexture` (@lines 1499303-1499385). The `…HDR` variants and the `HDR_TV` output string
(@line 1551549) confirm an HDR-output path.

**Final resolve:** `Resolve BackBuffer`, `Resolve DepthBuffer` (@lines 1554546-1554573).

**Volumetrics (cross-ref `wind_and_weather.md`):** `VolumetricCloudsComposite`, `LowResVolumetricsComposite`,
`LowResVolumetricsCompositeTornado`, `LowResCompositeCB`, `WaterBumpComposite` (@lines 140767-1505651).

## Data & config integration

- **Selector = ADF type-hash.** A render block is chosen by the material blob's type-id at `param_3[1]`
  (`0x60a5c450` Bark, `0x73375a05` CarPaint, `0x25970695`/`0x1bac0639`/`0x342303ce` Character, `0x0589d3e0`
  EnergyShield, `0x075af78a` Hologram, `0x1b12d103` light-like), with sub-gates on the mesh-attribute record
  `param_5+0x60` (`0x1c363162` CarLight/CarPaint, `0x15eec9de` DemonOrganic). These are ADF struct type-hashes
  (`docs/formats/adf.md`) carried in the AMF model unit's inline material (memory `[[model-amf]]`,
  `[[composite-assets]]`) (proven that they gate; the reverse map hash→named struct is open).
- **Names hash into one registry.** Frame stages, draw buckets and GPU passes are all interned through the
  same `FUN_140f27f60` / lookup3 `hashlittle` identity used for asset paths and gameplay classes
  (`docs/formats/name_hash.md`, memory `[[name-hash-cracked]]`) — so the pass-name strings above are directly
  hashable to the ids the resource system uses (proven).
- **Textures.** Bound as AVTX (`docs/formats/avtx.md`); the render blocks reference texture slots gathered from
  the mesh-attribute list (CarPaint `local_58[0..2]`) rather than embedding pixels (proven).

## Notable constants / tunables

- **Constant-buffer sizes** (bytes): General material 0x70 / instance 0x60; CarPaint static 0x100 / dynamic
  0x30; Bark 0x30 (+0x50 CPU struct); TypeWeather 3 × 0x40 (proven, cited above).
- **Render-block CB-handle offsets on the object:** General +0x2f0/+0x2f8; CarPaint +0x300/+0x308; Bark +0x128
  (struct +0x138); light-like +0x300/+0x304; TypeWeather +0xa0/+0xa8/+0xb0 (proven).
- **Material type-id magics:** see selector table (proven).
- **LOD/detail term (General):** `clamp((int)(DAT_141ebfa94 / maxBBoxExtent), 0, 255)` scaled by
  `DAT_142cca630+0x15c` (proven mechanism; the DAT_ values are data-driven, not in this dump — walled).
- **TypeWeather grid:** 5000 vertices, index buffer built as quad strips (`0x4e2` iterations of 6 indices)
  (proven, @lines 148622-148634).

## Call-graph highlights

- Render-block builders `FUN_14a383d90 … FUN_14a388220` ← dispatch thunks `0x140d211e0 … 0x140d23160`
  (contiguous per-type table) → all call `thunk_FUN_14ad70540` (CB create) (proven).
- Post/compute setup fns call `thunk_FUN_14aa0d100/d480/e400(device,&slot,"PassName")` → `FUN_140f215d0`
  (resource lookup) (proven).
- `FUN_140d50450` / `FUN_140d50550` → `FUN_140f27f60` (name→id) for stage/bucket interning (proven).
- `CModelInstance` setup → `thunk_FUN_147620a00(obj,bit,"FlagName")` ×12; vegetation compute via
  `PTR_s_RenderBlockVeg{Sampling,Update}Shared_*` (proven).

## Open questions / lower-confidence

- **Hash → named ADF struct** for each material type-id (`0x60a5c450`, `0x73375a05`, …) — gates are proven; the
  human-readable struct names are not recovered here (feed to `jc4_probe` / dict harvest).
- **General instance-constant math** (the `DAT_141ca6c*` wrap/periodic chain) — structure proven, semantics
  (time phase? wind sway?) inferred/speculative.
- **The four unnamed dispatch entries** (`FUN_14a386d50`, `FUN_14a387430`, `FUN_14a387a30`, `FUN_14a388220`) —
  builders proven; which named block each is (Light/Window/Decal/General-variant) is open.
- **`CTerrainRenderBlockForest`** — named in the anchor set but its builder wasn't isolated in this pass;
  terrain has its own `TerrainDeferred*` deferred family (proven present) worth a dedicated dig.
- **Intra-stage pass ordering** — the stage/bucket taxonomy is proven; the exact submission order within
  `PostEffects` (SSAO vs SSR vs bloom vs tonemap vs TAA vs CB-resolve) is inferred, not read from a single
  orchestrator function.
- **Shader bytecode & data-driven material constants** — walled (functions-only export; DAT_ data absent).

## Appendix — decomp anchors

- Render-block builders: `FUN_14a346d30` (General), `FUN_14a383d90` (Bark), `FUN_14a384370` (CarLight),
  `FUN_14a384a00` (CarPaint), `FUN_14a3855f0` (Character), `FUN_14a385dc0` (DemonDome), `FUN_14a386020`
  (DemonOrganic), `FUN_14a386a50` (EnergyShield), `FUN_14a387620` (Hologram), `FUN_14a386d50`/`FUN_14a387430`/
  `FUN_14a387a30`/`FUN_14a388220` (unnamed); TypeWeather CB setup @line 148584. Dispatch thunk table
  `0x140d211e0 … 0x140d23160`.
- Primitives: `FUN_14ad70540` (CB create), `FUN_14aa0d100`/`FUN_14aa0d480`/`FUN_14aa0e400` (shader-for-pass),
  `FUN_140f215d0` (resource lookup), `FUN_140c9cad0` (pass register), `FUN_140d50450`/`FUN_140d50550`
  (stage/bucket intern), `FUN_140f27f60` (name→id), `thunk_FUN_14aa0f4a0` (pass-name hash).
- Managers: `CModelInstanceManager`/`CModelInstance` @lines 3200055-3200072; `CMeshEffectManager` @line 172071;
  `ChromaManager` @lines 3209730/4139212/4380308; model-instance feature flags `thunk_FUN_147620a00`.
- Pass strings: see the pipeline section for exact line numbers (Checkerboard 1499303, Bloom 1514666, SSR
  1540429, SSAO 1551867, Tonemap/Histogram 1515060/1518758, TAA/FXAA 1512077/1516470, DoF/MotionBlur
  1510382/1510855, Resolve 1554546, GBuffer/stages 1537862/1537916, Emissive 3638548).
- Related docs: `docs/formats/adf.md`, `docs/formats/avtx.md`, `docs/formats/name_hash.md`;
  `wind_and_weather.md` (tornado/cloud volumetrics); memory `[[model-amf]]`, `[[composite-assets]]`,
  `[[name-hash-cracked]]`.

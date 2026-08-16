# Lighting & shadows — light types, cascaded/reflective shadow maps, clustered deferred lights

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

> "proven" = read directly from the decomp (`output/_ghidra_jc4/jc4_all_functions_decomp.txt`).
> All `FUN_`/`DAT_`/`PTR_` addresses below are from that dump. Names in `"quotes"` are the engine's own
> C++ class / GPU-pass / render-target / shader-permutation strings that survive as string constants.
>
> **Scope.** This doc covers **how lights are declared and driven** (the world-object light classes,
> spotlight controller, light occlusion planes), the **shadow-map pipeline** (the `Shadow` and
> `ReflectiveShadowMap` draw buckets, the 8-slot cascaded shadow define family, the RSM render targets,
> cloud shadows) and the **deferred/clustered light-resolve + ambient/GI/probe** stages. It builds on
> `rendering_graphics.md` (the render-block/material system, the frame-stage/draw-bucket interners, the
> shader-for-pass binder family) — those primitives are re-used here rather than re-derived. The **sky /
> time-of-day / atmospheric scattering / volumetric-cloud raymarch** side lives in the separate
> environment doc; where a lighting input is produced there (cloud shadow, sky irradiance) it is
> cross-referenced, not duplicated.

## Overview

JC4 (Avalanche **Apex** engine) lights the scene with a **deferred + clustered** pipeline. Opaque
geometry writes a G-buffer; a **`Deferred_ClusteredLighting`** compute/pixel stage then resolves all
local lights against clustered light lists, plus a separate **`Deferred_ClusteredLightingIrradiance`**
path for the diffuse-irradiance (GI/ambient) contribution (proven, `FUN_14a3479e0`). Sun/directional
shadowing is a **cascaded shadow map** with up to **8 cascades**, exposed as the shader-define family
`SHADOW_0…SHADOW_7` plus a parallel **static** cascade set `STATIC_SHADOW_0…STATIC_SHADOW_7` and a
`SHADOW_FILTERING` variant (proven, `FUN_140d78c40`). Every shadow-casting mesh is drawn through the
dedicated **`Shadow`** draw bucket, and one-bounce GI is fed by a **`ReflectiveShadowMap`** (RSM) bucket
whose pass writes a flux/normal/depth MRT (proven, `FUN_140d50550`, `FUN_140cdc0e0`). Extra shadow
inputs are composited in: **cloud shadows** (`CLOUDSHADOWS_0/1`, `cloud_shadow_texture`, produced by the
volumetric-cloud system) and **screen-space ambient obscurance** (a Scalable Ambient Obscurance / SAO
implementation, `SSAO_*`, `FUN_140d7a9e0`).

Lights enter the world as reflected **entity objects** — `CDynamicLightObject`, `CDynamicLightFlasher`,
`CEnvironmentLightingObject`, `CSpotlightController` — instantiated by name through the same global
name→type factory `FUN_140f27f60` that installs every other gameplay class (proven). Spotlights
additionally render a **volumetric cone** (`SpotLightCone` / `SpotLightConeInside`, `FUN_14a52edd0`), and
**`CLightOcclusionPlaneObject`** places planes that clip light leakage. Baked indirect light is served
from a **light-probe database** (`LightProbeDBKeys/…Items`, `FUN_14a068f70`) and a **dynamic sky
irradiance cube** (`DynamicSkyIrradianceCube`, `FUN_140cae100`).

## Key classes & functions

### Light world-objects (registered by name through `FUN_140f27f60`)

| Class string | Factory slot (`DAT_`) | Role |
|---|---|---|
| `CDynamicLightObject` | `DAT_142cb0a68` (@line 199283) | Placeable dynamic local light (point/omni or spot source) — the general runtime light (proven registration; light-type inferred) |
| `CDynamicLightFlasher` | `DAT_142cb0a70` (@line 199258) | Blinking/strobing light modulator (proven registration; role inferred from name) |
| `CEnvironmentLightingObject` | `DAT_142cb0ab4` (@line 199333) | Environment / ambient-lighting volume override (proven registration; role inferred) |
| `CLightOcclusionPlaneObject` | `DAT_142cb0a78` (@line 199433) | Light-occlusion plane — clips/darkens light bleed across a plane (proven registration; role inferred) |
| `CSpotlightController` | `DAT_142cb8e5c` (@line 840725/868829) | Drives a spotlight (aim/cone/intensity) — the animated spotlight (proven registration) |
| `CSpotlightWeaponComponent` | `DAT_142cb83c4` (@line 754493) | Weapon-mounted spotlight component (proven registration) |
| `CLightWeaponComponent` | `DAT_142cb8378` (@line 754309) | Weapon flashlight/light component (proven registration) |

(All rows proven for *registration*: the literal class string is passed to the name→type factory
`FUN_140f27f60` at the cited line; the exact light math/params live data-side — see Data & config.)

### Shadow / lighting pass + bucket machinery

| Name | `FUN_` | Role |
|---|---|---|
| Feature/pass-name enumerator | `FUN_140d78c40` @0x140d78c40 (size 3302) | `switch(index) → (*param_2 = stage-group id, *param_3 = define string)`. Source of the `STATIC_SHADOW_0..7`, `SHADOW_0..7`, `SHADOW_FILTERING`, `SHADOW_REFLECTIVE_SUN_NEAR/FAR/CAMERA`, `CLOUDSHADOWS_0/1`, `DEFERRED_LIGHTS(_IRRADIANCE)`, `GLOBAL_ILLUMINATION`, `SKY_LIGHTING`, `VFX_LIGHTING`, `SPOTLIGHT_VOLUMETRICS`, `AO_VOLUMES` names (proven) |
| Draw-bucket interner | `FUN_140d50550` @0x140d50550 | `index → {0:"Shadow", 1:"ReflectiveShadowMap", 2:"PreZ", 3:"Velocity", 4:"Vegetation", 5:"Transparancy"}` (proven — same table as `rendering_graphics.md`) |
| Opaque/depth shader-variant name builder | `FUN_14a37b810` @0x14a37b810 | Composes a shader-permutation name: base pass `{3:"Shadow", 4:"RSM", 0/1/0xe:"PreZ", 2:"PreZVelocity", 0x10:"Glint", 0x11-13:"Outline"}` + geometry suffix (`Skinned`/`VertexAnim`/`HwInstanced`/`DestructionSkin`/…) + `Alphablend`/`Alphatest` + `WorldSpaceNormals` + `Layered`/`Overlay` + `RoadBias` (proven) |
| Per-block bucket variant select | `FUN_14a4a1c90` @0x14a4a1c90 (thunk_FUN_14a4a1c90) | Picks the depth/shadow shader for a mesh by bucket: `{0:"Shadow", 1:"RSM", 2:"RSM", 3:"PreZ"}`, plus `Velocity`/`Billboard`/`Instanced`/`HwInstanced` variants (proven) |
| RSM render-target builder | `FUN_140cdc0e0` @0x140cdc0e0 | Allocates the RSM MRT: `flux_reflective_shadow_map`, `normal_reflective_shadow_map`, `depth_reflective_shadow_map`, each **double-buffered** (`param_1[0..2]` + `[3..5]`) (proven) |
| Clustered deferred-light setup | `FUN_14a3479e0` @0x14a3479e0 (thunk_FUN_14a3479e0) | Binds `PTR_s_Deferred_ClusteredLighting` pipeline + `Deferred_ClusteredLighting%s%s`, `Deferred_ClusteredLightingIrradiance%s`, `Deferred_PassThrough` shaders (proven) |
| SSAO (SAO) setup | `FUN_140d7a9e0` @0x140d7a9e0 | Binds `SSAO_SAO/_Apply/_ApplyIrradiance/_Blur/_Minify/_MipCopy/_ReconstructZ/_TemporalFilter/_5x5Blur` (proven) |
| Light-probe database | `FUN_14a068f70` @0x14a068f70 | `LightProbeDBKeys`, `LightProbeManagementEntries`, `LightProbeDBItems` — baked indirect-light probe store (proven strings; GI role inferred) |
| Dynamic sky irradiance | `FUN_140cae100` @0x140cae100 | `DynamicSkyIrradianceCube` / `IrradianceTarget` — sky-derived ambient irradiance (proven string; role inferred; produced sky-side) |

### Per-render-block shadow/depth shader binders

Each shadow-casting `CRenderBlock*` (see `rendering_graphics.md`) binds its own shadow-pass shaders via
the stage-tagged binders `thunk_FUN_14aa0e400` (VS/CS) / `thunk_FUN_14aa0d480` (PS) / `thunk_FUN_14aadc440`
(name+suffix):

| Block | `FUN_` | Shadow/depth pass shaders it binds |
|---|---|---|
| CarPaint | `FUN_14a388550` @0x14a388550 | `CarPaintShadow`, `CarPaintShadow_Deform`, `CarPaintShadow_Skinned`, `CarPaintPreZVelocity(_Deform/_Skinned)`, `CarLightShadowCloakTransition` (proven) |
| Character | `FUN_140d05a90` @0x140d05a90 | `Character`, `CharacterDepth`, `CharacterVelocity`, `CharacterDepthShadowCloakTransition`, `CharacterDepthCloakTransition`, hair/fur depth variants (proven) |
| SpotLightCone (volumetric) | `FUN_14a52edd0` @0x14a52edd0 (thunk) | `SpotLightCone` (VS+PS), `SpotLightConeInside` (PS) + builds cone geometry buffer via `thunk_FUN_14ad7a400` (proven). Companion: `FUN_14a52c640` @0x14a52c640, resource table `FUN_14a5369c0` @0x14a5369c0 (`SpotLightCone::ResourceTable`) |

## How it works (from the decomp)

### 1. Draw buckets: `Shadow` and `ReflectiveShadowMap` are first-class

The frame's draw lists are interned by `FUN_140d50550`: bucket index `0 → "Shadow"`, `1 →
"ReflectiveShadowMap"`, then `PreZ / Velocity / Vegetation / Transparancy` (proven). Every mesh that
casts a shadow is submitted into the `Shadow` bucket; every mesh that contributes to one-bounce GI is
also submitted into the `ReflectiveShadowMap` bucket. Which *shader* a mesh uses in each bucket is
decided by two cooperating helpers:

- `FUN_14a37b810` builds the **variant name** from the bucket id and the mesh's flags. `case 3 → "Shadow"`
  and `case 4 → "RSM"` select the base; it then appends the geometry/skinning/alpha/road suffixes, so a
  skinned alpha-tested shadow caster ends up requesting e.g. `Shadow` + `Skinned` + `Alphatest` (proven).
- `FUN_14a4a1c90` maps the runtime bucket index onto the concrete shader slot: `{0:"Shadow", 1:"RSM",
  2:"RSM", 3:"PreZ"}` with `Velocity`/`Billboard` and `Instanced`/`HwInstanced` sub-variants (proven). Note
  buckets 1 and 2 both map to `"RSM"` — the RSM is rendered at two ranges (near/far, see §3).

The CarPaint and Character render blocks show the concrete shadow shaders they compile: CarPaint binds
`CarPaintShadow{,_Deform,_Skinned}` (`FUN_14a388550`); Character binds `CharacterDepth` and the
`CharacterDepthShadow*` cloak variants (`FUN_140d05a90`) (proven).

### 2. Cascaded shadow maps — 8 dynamic + 8 static slots

The pass-name enumerator `FUN_140d78c40` is the authority on the shadow define set (proven, read directly
from its `switch`):

| Case range | Stage-group (`*param_2`) | Define strings |
|---|---|---|
| `0x20…0x27` | `3` | `STATIC_SHADOW_0` … `STATIC_SHADOW_7` |
| `0x28…0x2f` | `3` | `SHADOW_0` … `SHADOW_7` |
| `0x30` | `3` | `SHADOW_FILTERING` |
| `0x31…0x33` | `2` | `SHADOW_REFLECTIVE_SUN_NEAR`, `SHADOW_REFLECTIVE_SUN_FAR`, `SHADOW_REFLECTIVE_CAMERA` |
| `0x19…0x1a` | `0x11` | `CLOUDSHADOWS_0`, `CLOUDSHADOWS_1` |

The two parallel 8-entry families are the **cascade slots**: `SHADOW_0..7` are the dynamic sun cascades
re-rendered per frame, and `STATIC_SHADOW_0..7` are a matching set of **precomputed/static** cascades
(static geometry baked once and reused, a common Apex optimization) — the engine can source each cascade
from either (proven that both 8-slot families exist and share stage-group `3`; the static-vs-dynamic
split is inferred from the naming). `SHADOW_FILTERING` is the PCF/soft-shadow sampling variant (inferred
from name). Up to **8 cascades** is the hard ceiling read from the slot count (proven).

### 3. Reflective Shadow Map (RSM) → one-bounce GI

The `ReflectiveShadowMap` bucket feeds a dedicated MRT built by `FUN_140cdc0e0` (proven): three
render targets named `flux_reflective_shadow_map` (radiant flux / reflected colour),
`normal_reflective_shadow_map` (world normals) and `depth_reflective_shadow_map` (depth), each allocated
**twice** (double-buffered for temporal reuse). This is the textbook RSM G-buffer: rendering the scene
from the sun's view and capturing per-texel flux+normal+position lets the lighting stage gather one
indirect bounce. The three `SHADOW_REFLECTIVE_*` defines (sun-near, sun-far, camera) are the RSM's
capture configurations (proven names), and `GLOBAL_ILLUMINATION` (case `0x51`, stage-group `2` — the same
group as the RSM defines) is the resolve that consumes them (proven grouping; consume relationship
inferred).

### 4. Deferred clustered light resolve + irradiance

`FUN_14a3479e0` builds the deferred-lighting pipeline object (proven). It:

1. Binds the pipeline `PTR_s_Deferred_ClusteredLighting_141eb96f8` (obj+0xb8) (proven).
2. Loops building the two shader permutations from a suffix table (`PTR_DAT_142ae7b08` / `…7b18`):
   `Deferred_ClusteredLighting%s%s` for the **direct** local-light resolve, and
   `Deferred_ClusteredLightingIrradiance%s` for the **indirect/ambient irradiance** resolve — chosen by a
   mode flag `local_180` (`param_3`): mode 1 → irradiance path, else → direct path (proven).
3. Binds a `Deferred_PassThrough` shader for the non-irradiance branch (proven).

"Clustered" is confirmed by the pass name (`Deferred_ClusteredLighting`, also referenced at
`FUN_14a4732d0` `"Deferred_ClusteredLighting::LocalResources"` and hash `0x886e4712` at `FUN_14f14de69`),
i.e. lights are binned into view-space clusters and each G-buffer pixel resolves only the lights in its
cluster (froxel/cluster culling — the standard modern deferred approach; the binning code itself is a
compute pass, walled with the rest of the shader bytecode) (proven names; froxel mechanism inferred).
The two top-level define slots are `DEFERRED_LIGHTS` (case `0x6d`, group `0xe`) and
`DEFERRED_LIGHTS_IRRADIANCE` (case `0xb0`, group `0xe`) (proven).

### 5. Spotlights render a volumetric cone

Beyond the deferred contribution, spotlights draw a **volumetric light cone**. `FUN_14a52edd0` binds
`SpotLightCone` (VS via `…e400`, PS via `…d480`) and `SpotLightConeInside` (the shader used when the
camera is inside the cone), and builds the cone's vertex/index geometry through `thunk_FUN_14ad7a400`
(proven). This is gated by the `SPOTLIGHT_VOLUMETRICS` feature define (case `0xa8`, group `0x1f`,
`FUN_140d78c40`) (proven). `CSpotlightController` (registered class) is the gameplay-side driver that
positions/aims the light; the cone renderer is its GPU representation (registration proven; the
controller→cone binding inferred).

### 6. Cloud shadows

Cloud shadowing is produced by the volumetric-cloud system (documented sky-side) and consumed here as a
texture. `FUN_1401f6ef0` (the `VolumetricClouds` render object) binds `VolumetricCloudShadow` +
`VolumetricCloudShadow%s` shaders (proven); `FUN_1401ed750` renders `CirrusCloudsShadow` for the cirrus
layer (proven); `FUN_140210f90` allocates the `cloud_shadow_texture` resource (proven). The lighting side
samples it through the `CLOUDSHADOWS_0` / `CLOUDSHADOWS_1` shader defines (two cascades/layers of cloud
shadow, `FUN_140d78c40`) (proven). Detailed cloud raymarch/noise generation is out of scope — see the
sky/TOD doc.

### 7. Ambient occlusion & light occlusion planes

Screen-space AO is a **Scalable Ambient Obscurance (SAO)** implementation: `FUN_140d7a9e0` binds the full
chain `SSAO_SAO → SSAO_Blur/_5x5Blur → SSAO_Minify/_MipCopy/_ReconstructZ → SSAO_TemporalFilter →
SSAO_Apply / SSAO_ApplyIrradiance` (proven). The `_ReconstructZ`/`_Minify`/`_MipCopy` steps build a
depth mip-pyramid (the SAO hierarchical-Z technique), `_TemporalFilter` reprojects across frames, and
`_ApplyIrradiance` folds AO into the irradiance/ambient term rather than only darkening direct light
(proven names; SAO identification inferred from the exact pass set). A coarser volumetric AO exists as
`AO_VOLUMES` (case `0x69`, group `0xf`) (proven).

`CLightOcclusionPlaneObject` (registered) places author-defined planes that occlude light — used to stop
interior/exterior light bleed at portals/doorways (registration proven; use inferred).

## Data & config integration

- **Lights are RTPC entity objects.** `CDynamicLightObject`, `CDynamicLightFlasher`,
  `CEnvironmentLightingObject`, `CLightOcclusionPlaneObject` and `CSpotlightController` are all
  instantiated by the name→type factory `FUN_140f27f60` (the reflection registry, memory
  `[[name-cracking-engine]]`, keyed by lookup3 `hashlittle`). Their per-instance parameters (colour,
  intensity, radius, cone angle, falloff, cast-shadow flag, cascade config) are therefore **data-side** —
  carried in the entity's RTPC/ADF components, resolved by the property hashes in `[[rtpc-entity-assembly]]`
  (not present in this functions-only export). This is the natural next dig for exact light tunables.
- **Shader permutations are data-selected.** `FUN_14a37b810` / `FUN_14a4a1c90` choose shader names from
  mesh/material flag bits that come from the AMF material blob (see `rendering_graphics.md`), so which
  shadow/RSM/PreZ variant a mesh uses is driven by its material, not hardcoded (proven).
- **Cloud shadow & sky irradiance are cross-system inputs.** `cloud_shadow_texture` and
  `DynamicSkyIrradianceCube` are produced by the environment/TOD + volumetric-cloud systems and sampled by
  the lighting defines above — the coupling point between this doc and the sky doc.

## Notable constants / tunables

Read straight from the decomp (magnitudes are mostly data-side, so this list is deliberately short):

- **8 shadow cascades** (dynamic) + **8 static** — `SHADOW_0..7`, `STATIC_SHADOW_0..7` slot counts
  (`FUN_140d78c40`) (proven).
- **RSM = 3 MRTs, double-buffered** — `flux/normal/depth_reflective_shadow_map`, 6 target handles
  (`FUN_140cdc0e0`) (proven).
- **2 cloud-shadow layers** — `CLOUDSHADOWS_0/1` (`FUN_140d78c40`) (proven).
- **Deferred-light resolve modes** — direct (`Deferred_ClusteredLighting%s%s`) vs irradiance
  (`Deferred_ClusteredLightingIrradiance%s`), switched on `param_3==1` (`FUN_14a3479e0`) (proven).
- `cloud_shadow_texture` allocation carries float consts `0x471c4000` (=40000.0) and `0x3f800000` (=1.0)
  at obj+0x1c/+0x70 (`FUN_140210f90`) — likely a cloud-shadow world extent / scale (proven values;
  meaning speculative).
- Stage-group ids from `FUN_140d78c40`: shadows = `3`, reflection/RSM/GI = `2`, deferred lights = `0xe`,
  clouds/volumetric = `0x11`, sky/atmosphere = `0x10`, VFX lighting = `0x18`, spotlight volumetrics =
  `0x1f`, AO volumes = `0xf` (proven — these are the render-stage buckets each feature is scheduled into).

## Call-graph highlights

- `FUN_140d50550` (draw-bucket interner) ← `FUN_140d4e8b0` (×6 bucket registrations) — installs the
  `Shadow`/`ReflectiveShadowMap` buckets (proven).
- `FUN_140d78c40` (pass-name enumerator) ← `FUN_140d82640`, `FUN_140d756e0`, `FUN_140d7a540`,
  `FUN_140d80140`, `FUN_140d81540`, `FUN_14a55f500`, … — the many render-stage builders query it to name
  their pass (proven caller list).
- `FUN_14a3479e0` (clustered lighting) ← `thunk_FUN_14a3479e0` @0x140d0da70 (dispatch-table slot, the
  render-object vtable region alongside the render blocks) (proven).
- `FUN_14a4a1c90` (per-block bucket variant) ← `thunk_FUN_14a4a1c90` @0x140d2b9e0 (same dispatch region)
  (proven).
- `FUN_14a52edd0` (SpotLightCone) ← `thunk_FUN_14a52edd0` @0x140d59cb0 (proven).
- Shadow/depth shader binders (`FUN_14a388550` CarPaint, `FUN_140d05a90` Character) sit next to their
  render-block CB builders documented in `rendering_graphics.md` (proven).
- Light classes ← world-object registrar cluster (the `FUN_140f27f60(...)` calls near lines
  199258–199433, 754309–754493, 840725) — same registrar family (`FUN_14085fd00`) that installs all
  reflected objects (proven registration site).

## Open questions / lower-confidence

- **Exact light params** (colour/intensity/radius/cone/falloff/shadow-bias/cascade split distances) are
  data-side — resolve by decoding the `CDynamicLightObject` / `CSpotlightController` RTPC components
  against `[[rtpc-entity-assembly]]` (not in this export). (open)
- **Static vs dynamic cascade** split of `STATIC_SHADOW_*` vs `SHADOW_*` is inferred from naming; the
  code path that decides which cascade sources from the static set was not pinned down. (inferred)
- **Cluster/froxel binning** — the light-list build (grid dimensions, light assignment) is a compute pass;
  only the resolve pass names survive (`Deferred_ClusteredLighting::LocalResources`, hash `0x886e4712`).
  Bytecode + grid constants are walled. (walled)
- **Light-probe / dynamic-irradiance data flow** — `FUN_14a068f70` (`LightProbeDB*`) and `FUN_140cae100`
  (`DynamicSkyIrradianceCube`) are identified by string but their exact placement in the frame graph and
  their consumption by `Deferred_ClusteredLightingIrradiance` / `GLOBAL_ILLUMINATION` is inferred. (inferred)
- **`CEnvironmentLightingObject` / `CLightOcclusionPlaneObject`** roles are inferred from names + the
  registration; their runtime effect on the lighting math was not traced. (inferred)

## Appendix — decomp anchors

Exact strings + `FUN_`/`DAT_`/`PTR_` addresses used above (all re-verifiable in
`output/_ghidra_jc4/jc4_all_functions_decomp.txt`):

**Pass/feature name enumerator** `FUN_140d78c40` @0x140d78c40 — cases: `0x0d SKY_LIGHTING`(g0x10),
`0x19/0x1a CLOUDSHADOWS_0/1`(g0x11), `0x20-0x27 STATIC_SHADOW_0..7`(g3), `0x28-0x2f SHADOW_0..7`(g3),
`0x30 SHADOW_FILTERING`(g3), `0x31-0x33 SHADOW_REFLECTIVE_SUN_NEAR/FAR/CAMERA`(g2), `0x51
GLOBAL_ILLUMINATION`(g2), `0x59 VFX_LIGHTING`(g0x18), `0x69 AO_VOLUMES`(g0xf), `0x6d DEFERRED_LIGHTS`(g0xe),
`0xa8 SPOTLIGHT_VOLUMETRICS`(g0x1f), `0xb0 DEFERRED_LIGHTS_IRRADIANCE`(g0xe).

**Draw-bucket interner** `FUN_140d50550` @0x140d50550 — `{Shadow, ReflectiveShadowMap, PreZ, Velocity,
Vegetation, Transparancy}`.

**Shader-variant name builder** `FUN_14a37b810` @0x14a37b810 — base `{Shadow, RSM, PreZ, PreZVelocity,
Glint, Outline}` + suffixes.

**Per-block bucket→shader** `FUN_14a4a1c90` @0x14a4a1c90 (thunk @0x140d2b9e0) — `{Shadow, RSM, RSM,
PreZ}`.

**RSM targets** `FUN_140cdc0e0` @0x140cdc0e0 — `flux_reflective_shadow_map`,
`normal_reflective_shadow_map`, `depth_reflective_shadow_map` (×2).

**Clustered deferred lighting** `FUN_14a3479e0` @0x14a3479e0 (thunk @0x140d0da70) —
`PTR_s_Deferred_ClusteredLighting_141eb96f8`, `Deferred_ClusteredLighting%s%s`,
`Deferred_ClusteredLightingIrradiance%s`, `Deferred_PassThrough`; also
`FUN_14a4732d0` `"Deferred_ClusteredLighting::LocalResources"`, `FUN_14f14de69` hash `0x886e4712`.

**SSAO (SAO)** `FUN_140d7a9e0` @0x140d7a9e0 — `SSAO_SAO/_Apply/_ApplyIrradiance/_Blur/_Minify/_MipCopy/
_ReconstructZ/_TemporalFilter/_5x5Blur`.

**Spotlight cone** `FUN_14a52edd0` @0x14a52edd0, `FUN_14a52c640` @0x14a52c640, `FUN_14a5369c0`
@0x14a5369c0 — `SpotLightCone`, `SpotLightConeInside`, `SpotLightCone::ResourceTable`.

**Per-block shadow shaders** `FUN_14a388550` @0x14a388550 (`CarPaintShadow{,_Deform,_Skinned}`,
`CarLightShadowCloakTransition`); `FUN_140d05a90` @0x140d05a90 (`CharacterDepth`,
`CharacterDepthShadowCloakTransition`).

**Cloud shadow** `FUN_1401f6ef0` @0x1401f6ef0 (`VolumetricCloudShadow`), `FUN_1401ed750` @0x1401ed750
(`CirrusCloudsShadow`), `FUN_140210f90` @0x140210f90 (`cloud_shadow_texture`).

**Probes / sky irradiance** `FUN_14a068f70` @0x14a068f70 (`LightProbeDBKeys`,
`LightProbeManagementEntries`, `LightProbeDBItems`); `FUN_140cae100` @0x140cae100
(`DynamicSkyIrradianceCube`, `IrradianceTarget`).

**Light world-objects** (name→type factory `FUN_140f27f60`): `CDynamicLightObject` `DAT_142cb0a68`
(l.199283), `CDynamicLightFlasher` `DAT_142cb0a70` (l.199258), `CEnvironmentLightingObject` `DAT_142cb0ab4`
(l.199333), `CLightOcclusionPlaneObject` `DAT_142cb0a78` (l.199433), `CSpotlightController` `DAT_142cb8e5c`
(l.840725), `CSpotlightWeaponComponent` `DAT_142cb83c4` (l.754493), `CLightWeaponComponent` `DAT_142cb8378`
(l.754309).

**Shader-stage binders re-used** (from `rendering_graphics.md`): `thunk_FUN_14aa0e400` (VS/CS),
`thunk_FUN_14aa0d480` (PS), `thunk_FUN_14aa0d100` (CS), `thunk_FUN_14aadc440` (name+suffix format).

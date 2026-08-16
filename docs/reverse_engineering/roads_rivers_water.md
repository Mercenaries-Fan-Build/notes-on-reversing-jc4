# Roads, Rivers, Water & AI Road-Driving — the world's traversable surfaces

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative). "proven" = read
directly from `output/_ghidra_jc4/jc4_all_functions_decomp.txt` with the `FUN_`/`DAT_`/string cited.

> Scope note (methodology caveat, proven): this is a **functions-only** decomp export. What is recoverable
> here is the **registration skeleton** (which managers/components/passes exist and what they are named), the
> **asset the code loads**, the **render-pass ordering**, the **console/command surface**, and the
> **named force channels**. What is *not* recoverable is the per-tick body of each manager (river flow
> integration, on-road snapping math, WaveWorks spectrum params) — those live behind data-section vtables and
> in RTPC/ADF/`.watertunec` tuning data. Magnitudes are tagged accordingly.

---

## Overview

Just Cause 4 splits "traversable surface" into three cooperating subsystems, each a singleton **manager**
registered in the game's manager registrar `FUN_148f960c0` (proven — all three appear as rows there):

- **`CRoadManager`** + **`COnRoadService`** — the road network. Roads are authored as **splines**, baked into
  a **road graph** asset (`roadgraph.roadgraphc`) with a companion **route** table (`routes.routec`), and also
  rasterized into terrain as **signed-distance-field primitives** (`rcRoadSdfPrimitive`) for rendering. The
  `OnRoad` service is the query layer (snap-to-road / "am I on a road") that AI driving and the
  `enablestayonroad` gameplay behavior consume.
- **`CRiverManager`** — rivers, authored via `CRiverObject` / `CRiverControllerObject` / `CRiverPalette`,
  rendered through dedicated `RIVER_LIGHT_SAMPLES` and `RIVERS_RECONSTRUCT` passes.
- **Water / ocean** — a **NVIDIA WaveWorks + Gerstner** surface simulation (`terrain/water/nvwaveworks_mod_ra…`
  and `terrain/water/gerstner_mod_rawc`, tuned by `settings/water_tuning.watertunec`) drawn by a large family
  of `WATER_*` / `UNDERWATER_*` render passes, with buoyancy/drag applied to floating bodies through the
  physics engine's **named force channels** (`HydroStaticForce`, `WaterDragForce`, `PlaningForce`, …).

AI vehicle driving is the **consumer** of the road graph: the traffic system config `ai/traffic.aisystunec`
is loaded alongside the graph, traffic entities (`CAITrafficSpline`, `CAiRoadGraphFilter`, `CAiTrafficObstacle`,
`CTrafficLight`) drive along it, and character/vehicle pathing is provided by **Havok AI 2016.1.0.6**
(`hkaiWorld` + navmesh), with **dynamic navmesh cutting** (`CDynamicNavMeshCutter`) so destruction re-opens
paths. (proven: all names present as string constants; mechanism inferred.)

---

## Key classes, managers & functions

| Name (string constant) | FUN_ that references it | Role |
|---|---|---|
| `CRoadManager` | registrar `FUN_148f960c0` (also `FUN_146cd0bea`); vtable ptr `0x41cf9520` / `PTR_LAB_141d90628` | Road-network singleton manager (proven it's registered; body walled) |
| `COnRoadService` | registrar `FUN_148f960c0` (via `thunk_FUN_148f6e780`) | On-road query/snap service (proven registered) |
| `CRiverManager` | registrar `FUN_148f960c0`; vtable ptr `0x41d8f438` / `PTR_LAB_141d905a8` | River singleton manager (proven registered) |
| road-graph loader | `FUN_140467950` | Loads `packages/main/roadgraph.roadgraphc` + `packages/main/routes.routec` (proven) |
| traffic-system loader | `FUN_1403c89a0` | Loads `ai/traffic.aisystunec` (proven) |
| road console commands | `FUN_140ae98d0` | Registers `teleporttoroad`, `enablestayonroad`, `disablestayonroad`, `offroad_{exclusion,inclusion}_{enter,exit,reset}` (proven) |
| `rcRoadSdfPrimitive` | `FUN_140fba9c0` (`"TtrcRoadSdfPrimitive"`) | Road baked as terrain SDF primitive (proven) |
| render-pass name table | `FUN_140d78c40` | Enum→name+queue table containing every `ROAD_*`, `WATER_*`, `UNDERWATER_*`, `RIVER*` pass (proven) |
| WaveWorks/Gerstner loader | `FUN_1401a92f0` | Loads `terrain/water/nvwaveworks_mod_ra…`, `terrain/water/gerstner_mod_rawc`, `settings/water_tuning.watertunec` (proven) |
| ocean surface draw | `FUN_1401b3c40` | Calls `GFSDK_WaveWorks_Quadtree_Draw` (proven) |
| WaveWorks displacement/readback | `FUN_1401db560` | `"WaveWorks DisplacementMapTexture"`, `"WaveWorks Readback"` (proven) |
| foam generation | `FUN_1401dc2d0` | `NV_FoamGeneration` / `NV_FoamGenerationCompute::LocalResourceTable` (proven) |
| water foam mesh | `FUN_1401bc040` | `WaterFoam`, `WaterPaintFoamVertices`, `WaterWakeFoam` (proven) |
| Nv wave constant buffers | `FUN_1479f7340` | `NvWavePSCB`, `NvWaveVDSCB` (proven) |
| physics force-channel registry | `FUN_147759a20` | Registers `HydroStaticForce`, `WaterDragForce/Torque`, `PlaningForce/Torque`, `AirBuoyancy*`, `Water{Drag,Friction}Force/Torque`, `GenericObjectHitWaterSurfaceForce/…DragForce`, `OverallWaterDragForce/Torque` (proven) |
| spline sampler | `FUN_1414a89d0` | `SampleSpline` / `SampleSplineChunk` (proven) |

**Component classes** (registered by string name through the reflection factory `FUN_140f27f60` — the
shared name→type registry from `README.md`; the value is the lookup3 hash, proven registered):

- Road/traffic AI: `CAITrafficSpline` (0x10), `CAiRoadGraphFilter` (0x12), `CAiTrafficObstacle` (0x12),
  `CTrafficLight` (0xd), `CSplineFollowObject` (0x13).
- Navmesh authoring: `CDynamicNavMeshCutter` (0x15), `CNavMeshExcluder` (0x10).
- Generic splines (roads, rails, cables share this): `CSplineObject` (0xd), `CExtrudeSplineObject` (0x14),
  `CAnimSpline` (0xb), `CSplineTrigger` (0xe), `CSplineTriggerPoint` (0x13), plus entity-spline set
  `CEntitySpline` (0xd), `CEntitySplineSegment` (0x14), `CEntitySplinePoint` (0x12), `CEntitySplineOccupant`
  (0x15), and rail `CTrainRoute` (0xb) consuming `packages/main/routes.routec`.
- River/water objects: `CRiverObject` (0xc), `CRiverControllerObject` (0x16), `CRiverPalette` (0xd),
  `CWaterBox` (9), `CWaterEmitterObject` (0x13), `CWaterExtensions` (0x10). Behavior gate:
  `CWaterDepthCondition` (`DAT_142cb4274`, via `thunk_FUN_14aadec10`). Effects: `StaticWaterEffect` /
  `cWaterEffect` (`FUN_1400d07c0` at line ~877284).

---

## How it works (from the decomp)

### 1. The road graph & routes

`FUN_140467950` is the road-system constructor: it zero-inits a large object, installs two allocator handles
(`thunk_FUN_14aae64e0`), constructs a 3-element sub-array via `_eh_vector_constructor_iterator_`, then — when
`*(int*)(param_1+4)==0` (not-yet-loaded guard) — asynchronously loads the two shipped assets (proven):

```
thunk_FUN_14ad338e0(local_88,"packages/main/roadgraph.roadgraphc",4,param_1);   // road graph
thunk_FUN_14ad338e0(local_48,"packages/main/routes.routec",4,param_1);          // named routes
```

The `.roadgraphc` / `.routec` extensions are **ADF** typed containers (the backbone data format — see
`docs/formats/adf.md`). The graph is therefore a reflected node/edge structure decoded through the same ADF
path as every other config; the road **spline geometry** lives inside it. (proven that the assets are loaded;
their internal layout is inferred — decode is a data-side task, not visible in the functions export.)

`FUN_140467950`'s only caller is `FUN_1403c89a0`, which *also* loads the AI traffic config
`ai/traffic.aisystunec` (`FUN_1403c9480(param_1,*param_2,"ai/traffic.aisystunec",param_1+0x60)`). So the road
graph, the routes, and the traffic ruleset are brought up together as one subsystem init. (proven)

### 2. On-road service & the "stay on road" gameplay surface

The road command surface is registered in `FUN_140ae98d0` (a big command-registration function using
`thunk_FUN_14762d8a0(slot,"name",0xff,1)`) (proven):

- `teleporttoroad` — snap an entity onto the nearest road (implies `COnRoadService` exposes a nearest-road
  query; inferred).
- `enablestayonroad` / `disablestayonroad` — toggle the AI/assist behavior that keeps a driven vehicle on the
  graph.
- `offroad_exclusion_enter/exit/reset` and `offroad_inclusion_enter/exit/reset` — author volumes that force
  the driving solver off/onto roads in specific regions.

These are the *consumer* hooks: `COnRoadService` answers "where is the road / snap me to it", and the driving
behavior uses that answer. The snapping/routing math itself is behind the service vtable and not in the
functions body. (mechanism inferred; the command names are proven.)

### 3. AI vehicle driving on the graph

Traffic/driving is built from reflected components (all proven registered via `FUN_140f27f60`):

- **`CAITrafficSpline`** — a drivable lane spline; ambient traffic follows these.
- **`CAiRoadGraphFilter`** — filters/weights the road graph for a given AI query (e.g. exclude a road type).
- **`CAiTrafficObstacle`** — dynamic obstacle registered against the graph so traffic routes around it.
- **`CTrafficLight`** — intersection signalling.
- **`CSplineFollowObject`** / `SampleSpline`/`SampleSplineChunk` (`FUN_1414a89d0`) — the generic
  follow-a-spline evaluator that path-following (traffic and scripted convoys) rides on.

Character/on-foot and vehicle **pathfinding** is **Havok AI 2016.1.0.6** (proven from embedded source paths
`…\havok_ai\havok_ai-2016.1.0.6.0.1.1560996\source\ai\pathfinding\…`). Recovered `hkai*` classes include
`hkaiWorld`, `hkaiEdgeFollowingBehavior`, `hkaiGateFollowingBehavior`, `hkaiSingleCharacterBehavior`,
`hkaiNavMeshUtils`, `hkaiNavMeshGenerationUtils`, `hkaiNavMeshPruningUtils`, `hkaiNavMeshErosion`,
`hkaiNavMeshClearanceCacheManager`, `hkaiTraversalAnalysis`, `hkaiGatePathUtil`, `hkaiUserEdgeUtils`,
`hkaiFindPointInPolygon`, `hkaiSplitGenerationUtils`, `hkaiPhysicsWorldListener`, `hkaiPhysicsGeometryConverter`,
`hkaiWorld_silhouette`. Navmesh streaming is exposed as commands `navmesh_load` / `navmesh_unload` /
`navmesh_unloadall` (line ~354702). **Dynamic navmesh cutting** — `CDynamicNavMeshCutter` (Havok
`hkaiNavMeshInstanceCutter` / `hkaiNavMeshCutConfiguration`) and `CNavMeshExcluder` — lets destruction and
placed objects re-carve walkable/drivable area at runtime; the callback `dynamicNavMeshModifiedCallback` fires
on change (lines ~2230267/2230482). (proven middleware + names; per-frame path math walled behind Havok.)

> Note the split: **roads = spline/road-graph** (`CRoadManager`, `roadgraphc`) for wheeled traffic; **navmesh
> = Havok AI** for on-foot / free-roaming agents. They are distinct graphs. (inferred from the two separate
> subsystems.)

### 4. Road rendering

Roads are **rasterized as signed-distance-field primitives** (`rcRoadSdfPrimitive` / `TtrcRoadSdfPrimitive`,
`FUN_140fba9c0`) so they blend into terrain rather than being separate meshes (proven the type exists;
SDF-into-terrain is the standard Apex approach — inferred). Their draw passes in the pass table
`FUN_140d78c40` (proven, with the queue-bucket id in parentheses): `ROAD_JUNCTION_OPAQUE` (0x4e, q=10),
`ROAD_LAYERS` (0x4f, q=10), `ROAD_JUNCTION` (0x50, q=10), `SCREEN_SPACE_ROAD_DECALS` (0x63, q=0x1b), and
`SKIDMARKS` (0x8d, q=0x19) for tire marks.

### 5. Water / ocean simulation — WaveWorks + Gerstner

`FUN_1401a92f0` brings up the water surface (proven):

```
thunk_FUN_147990320(PTR_s_terrain_water_nvwaveworks_mod_ra_…, param_1,            0x100000, …);
thunk_FUN_147990320(PTR_s_terrain_water_gerstner_mod_rawc_…,  param_1+0x100000,   0x100000, …);
… uVar4 = thunk_FUN_14aadec10(".watertunec");
… PTR_s_settings_water_tuning_watertunec_…   // settings/water_tuning.watertunec
```

Two **1 MB (0x100000)** data blocks back the surface — a **NVIDIA WaveWorks** FFT-ocean modifier and a
**Gerstner-wave** modifier — with the water look/behavior driven by the `settings/water_tuning.watertunec` ADF.
The two blocks are byte-filled to `0xFF` before load (init/clear). (proven the two modifiers + tuning file
load; the FFT spectrum and Gerstner coefficients are data-side.)

The ocean is drawn as an adaptive quadtree: `FUN_1401b3c40` reaches
`GFSDK_WaveWorks_Quadtree_Draw` (proven — the GameWorks WaveWorks quadtree API). Displacement + CPU readback
(for buoyancy sampling) go through `FUN_1401db560` (`"WaveWorks DisplacementMapTexture"`,
`"WaveWorks Readback"`) — the readback is what lets gameplay know surface height at an arbitrary point
(inferred). Mesh buffers `NvWaveWorksMeshVB` / `NvWaveWorksMeshIB` and spline-shoreline buffers
`NvWaterSplineVertices` / `NvWaterSplineIndices` (line ~119430) feed the draw; constant buffers are
`NvWavePSCB` / `NvWaveVDSCB` (`FUN_1479f7340`). (all proven.)

**Foam** is a separate compute stage: `NV_FoamGeneration` / `NV_FoamGenerationCompute::LocalResourceTable`
(`FUN_1401dc2d0`), plus artist foam `WaterPaintFoam` (`FUN_140c91a90`), `WaterPaintFoamVertices`, wake foam
`WaterWakeFoam` (`FUN_1401bc040`, which registers the `WaterFoam` node) and `WaterFoamTexture`. (proven.)

### 6. Rivers

Rivers are authored as `CRiverObject` (spline-based, `CRiverControllerObject` steering flow, `CRiverPalette`
for look) and rendered by two dedicated passes in `FUN_140d78c40`: `RIVER_LIGHT_SAMPLES` (0x6e, q=0x1a) and
`RIVERS_RECONSTRUCT` (0x89, q=0x1a) (proven). The "reconstruct" pass name implies rivers are screen-space
reconstructed/composited rather than drawn as the WaveWorks ocean quadtree — i.e. rivers and ocean are
**different renderers** sharing the water shading tail (`POST_WATER`, `WATER_GODRAYS`). (name proven;
reconstruction mechanism inferred.)

### 7. Water shading tail, underwater & wetness

Full water/underwater pass ordering from `FUN_140d78c40` (proven; queue bucket in parens):

| Pass | case | queue | role |
|---|---|---|---|
| `WATER_CS_PRE` | 0x35 | 0xb | compute pre-pass (sim update) |
| `WATER_WAKES_PRE` | 0x36 | 0xb | boat/vehicle wake injection |
| `WATER_FOAM_PRE` | 0x37 | 0xb | foam pre-pass |
| `WATER_DISPLACEMENT_PRE` | 0x38 | 0xb | apply WaveWorks displacement |
| `REFLECTIVE_WATER_PLANES` / `RP_REFLECTIVE_WATER_PLANES` | 0x65 | 1 | planar reflections |
| `RIVER_LIGHT_SAMPLES` | 0x6e | 0x1a | river lighting |
| `RIVERS_RECONSTRUCT` | 0x89 | 0x1a | river surface reconstruct |
| `WATER` | 0x8a | 0xb | main opaque water |
| `VIRTUAL_WATER_PLANE` | 0x8b | 0xb | far/infinite water plane |
| `POST_WATER` | 0x8c | 0xb | post-water composite |
| `MASK_WATER` | 0x98 | 0xb | water stencil/mask |
| `WATER_POST_CLOUDS` | 0x95 | 0xb | water re-composite after clouds |
| `WATER_GODRAYS` | 0x9d | 0x10 | underwater god-rays |
| `UNDERWATER_FOG_GRADIENT` | 0x5c | 0x10 | `UnderwaterFogGradientShader` (`FUN_140620…`/`FUN_1401511…`) |
| `UNDERWATER_VEGETATION` / `…_TRANSPARENT` | 0x57 / 0x87 | 7 | submerged foliage |
| `UNDERWATER_CLOUDS` | 0x86 | 0x11 | underwater cloud/volume look |

Supporting shaders/textures (proven strings): `WaterWaves`, `WaterDisplacementOverride`,
`UnderwaterFogGradient`, and the texture set `textures/water/{foam_merge, wave_foam_dif, wave_dif, wave_mask,
water_bump, water_godrays_dif, water_godrays_alphamask, water_tint_dif_alpha, water_mod}`,
`reflection_proxy_water_plane_texture`, and character/surface **wetness** via
`textures/noise/wetness_noise_tile.ddsc` (line ~1552647). Water-shading may share the weather GPU fluid
pipeline — see `wind_and_weather.md` (`FUN_1401f7f70`) — the WaveWorks/foam compute passes sit in the same
render-queue buckets as the weather volumetrics. (cross-ref; sharing inferred.)

### 8. Buoyancy & water physics — the named-force bus

Floating/submerged bodies are pushed by the physics engine's **named force channels**, all registered in one
table `FUN_147759a20` via `thunk_FUN_147728150("<name>")` (proven). The water-relevant channels are:

- `HydroStaticForce` — **buoyancy** (Archimedes lift on submerged volume).
- `GenericObjectHitWaterSurfaceForce` — impact/splash force when a body enters water.
- `GenericObjectWaterDragForce`, `OverallWaterDragForce`, `OverallWaterDragTorque`, `WaterDragForce`,
  `WaterDragTorque`, `WaterFrictionForce`, `WaterFrictionTorque` — drag/friction resisting motion in water.
- `PlaningForce`, `PlaningTorque` — boat planing lift at speed.
- Lighter-than-air (blimps/balloons): `AirBuoyancy`, `AirBuoyancyForce`,
  `AirBuoyancyExtraWhenDualTethered` (the last ties into the grappling dual-tether system — see
  `grappling_hook.md`).

This is the same **external-force bus** documented in `physics_constraints.md`: the executable holds the
*channel names and application path*, while the **magnitudes** are data-side (per-vehicle/creature RTPC/ADF).
Surface height for these forces is sampled from the WaveWorks readback (§5). (channels proven; values +
sampling wiring inferred.)

---

## Data & config integration

- `packages/main/roadgraph.roadgraphc` — ADF road graph (nodes/edges/splines) consumed by `CRoadManager`.
- `packages/main/routes.routec` — ADF named routes (also feeds `CTrainRoute`).
- `ai/traffic.aisystunec` — ADF AI traffic ruleset (`.aisystunec` = AI-system tuning container).
- `settings/water_tuning.watertunec` — ADF water tuning (WaveWorks/Gerstner params, drag, look).
- `terrain/water/nvwaveworks_mod_ra…c`, `terrain/water/gerstner_mod_rawc` — 1 MB surface-sim modifier blobs.
- Havok navmesh assets streamed via `navmesh_load` (per-tile; Havok AI tagfile, cf. `[[havok-hct-2018]]`).
- Component classes above map to RTPC entity components (class = lookup3 hash of the name, per
  `[[rtpc-entity-assembly]]` / `docs/formats/name_hash.md`); force magnitudes live in the same RTPC/ADF data.

All of these decode through the already-cracked **ADF** pipeline (`jc4_adf`, `docs/formats/adf.md`) — decoding
`roadgraphc` / `routec` / `watertunec` is the concrete next data-side step to recover the actual graph layout
and water magnitudes. (proven the files load; contents pending ADF decode.)

---

## Notable constants / tunables (read straight from the decomp)

- Water sim blobs: **0x100000 (1 MB) each**, filled `0xFF` pre-load (`FUN_1401a92f0`). (proven)
- Road render queue bucket = **10** (`ROAD_*` passes); water = **0xb**; underwater fog/godrays = **0x10**;
  rivers = **0x1a** (`FUN_140d78c40`). (proven)
- Road-graph loader load-priority argument = **4** for both `roadgraphc` and `routec` (`FUN_140467950`).
  (proven)
- Numeric force magnitudes (buoyancy/drag/planing) are **not in the functions export** — data-side. (proven
  absence)

---

## Call-graph highlights

- `FUN_1403c89a0` → `FUN_140467950` (road graph + routes) and → `FUN_1403c9480(…,"ai/traffic.aisystunec",…)`:
  the combined road+traffic subsystem init. (proven)
- `FUN_148f960c0` (manager registrar) installs `CRoadManager`, `COnRoadService`, `CRiverManager` singletons
  (alongside `CWindTunnelManager`, `CTopographicalWind`, `CCoverageManager` — cross-ref `wind_and_weather.md`).
  (proven)
- `FUN_140d78c40` (pass-name table) is called by the render-pass scheduler family
  (`FUN_140d82640`, `FUN_140d80140`, `FUN_140d81540`, `FUN_14a55e050`, `FUN_14a56b020`, …) — every water/road/
  river pass id resolves to its name here. (proven from `callers=[…]`)
- `FUN_1401b3c40` → `GFSDK_WaveWorks_Quadtree_Draw`; foam `FUN_1401dc2d0`, displacement `FUN_1401db560`,
  CB `FUN_1479f7340` form the WaveWorks draw cluster. (proven)
- `FUN_147759a20` force registry underpins water/buoyancy application (physics side). (proven)

---

## Open questions / lower-confidence

- **Road-graph internal layout** (node/edge/spline structs, lane counts, junction encoding) — needs ADF decode
  of `roadgraphc`/`routec`. (unresolved)
- **On-road snapping/routing math** inside `COnRoadService` — behind the service vtable; not in this export.
  (walled)
- **How AI vehicles are steered** frame-to-frame along `CAITrafficSpline` (pure-pursuit? PID?) — the tick body
  is data/vtable-driven and not visible. Only the component/command surface is proven. (walled)
- **River flow simulation** (does `CRiverControllerObject` push a velocity field into the water drag forces?) —
  plausible but unproven; `RIVERS_RECONSTRUCT` is a *render* pass, not the sim. (speculative)
- **Ocean↔weather GPU fluid sharing** — the WaveWorks compute passes and weather volumetrics occupy adjacent
  queue buckets; whether they share the `FUN_1401f7f70` fluid solver is inferred, not proven.
- **Whether roads feed navmesh** — roads (spline graph) and Havok navmesh look independent; a `CAiRoadGraphFilter`
  bridge exists but the coupling is unconfirmed. (inferred)

---

## Appendix — decomp anchors (re-verifiable)

Managers / registrar:
- `FUN_148f960c0` — manager registrar; strings `"CRoadManager"` (vtable `0x41cf9520`), `"CRiverManager"`
  (`0x41d8f438`), `"COnRoadService"`, `"CWindTunnelManager"`, `"CTopographicalWind"`, `"CCoverageManager"`.
- `FUN_146cd0bea` — 58-byte mirror referencing the same manager strings.

Road graph / traffic / commands:
- `FUN_140467950` — `"packages/main/roadgraph.roadgraphc"`, `"packages/main/routes.routec"`.
- `FUN_1403c89a0` — `"ai/traffic.aisystunec"` (via `FUN_1403c9480`).
- `FUN_140ae98d0` — `"teleporttoroad"`, `"enablestayonroad"`, `"disablestayonroad"`,
  `"offroad_exclusion_{enter,exit,reset}"`, `"offroad_inclusion_{enter,exit,reset}"`.
- `FUN_140419bc0` / `FUN_140860…` region — `FUN_140f27f60("CAITrafficSpline"|"CAiRoadGraphFilter"|
  "CAiTrafficObstacle"|"CDynamicNavMeshCutter"|"CNavMeshExcluder"|"CSplineFollowObject"|"CTrafficLight"|
  "CEntitySpline*"|"CTrainRoute")`.
- Navmesh commands ~line 354702: `"navmesh_load"`, `"navmesh_unload"`, `"navmesh_unloadall"`.
- Havok AI source paths: `…\havok_ai-2016.1.0.6.0.1.1560996\source\ai\pathfinding\…`; `hkaiWorld`,
  `hkaiEdgeFollowingBehavior`, `hkaiGateFollowingBehavior`, `hkaiSingleCharacterBehavior`, `hkaiNavMesh*`,
  `hkaiTraversalAnalysis`, `dynamicNavMeshModifiedCallback` (~2230267).
- `FUN_140fba9c0` — `"rcRoadSdfPrimitive"` / `"TtrcRoadSdfPrimitive"`.
- `FUN_1414a89d0` — `"SampleSpline"`, `"SampleSplineChunk"`.

Render passes:
- `FUN_140d78c40` — the enum→name table: `ROAD_JUNCTION_OPAQUE`/`ROAD_LAYERS`/`ROAD_JUNCTION`,
  `SCREEN_SPACE_ROAD_DECALS`, `SKIDMARKS`, `WATER_CS_PRE`/`WATER_WAKES_PRE`/`WATER_FOAM_PRE`/
  `WATER_DISPLACEMENT_PRE`, `WATER`/`VIRTUAL_WATER_PLANE`/`POST_WATER`/`MASK_WATER`/`WATER_POST_CLOUDS`/
  `WATER_GODRAYS`, `REFLECTIVE_WATER_PLANES`, `RIVER_LIGHT_SAMPLES`, `RIVERS_RECONSTRUCT`,
  `UNDERWATER_{VEGETATION,VEGETATION_TRANSPARENT,FOG_GRADIENT,CLOUDS}`.

Water sim / render:
- `FUN_1401a92f0` — `terrain/water/nvwaveworks_mod_ra…`, `terrain/water/gerstner_mod_rawc`,
  `settings/water_tuning.watertunec`, `.watertunec`.
- `FUN_1401b3c40` — `"GFSDK_WaveWorks_Quadtree_Draw"`.
- `FUN_1401db560` — `"WaveWorks DisplacementMapTexture"`, `"WaveWorks Readback"`.
- `FUN_1401dc2d0` — `"NV_FoamGeneration"`, `"NV_FoamGenerationCompute::LocalResourceTable"`.
- `FUN_1401bc040` — `"WaterFoam"`, `"WaterPaintFoamVertices"`, `"WaterWakeFoam"`; `FUN_140c91a90`
  `"WaterPaintFoam"`.
- `FUN_1479f7340` — `"NvWavePSCB"`, `"NvWaveVDSCB"`; `"NvWaveWorksMeshVB/IB"`, `"NvWaterSplineVertices/Indices"`.
- Textures: `textures/water/foam_merge.ddsc` (~124093/4609780), `wave_foam_dif`, `wave_dif`, `wave_mask`,
  `water_bump`, `water_godrays_dif`, `water_godrays_alphamask`, `water_tint_dif_alpha`, `water_mod`,
  `reflection_proxy_water_plane_texture`, `textures/noise/wetness_noise_tile.ddsc` (~1552647).

River / water objects:
- `FUN_1402abfd0` region — `FUN_140f27f60("CRiverObject"|"CRiverControllerObject"|"CRiverPalette"|
  "CWaterEmitterObject")`.
- `FUN_140846a70` (component registrar `FUN_14085fd00`) — `"CWaterBox"`, `"CWaterExtensions"`.
- `CWaterDepthCondition` — `DAT_142cb4274` via `thunk_FUN_14aadec10` (~506405).
- `StaticWaterEffect` / `cWaterEffect` — `FUN_1400d07c0` (~877284).

Physics force channels:
- `FUN_147759a20` — `HydroStaticForce`, `GenericObjectHitWaterSurfaceForce`, `GenericObjectWaterDragForce`,
  `OverallWaterDragForce`/`Torque`, `WaterDragForce`/`Torque`, `WaterFrictionForce`/`Torque`,
  `PlaningForce`/`Torque`, `AirBuoyancy`, `AirBuoyancyForce`, `AirBuoyancyExtraWhenDualTethered`.

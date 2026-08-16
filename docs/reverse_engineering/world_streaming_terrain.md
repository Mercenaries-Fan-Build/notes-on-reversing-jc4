# World Streaming, Terrain & Discovery — how JC4 builds, streams and reveals its open world

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

> Scope: how the world is divided into streamable units and loaded from TAB/ARC, how terrain/landscape is
> represented and LOD'd on the GPU, how biomes are resolved, and how map discovery / % explored is tracked.
> **Roads, rivers, water, population and traffic spawning are separate docs** — referenced here, not duplicated.
> The retail world archive format (TAB/ARC v2) is already cracked; see `docs/formats/tab_arc_v2.md` and memory
> `[[tab-v2-cracked]]`. This doc reconstructs the *code* that consumes it.

## Overview

JC4 (Avalanche **Apex** engine) streams its world out of the same **TAB/ARC v2** archives we already cracked:
a `.tab` index table + `.arc` block file per archive, entries Oodle-compressed. The engine-side owner is the
reflected **`CResourceLoaderManager`** subsystem (type-id hash `0x41cf8e00`), wired in at boot by the system
registrar `FUN_148f958e0`. The concrete reader opens `<name>.tab`/`<name>.arc`, validates the `'TAB\0'` magic,
detects **version 2**, walks a **12-byte entry table**, and feeds a **multi-codec block decompressor** whose
codec 4 is the retail **`OodleLZ_Decompress`** path (`FUN_14ad55350`). Loads can be synchronous or dispatched as
async **`decode_fragment`** tasks. (proven — FUN_140f90c80, FUN_140f91f00, FUN_14ad55350, OodleLZ_Decompress
@ line 4966763)

The world itself is a set of singleton **managers** registered together in one big table (`FUN_146cd0bea` /
`FUN_148f960c0`): `CLandscapeManager` (terrain), `CBiomeManager` (climate/biome regions), `CGameObjectManager`
+ `CSpawnSystem` (the spawn side streaming feeds), `CCoverageManager` (terrain ground-coverage overlay),
`CDiscoveryManager` + `CCoverageManager`-adjacent discovery entities (map reveal / % explored), plus
`CProjectEffectToTerrain`, `CModelInstanceManager`, `CGameWorld`. Each row carries its **type-id hash** (the
same lookup3 `hashlittle` identity used for asset paths, memory `[[name-hash-cracked]]`). (proven — literal
hash stores in FUN_146cd0bea)

**Terrain is fully GPU-driven and adaptive.** The landscape init (`FUN_1401bef80`) loads `settings/terrain.bin`,
`settings/landscape_effects.bin` and the world height/patch file **`terrain/jc3/jc3.world`** (the internal path
still says *jc3* — an Apex-lineage tell that JC4's world tech is a later JC3 revision, memory
`[[lineage-apex-engine]]`), and opens Havok **`terrain_physics_port`** / `vegetation_physics_port` collision
streams. Rendering runs a quadtree/clipmap **patch → subdivide → tessellate → morph** compute pipeline
(`FUN_1401a6d70`) backed by a **virtual-texture terrain cache** (`terrain_indirection_map` + color/material/ssdf
atlases, `FUN_140f108b0`). (proven — those functions/strings)

Map **discovery** is a `std::vector<uint>` of discovered location IDs held by `CDiscoveryManager`, serialized to
the save (`FUN_14088e490`); **% explored** is `discovered*100/total` (`FUN_140e50630`), gating the
`ach_discover_half` / `ach_discover_everything` achievements. (proven)

## Key classes & functions

### Streaming / resource loader
| Class / string | FUN_ (role) | One-line role |
|---|---|---|
| `CResourceLoaderManager` | reg `FUN_148f958e0` (hash `0x41cf8e00`); typename `FUN_147e9b280`; instance pool `FUN_140309b80` | The engine resource-loader subsystem |
| TAB/ARC opener | `FUN_140f90c80` @0x140f90c80 (caller `FUN_140f901d0` mount) | Opens `%s/%s%d.tab`+`.arc`, checks `'TAB\0'`, detects v2, reads 12-byte entry table |
| leading-slash variant | `FUN_140f93aa0` @0x140f93aa0 | Same, path `/%s/%s%d.tab` |
| entry read + decompress loop | `FUN_140f91f00` @0x140f91f00 (via `FUN_14ad432d0`) | Per-entry sync decode or async `decode_fragment` task |
| block codec dispatcher | `FUN_14ad55350` @0x14ad55350 | 0=store 1=zlib("1.2.8") 2=lz4 3=zstd **4=OodleLZ_Decompress** |
| zlib inflate core | `FUN_140f5eac0` @0x140f5eac0 | Used by codec-1 |
| AAF container sniffer | `FUN_14ad36520` @0x14ad36520 | `strncmp(hdr,"AAF",4)`, version==1 |
| `.arc` block-path resolvers | `FUN_14ad3f430`, `FUN_14725ef12` | Build `%s/%s%u.arc` |
| `.resourcebundle` handler | `FUN_147ea7ee0` @0x147ea7ee0 | Resource-catalog / bundle registration |
| `MatchStreamingFacesTask` | `FUN_1415730a0` @0x1415730a0 (→ `FUN_141543ef0`) | Profiled mesh/LOD-face streaming task |

### Terrain / landscape
| String | FUN_ (role) | One-line role |
|---|---|---|
| `CLandscapeManager` | reg hash `0x41cd4e80` (FUN_146cd0bea @ line 3209145) | Terrain/landscape singleton manager |
| terrain-system init | `FUN_1401bef80` @0x1401bef80 (size 4647) | Loads `settings/terrain.bin`, `terrain/jc3/jc3.world`, physics ports; builds terrain obj + impostors |
| climate config load | `FUN_1401c0480` @0x1401c0480 | Loads `climate/terrainsystem.terrainsystemc`; builds `TerrainPatchSetup` |
| terrain compute pass-graph | `FUN_1401a6d70` @0x1401a6d70 (size 7663) | Adaptive tessellation pipeline (all `Terrain*` passes) |
| terrain virtual-texture cache | `FUN_140f108b0` @0x140f108b0 | Allocates `terrain_indirection_map`/`_color_data`/`_material_data`/`_ssdf_atlas` |
| terrain cache-update submit | `FUN_147962890` @0x147962890 | `"Terrain update cache"` compute submission |
| tree impostor mesh/IB | `FUN_1401a8b60` @0x1401a8b60 | 16384-quad `TreeImpostorIB` |
| tree impostor instance buffers | `FUN_14799cf80` @0x14799cf80 | Per-instance `TreeImpostor` structured buffers |
| tree impostor shader variants | `FUN_1401cf660` @0x1401cf660 | `"TreeImpostor%s%s%sInstanced"` |
| heightfield collision | Havok `hknpHeightFieldShape::updateRegion` (line 5627852) | Streamed terrain physics |

### Biome
| String | FUN_ / DAT_ | Role |
|---|---|---|
| `CBiomeManager` | registered via `thunk_FUN_148f6e780` (FUN_146cd0bea line 3209685) | Biome/climate region manager |
| biome resolve loop | `FUN_1404a57d0` @0x1404a57d0 | Point-in-volume test vs worldsim biome volumes → biome index |
| biome → rich-presence | `FUN_140aa2d20` @0x140aa2d20 | Maps biome index 1–9 → `rp_exploring_biome_*` |
| `CBiomeVolume` | `_DAT_142cb97fc` (FUN_140f27f60 "CBiomeVolume") | Entity component: a biome region volume |
| `CConditional_IsPlayerInsideBiome` | `DAT_142cb5d88` | Behavior predicate |
| `CAiWorldSimSpawnPoint`/`CAiWorldSimModifier` | `DAT_142cb2350`/`DAT_142cb2348` | Worldsim (population — see population doc) |

### Discovery / coverage / map reveal
| String | FUN_ / DAT_ | Role |
|---|---|---|
| `CDiscoveryManager` | reg via thunk (FUN_146cd0bea line 3209680); type-id `DAT_142cba0a0` | Owns discovered-location set |
| discovery state serialize | `FUN_14088e490` @0x14088e490 | Serializes `std::vector<uint>` (obj+0x148..+0x150) = discovered IDs |
| discovery reflection prop | `FUN_14087c480` @0x14087c480 | Registers the `vector<uint>` property on the type |
| % explored / progress UI | `FUN_140e50630` @0x140e50630 | `discovered*100/total`; sets `location_discovered_progress` |
| discovery achievements | `FUN_1408aece0` @0x1408aece0 | `ach_discover_half` (`DAT_142cba078`), `ach_discover_everything` (`DAT_142cba080`) |
| `CDiscoveryLocation` | `DAT_142cb9324` | Entity: a discoverable location |
| `CDiscoveryLocationSetter` | `DAT_142cb9874` | Entity: sets a location discovered |
| `CDiscoveryMapIcon` | `DAT_142cb97f4` | Entity: map icon for a location |
| `CDiscoveryVolume` | `DAT_142cb97ec` | Entity: trigger volume that discovers |
| `CConditional_IsLocationDiscovered` | `DAT_142cb5bfc` | Behavior predicate |
| `CConditional_IsNodeDiscovered` | `DAT_142cb5ca0` | Behavior predicate |
| `CTacticalNodeDiscover` | `DAT_142cba928` | Discovers a tactical node |
| `CCoverageManager` | reg hash `0x41ce7b98` (FUN_146cd0bea line 3209315) | Terrain ground-coverage overlay (render) |
| `CCoverageObject` | `_DAT_142cb0a24` | Coverage entity component |
| coverage RT alloc | `FUN_147b0f830` @0x147b0f830 | Half-res `CoverageResolveTarget`/`CoverageUVMap` |
| coverage resolve/overlay pass | `FUN_147b42b50`, `FUN_147b65cb0` | `CoverageOverlay` / `CoverageResolve` GPU passes |
| `CGameObjectManager` | reg hash `0x41cefc18` | Spawn side that streaming feeds (see population/spawn docs) |
| `CSpawnSystem` | reg hash `0x41d8de38` | Spawn side |
| `CGameWorld` | reg hash `0x41d8e370` | World root |

## How it works (from the decomp)

### 1. Streaming from TAB/ARC (proven)

`CResourceLoaderManager` is a reflected engine subsystem, appended to the boot system table by
`FUN_148f958e0` with type-id hash `0x41cf8e00` alongside `CInputSystem`, `COnlinePlatformSystem`, etc.
Instances come from an object pool (`FUN_140309b80`, 0xae8-byte instances); the trivial typename getter is
`FUN_147e9b280` (`*param_2 = "CResourceLoaderManager"`). (proven)

Archive access is unambiguous and matches `docs/formats/tab_arc_v2.md`:

`FUN_140f90c80` (mounted via `FUN_140f901d0`) builds the paths `"%s/%s%d.tab"` and `"%s/%s%d.arc"`, opens both,
reads a **0x18-byte header**, and checks magic byte-for-byte `local_468=='T' && =='A' && =='B' && =='\0'`.
Version gate: if `local_462 != 1 || local_464 != 2` it falls back to the legacy path (`*(obj+0x1c)=1`, validates
block size `0x800`); otherwise it stamps **`*(obj+0x1c)=2`** — **TAB v2**. It then reads the **12-byte (0xc)
per-entry records** into `obj[4]..obj[5]` (byte-swapping each of the 3 dwords when big-endian) and keeps the
`.arc` handle at `obj[0]`. (proven — FUN_140f90c80 body lines ~1761945+, the `'TAB\0'` check and `(1,2)`
version test read directly)

Entry payloads are read + decompressed by `FUN_140f91f00` (thin wrapper `FUN_14ad432d0`): each entry is either
decoded synchronously through the codec dispatcher or packaged as a 0x58-byte **`decode_fragment`** job for the
task system (interned string cached in `DAT_142cd9040`). The dispatcher `FUN_14ad55350` selects by codec id:
`0`=store/memcpy, `1`=zlib (inits `"1.2.8"`, core `FUN_140f5eac0`), `2`=LZ4-style, `3`=Zstd-style, and
**`4`=`OodleLZ_Decompress(src,srcLen,dst,dstLen,1,0,…,3)`** — the single `OodleLZ_Decompress` import in the
binary (line 4966763). Returns 0 on success / 6 on failure. (proven)

Container/format helpers around this layer: `FUN_14ad36520` sniffs an **`"AAF"`** (Avalanche Archive Format)
container header (version==1), and `FUN_14ad3f430` / `FUN_14725ef12` resolve `%s/%s%u.arc` block-file paths.
`.resourcebundle` catalogs are handled by the large `FUN_147ea7ee0`. A `"CRegionName"` string is interned into
`DAT_142cb9b28` (a region id/name table). (proven — those strings/functions)

**Walled (open):** the *position→which-entry* decision (the cell/tile/streaming-radius policy that decides what
to load/unload around the player) is **not string-tagged**. No `CStreaming`/`CSector`/`CWorldCell`/`LoadCell`
literals survive; that logic lives in the untitled callers of `FUN_140f901d0`/`FUN_140f91f00`. Route to resolve:
the streaming grid parameters are data-side in `settings/terrain.bin` and the `jc3.world` header (see §2), both
reachable via the cracked TAB/ADF tooling.

### 2. Terrain & landscape (proven)

`FUN_1401bef80` is the landscape/terrain-system constructor (only ~4.6 KB of it). It:

- interns `"settings/terrain.bin"` (len 0x14) and `"settings/landscape_effects.bin"` (0x1e) into the object;
- **resolves + reads `"terrain/jc3/jc3.world"`** via a two-call idiom — `thunk_FUN_14ad2fb20(...,"terrain/jc3/jc3.world",&size)` to get the size, then `thunk_FUN_14ad31640(...,"terrain/jc3/jc3.world",0,buf)` to read it into an allocated buffer;
- writes a **world/grid parameter block** (from the `.world` header or defaults): `0x8000` (32768), `0x1000`
  (4096), `9`, `9`, `0xc` (12), `0x200` (512) — plausibly world extent / patch counts / clipmap levels / tile
  resolution (**inferred** as to exact roles; the literals are proven);
- opens Havok collision streams **`terrain_physics_port`** and **`vegetation_physics_port`** (`thunk_FUN_14ad68360`);
- constructs the terrain object at `+0x198` and initializes tree impostors (`FUN_1401a8b60`).

(proven — FUN_1401bef80 lines ~125690–125745)

`FUN_1401c0480` loads the ADF config **`climate/terrainsystem.terrainsystemc`** (resolve-then-read) and creates
the `TerrainPatchSetup` pass object, computing climate-map dimensions. (proven)

**Rendering is a GPU-driven adaptive-tessellation clipmap.** `FUN_1401a6d70` (~7.6 KB) builds the terrain pass
graph, populating labeled compute passes paired with GPU resource slots. The pass names spell out the whole
pipeline: `TerrainClear` → `TerrainPreProcessPatch` → `TerrainProcessPatch` → `TerrainPatchBuildChildMask` →
`TerrainQuadClassification` → `TerrainQuadCalculateOffsets` → `TerrainQuadSumHistogram` →
`TerrainPostQuadClassification` → `TerrainQuadSort` → `TerrainCacheSetup` → `TerrainSubdivideCells` →
`TerrainCellMeshAlloc`/`Sort` → `TerrainDisplaceCells` → `TerrainProjectCells` → `TerrainMorphCells` →
`TerrainTessellateCells` → `TerrainCalculateVertices` → `TerrainBakeDisplacement` → `TerrainBuildTexture`. This
is a **quadtree of patches** that is classified by screen error, subdivided/sorted into cells, then tessellated
and morphed (geomorphing between LODs) on the GPU. Screen passes `TerrainScreenPass{Near,Detail,Far}` and the
`TerrainDeferred*` / `TerrainBasemesh*` / `TerrainShaderForest*` variants are the draw side. (proven — string
inventory + FUN_1401a6d70)

The terrain textures are a **virtual-texture cache**. `FUN_140f108b0` conditionally creates/releases GPU
textures tagged `terrain_material_data` (fmt `0x43`/`0x3d`), `terrain_color_data` (fmt 6), `terrain_ssdf_atlas`
(fmt 1) and `terrain_indirection_map` (fmt `0x1d`), storing handles in a resource struct and freeing old ones —
i.e. an indirection-map + physical-atlas cache. `FUN_147962890` submits the `"Terrain update cache"` compute
(6-entry binding block), and the `TerrainCacheMiss`/`TerrainCacheFlush`/`TerrainCacheUpdate` passes in
`FUN_1401a6d70` service misses. This is how terrain detail is *streamed as texels* rather than as whole meshes.
(proven — FUN_140f108b0, FUN_147962890)

**Vegetation LOD = tree impostors.** `FUN_1401a8b60` builds a static index buffer of `0x4000` (16384) quads
(6 indices each, 0,1,2,0,2,3) named `"TreeImpostorIB"` and a vertex decl from `PTR_s_TreeImpostor_141ccc240`;
`FUN_14799cf80` allocates per-instance `TreeImpostor` structured buffers (0xc-byte stride); `FUN_1401cf660`
selects impostor shader permutations via `"TreeImpostor%s%s%sInstanced"` / `"TreeImpostor%s%s%s%s"`. Distant
trees render as camera-facing impostor billboards instead of meshes. (proven)

**Terrain collision** is a Havok `hknpHeightFieldShape`; `hknpHeightFieldShape::updateRegion` (line 5627852) is
the region-update entry, consistent with the `terrain_physics_port` stream opened at init — the heightfield
collision is patched/streamed region-by-region. (proven — string; the update body is Havok-internal)

### 3. Biomes (proven)

`CBiomeManager` is a registered singleton (via `thunk_FUN_148f6e780`). Biome membership is **volume-based**:
`FUN_1404a57d0` (a per-frame worldsim update) lazily loads four named biome volumes —
`worldsim_biome_desert`, `_grassland`, `_alpine`, `_rainforest` — into struct slots `param_1+0x910/+0x928/
+0x940/+0x958` (via `FUN_14046c250`), then tests the player position against each with a point-in-volume probe
`thunk_FUN_148169950`, producing a biome index/bitmask (`0x101`,`0x102`,`0x104`,`0x108`,`0x110`, …). (proven)

`FUN_140aa2d20` maps the resolved biome to a **rich-presence** string, which fixes the game's biome enumeration:
`1=alpine`, `2=grassland`, `3=rainforest`, `4=desert`, `5=ocean`, `6=frontline`, `7=dlc1_island`,
`8=dlc2_island`, `9=dlc2_infestation` → `rp_exploring_biome_*` / `rp_exploring_dlc*`. The behavior predicate
`CConditional_IsPlayerInsideBiome` (`DAT_142cb5d88`) exposes the same test to the gameplay VM
(see `behavior_system.md`). (proven)

### 4. Map discovery & % explored (proven)

`CDiscoveryManager` holds the discovered set as a **`std::vector<unsigned int>`** at object offset `+0x148..
+0x150`. `FUN_14088e490` serializes it to the save stream: it writes the type-id tag (XOR of the
`CDiscoveryManager` / `vector<uint>` / `u64` / `u32` type-id hashes), then the element count
`(0x150-0x148)>>2`, then each `uint` ID. `FUN_14087c480` is the reflection registrar that declares that
`vector<uint>` property on the type. So **discovery persistence = a flat list of discovered location IDs**.
(proven — FUN_14088e490 body)

Discovery is authored with entity components: `CDiscoveryLocation` (the point of interest),
`CDiscoveryVolume` (a trigger that marks it discovered), `CDiscoveryLocationSetter`, `CDiscoveryMapIcon`, and
`CLocationResolverDiscoveryLocation`; behavior predicates `CConditional_IsLocationDiscovered`,
`CConditional_IsNodeDiscovered`, `CConditional_IsHoveringOverDiscoveryLocation`,
`CConditional_IsPlayerInsideNodeDiscoveryVolume`, plus `CTacticalNodeDiscover`. (proven — the
`FUN_140f27f60(...)` type registrations at the cited DAT_ globals)

**% explored** is computed in `FUN_140e50630`: with `discovered = *(rec+0x70)` and `total = *(rec+0x74)`, it
forms `progress = total<1 ? 100 : discovered*100/total`, formats it into the localized string
`"location_discovered_progress"`, and — when `discovered >= total` — plays `"sfx_gui_location_complete_enter"`.
The 50% / 100% milestones drive achievements `ach_discover_half` and `ach_discover_everything`
(`FUN_1408aece0`, `DAT_142cba078`/`DAT_142cba080`). (proven)

### 5. Coverage overlay (proven — but not fog-of-war)

`CCoverageManager` (`0x41ce7b98`) and `CCoverageObject` are a **terrain ground-coverage rendering** system, not
map fog. `FUN_147b0f830` allocates **half-resolution** (`w>>1`,`h>>1`) render targets `CoverageResolveTarget`
and `CoverageUVMap`; `FUN_147b42b50`/`FUN_147b65cb0` run the `CoverageOverlay` and `CoverageResolve`
(`CoverageResolve::LocalResources`) GPU passes. The assets are
`models/environments/shared/coverage/textures/coverage_{alpha_dif,nrm,mpm}.ddsc` — blended ground detail /
decal coverage projected onto terrain. (proven — FUN_147b0f830 + string inventory). *Map reveal / fog-of-war is
the Discovery system in §4, a separate mechanism despite the similar "coverage" wording.*

## Data & config integration

- **World payload:** everything above is packed in **TAB/ARC v2** (`docs/formats/tab_arc_v2.md`,
  `[[tab-v2-cracked]]`); read via `FUN_140f90c80` + Oodle in `FUN_14ad55350`. Route new decodes through
  `tools/jc4_arc`.
- **Terrain config:** `settings/terrain.bin`, `settings/landscape_effects.bin`, and the height/patch world file
  `terrain/jc3/jc3.world` (loaded by `FUN_1401bef80`); climate/patch config
  `climate/terrainsystem.terrainsystemc` is **ADF** (`FUN_1401c0480`) — decode with `tools/jc4_adf`
  (`docs/formats/adf.md`). The streaming grid tunables (cell size, load radius, LOD ring distances) live in
  these files, not in code (walled).
- **Entities:** biome volumes (`CBiomeVolume`), discovery locations/volumes/icons (`CDiscovery*`) and coverage
  objects (`CCoverageObject`) are RTPC entity components — map their class names to component hashes via
  `[[rtpc-entity-assembly]]` / `[[composite-assets]]`.
- **Terrain collision:** Havok `hknpHeightFieldShape` streamed through `terrain_physics_port`; the tagfile
  lineage is Havok 2016.x (`[[havok-hct-2018]]`).
- **Type-id ↔ asset hash identity:** all the `0x41cd….`/`0x41cf….` manager hashes and the class-name strings
  hash under the same lookup3 `hashlittle` as asset paths (`docs/formats/name_hash.md`).

## Notable constants / tunables

| Value | Where (FUN_) | Meaning |
|---|---|---|
| `'T','A','B','\0'` | FUN_140f90c80 | TAB archive magic |
| version `(local_462==1, local_464==2)` → `*(obj+0x1c)=2` | FUN_140f90c80 | TAB **v2** selector (else legacy `=1`) |
| block size `0x800` (2048) | FUN_140f90c80 | Legacy-TAB block size check |
| entry record = `0xc` (12) bytes, 3 dwords | FUN_140f90c80 | `.tab` entry stride |
| codec ids `0/1/2/3/4` | FUN_14ad55350 | store / zlib / lz4 / zstd / **Oodle** |
| `OodleLZ_Decompress(...,1,0,…,3)` | line 4966763 | Retail block decompressor call |
| zlib `"1.2.8"` | FUN_14ad55350 | Bundled zlib version |
| `0x8000, 0x1000, 9, 9, 0xc, 0x200` | FUN_1401bef80 | World/terrain grid params (roles inferred) |
| `0x4000` (16384) quads | FUN_1401a8b60 | `TreeImpostorIB` capacity |
| tex fmts `0x43/0x3d, 6, 1, 0x1d` | FUN_140f108b0 | material / color / ssdf / indirection cache formats |
| `discovered*100/total`, clamp 100 if total<1 | FUN_140e50630 | % explored |
| coverage RT at `w>>1, h>>1` | FUN_147b0f830 | Half-res coverage targets |
| manager type-id hashes | FUN_146cd0bea / FUN_148f960c0 / FUN_148f958e0 | `CLandscapeManager 0x41cd4e80`, `CCoverageManager 0x41ce7b98`, `CGameObjectManager 0x41cefc18`, `CModelInstanceManager 0x41cf8298`, `CProjectEffectToTerrain 0x41cf8da0`, `CSpawnSystem 0x41d8de38`, `CGameWorld 0x41d8e370`, `CResourceLoaderManager 0x41cf8e00` |

## Call-graph highlights

```
boot: FUN_148f958e0  → registers CResourceLoaderManager (0x41cf8e00)
      FUN_146cd0bea / FUN_148f960c0 → register CLandscapeManager, CBiomeManager,
                                       CGameObjectManager, CCoverageManager, CDiscoveryManager, …

streaming:
  FUN_140f901d0 (mount) ─ FUN_140f90c80  open .tab/.arc, 'TAB\0', v2, 12B entry table
  FUN_14ad432d0 ─ FUN_140f91f00  (read+decompress)
        ├ sync  → FUN_14ad55350 codec: 0 store · 1 zlib(FUN_140f5eac0) · 2 lz4 · 3 zstd · 4 OodleLZ_Decompress
        └ async → task "decode_fragment" → same FUN_14ad55350

terrain:
  FUN_1401bef80 (landscape init)
     ├ load settings/terrain.bin, terrain/jc3/jc3.world, physics ports
     ├ FUN_1401c0480  climate/terrainsystem.terrainsystemc + TerrainPatchSetup
     └ FUN_1401a8b60  tree impostors (TreeImpostorIB)
  FUN_1401a6d70 (pass graph)  Terrain{PreProcess,Process}Patch → Quad{Classify,Sort,Histogram}
     → SubdivideCells → {Displace,Project,Morph,Tessellate}Cells → CalculateVertices → BuildTexture
  FUN_140f108b0 / FUN_147962890  terrain virtual-texture cache (indirection_map + atlases)

biome:      FUN_1404a57d0 (resolve by volume) → FUN_140aa2d20 (biome index → rp_exploring_biome_*)
discovery:  FUN_14088e490 (serialize vector<uint>) · FUN_140e50630 (% explored) · FUN_1408aece0 (achievements)
coverage:   FUN_147b0f830 (RT alloc) → FUN_147b42b50 / FUN_147b65cb0 (CoverageResolve/Overlay)
```

## Open questions / lower-confidence

- **Streaming grid policy (walled).** The concrete cell/tile geometry and the position→load-radius policy are
  not string-tagged and not recoverable from code alone; they sit in `settings/terrain.bin` / `jc3.world`.
  Next: decode those with `jc4_adf`/`jc4_arc` and correlate the `0x8000/0x1000/9/9/0xc/0x200` block against the
  header layout. (inferred)
- **World grid param roles.** The six literals in `FUN_1401bef80` are proven values but their exact semantics
  (extent vs. patch count vs. clipmap depth vs. tile res) are inferred; confirm against the `.world` header.
- **`MatchStreamingFacesTask`** is confirmed as a profiled task but its body (`FUN_141543ef0`) — whether it
  streams AMF mesh LODs by face budget — needs a read pass. (inferred)
- **CBiomeManager runtime store.** The biome-volume list is read from `DAT_142cb1d48`/`DAT_142c84b90`; the
  manager ctor and how volumes are ingested from entities is not yet traced. (open)
- **`CProjectEffectToTerrain`** (`0x41cf8da0`) and the `Vegetation_Terrain`/`WeatherTerrainForce` hooks are
  registered but unexplored here — they bridge weather/effects onto terrain (see `wind_and_weather.md`). (open)
- **Async `decode_fragment` task graph** — the consumer side (who drains completed fragments back into resident
  resources) is untitled; trace callers of `DAT_142cd9040`. (open)

## Appendix — decomp anchors

Strings (all quoted literals present in the dump):
`"CResourceLoaderManager"`, `"CLandscapeManager"`, `"CBiomeManager"`, `"CDiscoveryManager"`,
`"CCoverageManager"`, `"CGameObjectManager"`, `"CGameWorld"`, `"CSpawnSystem"`, `"CProjectEffectToTerrain"`,
`"CModelInstanceManager"`, `"CBiomeVolume"`, `"CCoverageObject"`, `"CDiscoveryLocation"`,
`"CDiscoveryLocationSetter"`, `"CDiscoveryMapIcon"`, `"CDiscoveryVolume"`, `"CLocationResolverDiscoveryLocation"`,
`"CTacticalNodeDiscover"`, `"CRegionName"`, `"CConditional_IsPlayerInsideBiome"`,
`"CConditional_IsLocationDiscovered"`, `"CConditional_IsNodeDiscovered"`,
`"CConditional_IsHoveringOverDiscoveryLocation"`, `"CConditional_IsPlayerInsideNodeDiscoveryVolume"`,
`"location_discovered_progress"`, `"ach_discover_half"`, `"ach_discover_everything"`,
`"sfx_gui_location_complete_enter"`, `"worldsim_biome_{desert,grassland,alpine,rainforest}"`,
`"rp_exploring_biome_{alpine,grassland,rainforest,desert,ocean,frontline}"`,
`"rp_exploring_dlc{1_island,2_island,2_infestation}"`, `"terrain/jc3/jc3.world"`, `"settings/terrain.bin"`,
`"settings/landscape_effects.bin"`, `"climate/terrainsystem.terrainsystemc"`, `"terrain_physics_port"`,
`"vegetation_physics_port"`, `"terrain_{indirection_map,color_data,material_data,normal,displacement,ssdf_atlas}"`,
the `"Terrain*"` compute-pass family, `"TreeImpostor"`/`"TreeImpostorIB"`,
`"Coverage{ResolveTarget,UVMap,Overlay,Resolve}"`, `"CoverageResolve::LocalResources"`,
`"MatchStreamingFacesTask"`, `"showStreamingCollection"`, `"decode_fragment"`, `"AAF"`, `".resourcebundle"`,
`OodleLZ_Decompress`, `hknpHeightFieldShape::updateRegion`, and the `'TAB\0'` byte check.

Functions:
FUN_148f958e0, FUN_148f960c0, FUN_146cd0bea, FUN_147e9b280, FUN_140309b80, FUN_140f901d0, FUN_140f90c80,
FUN_140f93aa0, FUN_140f91f00, FUN_14ad432d0, FUN_14ad55350, FUN_140f5eac0, FUN_14ad36520, FUN_14ad3f430,
FUN_14725ef12, FUN_147ea7ee0, FUN_1415730a0, FUN_1401bef80, FUN_1401c0480, FUN_1401a6d70, FUN_140f108b0,
FUN_147962890, FUN_1401a8b60, FUN_14799cf80, FUN_1401cf660, FUN_1404a57d0, FUN_140aa2d20, FUN_14088e490,
FUN_14087c480, FUN_140e50630, FUN_1408aece0, FUN_147b0f830, FUN_147b42b50, FUN_147b65cb0.
OodleLZ_Decompress @ decomp line 4966763; `hknpHeightFieldShape::updateRegion` @ line 5627852.

> Methodology caveat (project mandate): this is a **functions-only** export. Recovered here: the registration
> skeletons, the TAB/ARC+Oodle reader call graph, the GPU terrain pass pipeline, biome/discovery mechanics and
> present constants. **Not** recovered from code: the position-driven streaming policy and the numeric
> streaming/LOD tunables — those are data-side (`settings/terrain.bin`, `jc3.world`, ADF climate config) and are
> tagged **walled/open** above, routed through the already-cracked `jc4_arc` / `jc4_adf` tooling.

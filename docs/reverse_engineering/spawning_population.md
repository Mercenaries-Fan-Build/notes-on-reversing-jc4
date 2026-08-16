# Spawning & Population — how the world gets populated with NPCs, traffic & dynamic objects

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

> Methodology wall (see repo `docs/binary_recon.md` + this dir's README): the decomp export is
> **functions-only**. Recoverable here = the **registration skeletons** (`FUN_140f27f60` /
> `FUN_14085fd00` component tables), the **CSpawnSystem load path**, the **config-blob asset names +
> ADF field hashes**, the **debug-command surface**, string constants, and the reachable CPU logic
> (spawn-rule enumeration, player-spawn-point trigger). **NOT recoverable** = the per-frame spawn-director
> *tick body* (where/when/how-many is decided each frame) — it lives behind data-section vtables
> (`PTR_LAB_141d578f0`, the manager vtables) — and the **magnitudes** (budget counts, density caps,
> distances), which live in the two ADF config blobs (`settings/spawn_budget_pools.bin`,
> `packages/main/spawn_expentity_defs.bin`) and the `ai/*.aisystunec` tuning blocks, i.e. the *data* side.
> Present constants below are the few baked into the code; the tuning **magnitudes are named but walled**.

## Overview

Just Cause 4 populates Solís through **`CSpawnSystem`** — a global manager (registered by name, hash
`0x41d8de38`, vtable `PTR_LAB_141d90548`, in the master manager-registry `FUN_146cd0bea`) that owns a pool
of **spawn-rule** objects and a table of **exported entity definitions**. At load it reads two data blobs —
`settings/spawn_budget_pools.bin` (the density/budget pools) and `packages/main/spawn_expentity_defs.bin`
(the per-definition spawn table) — and builds a fixed-size occupancy pool sized to the definition count
(`FUN_140b8f4e0`). Placement/gating is expressed as world-placed **component entities**: spawn points
(`CAiWorldSimSpawnPoint`, `CPlayerSpawnPoint`, `CFastTravelSpawnPoint`), spawn *rules* + *sources*
(`CSpawnRule`, `CObjectSourceSpawners`, `CSpawnTagConditional`), **density volumes** (`CLocalDensityVolumeObject`,
`CAiHeatDensityVolume`), **traffic** authoring (`CAITrafficSpline`, `CAiTrafficObstacle`, `CTrafficLight`), and
**despawn** controls (`CDespawnVolume`, `CEncounterDespawnSettings`) — all registered as reflected components
through `FUN_14085fd00`/`FUN_140f27f60`. Ambient AI numbers (traffic/worldsim/pedestrian densities) come from
the AI tuning blocks (`ai/traffic.aisystunec`, `ai/worldsim.aisystunec`), loaded by `CAiSystem` (`FUN_1403c89a0`).
**Reinforcement waves** are a *separate* spawner driven by heat — the 3-pool `AiHeatManager` — documented in
`faction_frontline_chaos.md`; this doc covers the ambient/director side and cross-references it.

## Key classes & functions

| String / class | FUN_ (registration / implementation) | Role |
|---|---|---|
| `CSpawnSystem` | registered in `FUN_146cd0bea` (hash `0x41d8de38`, vtable `PTR_LAB_141d90548`) | The spawn director manager singleton |
| CSpawnSystem load/init | `FUN_140b8f4e0` (size 6117; callers `FUN_14086d1e0`) | Loads budget pools + entity-defs, builds occupancy pool, registers `spawn.*` cmds |
| spawn-rule-system ctor | `FUN_140697bc0` (vtable `PTR_LAB_141d578f0`; caller `FUN_14050f3a0`) | Constructs a spawn-rule owner; registers `spawn.reset_all_spawn_rules` / `…post_reset…` |
| spawn cmd binder | `FUN_140b91210` | Binds `spawn.reset`, `spawn.reset_all_spawn_rules`, `spawn.post_reset_all_spawn_rules` |
| spawn-rule enumeration | `FUN_140b9d840` (size 302) | Walks the spawn-rule pointer list, virtual type-check, collects matches; then jumps to reset vtable slot `+0x108` |
| `CSpawnRule` | reflected component (`FUN_140f27f60`) | A rule governing what/where to spawn |
| `CObjectSourceSpawners` | reflected component | Source set a rule draws instances from |
| `CSpawnTagConditional` / `CConditional_SpawnTags` | reflected component / condition | Tag-gated spawn predicate |
| `CObjectTrackerFilterSpawnSources` / `…SpawnTags` | reflected component | Object-tracker filters keyed on spawn source/tag |
| `CSpawnParam_ObjectCritical` / `CSpawnParam_ObjectiveCritical` | reflected component | Per-spawn params marking an instance non-cullable |
| `CAiWorldSimSpawnPoint` | reflected component | Ambient (world-sim) spawn point — the ambient population anchor |
| `CLocalDensityVolumeObject` / `CAiHeatDensityVolume` | reflected component | Volumes that scale local spawn density |
| `CAITrafficSpline` / `CAiTrafficObstacle` / `CTrafficLight` | reflected component | Traffic path graph, obstacle blocking, signal control |
| `CBirdSpawner` / `CBulletSpawner` / `CDemonWaveSpawner` / `CTornadoSpawnPoint` | reflected component | Specialised spawners (ambient birds, projectile, DLC demon waves, tornado) |
| `CDespawnVolume` / `CEncounterDespawnSettings` | reflected component | Despawn/recycle regions + per-encounter despawn policy |
| `CFrontlineSpawnBlocker` | reflected component | Suppresses spawning inside frontline zones (ties to `CFrontlineManager`) |
| `CConveyorBeltSpawnRule` | reflected component | Streaming conveyor spawn rule |
| `CReparentOnSpawn` / `CRespawnAtPoint` | reflected component | Attach-on-spawn / respawn-at-a-point behaviours |
| `CPlayerSpawnPointManager` | registered in `FUN_146cd0bea` (vtable `PTR_LAB_141d90908`) | Owns the **player** respawn/checkpoint set |
| `CPlayerSpawnPoint` / `CPlayerSpawnPointTrigger` | reflected components | Player respawn anchors + their triggers |
| `CFastTravelSpawnPoint` | reflected component | Fast-travel arrival point |
| player-spawn-point trigger | `FUN_1498a9520` (fires `playerspawnpoint.triggered`) | On trigger, snapshots the 12-float transform → `FUN_140b23690` (register current respawn) |
| player-spawn entity build | `FUN_14916d950` (size 2911) | Builds a player-spawn entity; binds `playerspawnpoint_loading_done` |
| AI tuning loader | `FUN_1403c89a0` | Loads `ai/traffic.aisystunec`, `ai/worldsim.aisystunec`, `ai/heat.aisystunec` (+7 more) |

## How it works (from the decomp)

### 1. Load path — `CSpawnSystem` init (`FUN_140b8f4e0`, proven)

On init the spawn system:

1. Pulls the global exported-entity list:
   `FUN_140b52890(…, "packages/main/generated_global_resources.exported_entity_files")` (line 1374063).
2. Loads **`settings/spawn_budget_pools.bin`** (line 1374072) and reads it as ADF — field hashes
   **`0x83647b76`** then nested **`0x8eb4f892`** (`thunk_FUN_147897970`, lines 1374077–1374079). This is the
   **density / budget pool** table. (proven the blob is loaded and ADF-walked; the pool *values* are ADF
   data — walled.)
3. Loads **`packages/main/spawn_expentity_defs.bin`** (line 1374597), reads ADF field **`0x4fdd5ddf`**
   (line 1374598), takes the **entry count** as a `ushort` at `+0x0a` of the ADF array header
   (line 1374604), and **allocates `count × 0x188` bytes**, constructing each **0x188-byte (392-byte)
   definition record** with element ctor `LAB_140b6d420` (`_eh_vector_constructor_iterator_`, lines
   1374619–1374625). (proven — the per-definition record is 0x188 bytes.)
4. Builds an **occupancy bitmask pool** sized to the definition count: it stores `count` at `+0xb4`, `0`
   at `+0xb8`, and **`(count + 0x3f) >> 6`** (number of 64-bit words to hold `count` bits) at `+0xb0`
   (lines 1374628–1374630). (proven — this is the classic fixed-slot pool: one bit per definition slot,
   rounded up to 64-bit words → the **pooling/recycling substrate**.)
5. Iterates the definition/rule set; for matching rules it fires the game event
   **`requested_supply_spawned`** (`thunk_FUN_147625370`, line 1382170) — i.e. spawn requests are raised as
   named events, not direct instantiations (decoupled request → fulfilment). The related fire-and-forget
   event **`supply_drop_crate_spawned`** (`thunk_FUN_148fdf5b0`, line 1426474) is raised elsewhere on the
   same event bus. (proven strings; the request→spawn wiring beyond the event is walled.)

### 2. Spawn rules & the reset surface (proven)

The spawn system exposes three console/debug commands, bound in `FUN_140b91210` and again in the spawn-rule
owner ctor `FUN_140697bc0`:

- **`spawn.reset`** — bound at owner `+0x188`.
- **`spawn.reset_all_spawn_rules`** — bound at `+0x190`/`+0x100`.
- **`spawn.post_reset_all_spawn_rules`** — bound at `+0x198`/`+0x101`.

`FUN_140b9d840` is the reset worker: it walks the spawn-rule **pointer array** (`param_1[0x31] …
param_1[0x32]`, 8-byte stride — a `std::vector` of rule pointers), calls a **virtual predicate at vtable
slot `+0x10`** on each rule against type token `&DAT_141d919f0`, collects the passing rules' handles into an
output vector, re-registers `spawn.reset_all_spawn_rules`, then tail-jumps through **vtable slot `+0x108`**
(the actual reset). (proven the enumeration exists and is virtual-dispatched; the reset body is behind the
vtable → walled.)

The spawn-rule owner ctor `FUN_140697bc0` also constructs a **3 × 0x80-byte inline sub-array**
(`_eh_vector_constructor_iterator_(param_1 + 0x103, 0x80, 3, …)`, line 655555) and a **900-byte zeroed
region** at `+0x184` (`FUN_141afbf5e(param_1 + 0x184, 0, 900)`, line 655558) — fixed scratch/pool storage.
(proven.)

### 3. Player spawn / respawn / checkpoints (proven)

`CPlayerSpawnPointManager` (vtable `PTR_LAB_141d90908`) owns the player respawn set; the world-placed anchors
are `CPlayerSpawnPoint` + `CPlayerSpawnPointTrigger`, plus `CFastTravelSpawnPoint` for fast-travel arrivals
and `CRespawnAtPoint` as a respawn behaviour.

`FUN_1498a9520` is the **trigger handler**: it fires the event `playerspawnpoint.triggered`, and if flag
`(*(byte*)(param_1+0x144) & 1)` is set it calls `FUN_14024deb0`, then snapshots a **12-float transform**
(a 4×3 matrix: `+0x104 … +0x140`, lines 4396484–4396495) and hands it to **`FUN_140b23690`** — i.e. touching
the trigger records that point as the **current player respawn location** (the checkpoint mechanism).
(proven.) The player-spawn entity itself is assembled in `FUN_14916d950`, which binds the
`playerspawnpoint_loading_done` completion event (line 4176918). (proven.)

### 4. Ambient population, traffic & density (inferred/walled)

Traffic and world-sim (ambient pedestrian/vehicle) population are authored as component entities —
`CAITrafficSpline` (path graph), `CAiTrafficObstacle` (dynamic blockers), `CTrafficLight` (signal state),
`CAiWorldSimSpawnPoint` (ambient spawn anchor) — and their **densities/rates are AI tuning data**, loaded by
`CAiSystem`'s tuning loader `FUN_1403c89a0` from `ai/traffic.aisystunec` (→ `+0x60`), `ai/worldsim.aisystunec`
(→ `+0x68`), `ai/heat.aisystunec` (→ `+200`) (lines 332017–332021). The `.aisystunec` blobs are ADF; the
numbers are on the data side → **walled**. (proven the tuning blobs are loaded at those offsets; magnitudes
walled.) **Local density scaling** is done with `CLocalDensityVolumeObject` / `CAiHeatDensityVolume` volumes;
the many `LocalDensityVolume*` / `DENSITY_VOLUME` / `LOCAL_DENSITY_VOLUME_TILE_BINNING` strings are the GPU
voxelization path for VFX fog density (`VolumetricFogDensityInject`, etc.), *distinct* from the AI spawn
density volumes — do not conflate. (inferred from string families.)

### 5. Despawn / recycling (proven strings, walled bodies)

`CDespawnVolume` (region-based despawn), `CEncounterDespawnSettings` (per-encounter despawn policy), and
`CFrontlineSpawnBlocker` (spawn suppression inside frontline zones) are the recycling/gating components.
Combined with the per-definition occupancy bitmask (§1.4), this is a **fixed-pool spawn-and-recycle** design:
a bounded set of definition slots, bits toggled as instances are spawned/despawned. (inferred — pool sizing
is proven; the toggle logic is in the walled tick.)

### 6. Reinforcement waves (cross-reference)

Heat-driven **reinforcement waves** are handled by a *separate* spawner from ambient population: the
**`AiHeatManager`** 3-pool reinforcement spawner (`AiHeatManager/"Vehicles"`, `/"Character"`, `/"Props"`,
singleton `DAT_142cb1d20`, alloc `FUN_140b80530`, pool logic near `FUN_140437…` lines 381120–381270). The
world-placed anchors are `CAiHeatSpawner`, `CAiHeatAOOSpawnPoint`, `CAiHeatDensityVolume`. Full treatment is
in **`faction_frontline_chaos.md`** (§ reinforcement). Character/creature *pools* (the 64-slot creature pool)
are in **`characters_creatures.md`** — this doc is the spawn-**director** side (where/when/how-many), those
are the instance pools it fills. (proven cross-refs.)

## Data & config integration

- **`settings/spawn_budget_pools.bin`** — ADF; the density/budget pool definitions. Root field hash
  `0x83647b76`, nested `0x8eb4f892` (`FUN_140b8f4e0`). Decode with `jc4_adf` to recover the actual pool
  counts/budgets. (proven the blob + hashes; values walled until decoded.)
- **`packages/main/spawn_expentity_defs.bin`** — ADF; the exported spawn-entity definition table. Field hash
  `0x4fdd5ddf`; array of **0x188-byte** records; count is a `ushort` at ADF-array `+0x0a` (`FUN_140b8f4e0`).
- **`packages/main/generated_global_resources.exported_entity_files`** — the master exported-entity manifest
  the spawn defs index into (`FUN_140b52890`).
- **`ai/traffic.aisystunec`, `ai/worldsim.aisystunec`, `ai/heat.aisystunec`** — AI tuning ADF blocks holding
  ambient traffic/pedestrian/reinforcement densities (`FUN_1403c89a0`).
- All spawn components are RTPC entity components instantiated by **name→hash** through `FUN_140f27f60`
  (lookup3 `hashlittle`, the project's cracked name hash — `docs/formats/name_hash.md`,
  `[[rtpc-entity-assembly]]`). To resolve a component's *fields*, tie its class name here to its entity
  component hash in the RTPC crack. (proven pipeline; per-component hashes not yet mapped here.)

## Notable constants / tunables (baked in code — the rest are walled in ADF)

| Value | Where | Meaning (grade) |
|---|---|---|
| record stride **`0x188` (392 B)** | `FUN_140b8f4e0` @1374610 | Size of one spawn-entity-definition record (proven) |
| **`(count + 0x3f) >> 6`** at `+0xb0` | `FUN_140b8f4e0` @1374630 | Occupancy-bitmask word count → per-slot spawn pool (proven) |
| **`100`** (int) at owner `+0x7dc` | `FUN_140697bc0` @655547 | A spawn-owner count/budget field (proven value; semantics inferred) |
| **`0x42c80000` = 100.0f** at owner `+0x17c` | `FUN_140697bc0` @655545 | A spawn-owner distance/radius field (proven value; semantics inferred) |
| **`5`** at owner `+0x174` / `+0x2e` | `FUN_140697bc0` @655542-655543 | Small count/limit (proven value; semantics inferred) |
| **`0x1010101` / `0x101`** init bytes | `FUN_140697bc0` @655534-655537 | Four/two bool flags initialised to 1 (proven) |
| **`3 × 0x80`-byte** inline array | `FUN_140697bc0` @655555 | Fixed sub-pool in the spawn-rule owner (proven) |
| **`900`-byte** zeroed region `+0x184` | `FUN_140697bc0` @655558 | Fixed scratch/table in the spawn-rule owner (proven) |

> The gameplay-meaningful magnitudes — per-pool budgets, spawn/despawn distances-from-player, traffic/pedestrian
> densities, wave sizes — are **named but data-side** (the two `.bin` blobs + `.aisystunec`), not in the code.

## Call-graph highlights

- Manager install: `FUN_146cd0bea` (master registry) → registers `CSpawnSystem` (`0x41d8de38`) and
  `CPlayerSpawnPointManager`.
- Component install: `FUN_14085fd00` → `FUN_140f27f60("C…Spawn…"/"C…Traffic…"/"C…Density…", …)` — every
  spawn/traffic/density component is a row here.
- Spawn-system boot: `FUN_14086d1e0` → **`FUN_140b8f4e0`** (load blobs, build pool, bind `spawn.*`).
- Spawn-rule owner: `FUN_14050f3a0` → **`FUN_140697bc0`** (ctor). Reset path: command → **`FUN_140b9d840`**
  (enumerate rules) → vtable `+0x108`.
- Player respawn: trigger → **`FUN_1498a9520`** (`playerspawnpoint.triggered`) → **`FUN_140b23690`**
  (register transform). Entity build: **`FUN_14916d950`**.
- AI tuning: `FUN_1403c89a0` → `FUN_1403c9480(… "ai/traffic.aisystunec" …)` ×10 tuning blocks.
- Reinforcements (separate): `AiHeatManager` `DAT_142cb1d20`, pools via `FUN_140b80530` — see
  `faction_frontline_chaos.md`.

## Open questions / lower-confidence

- **The per-frame spawn-director tick is walled** (data-section vtable `PTR_LAB_141d578f0` and the manager
  vtables). Where/when/how-many-per-tick, distance-from-player gating math, and the density-volume → spawn-rate
  mapping are not in the functions export. Resolving needs either a vtable/data dump or live `x64dbg` on the
  spawn manager update.
- **Budget/density magnitudes** require decoding `settings/spawn_budget_pools.bin` (`jc4_adf`, field
  `0x83647b76`/`0x8eb4f892`) and the `.aisystunec` blocks. Named, not yet valued.
- **Per-component field layouts** (e.g. `CAiWorldSimSpawnPoint`, `CSpawnRule`, `CDespawnVolume`) — map each
  class name to its RTPC entity-component hash to recover fields (`[[rtpc-entity-assembly]]`).
- **`requested_supply_spawned` → instance** wiring: the request event is proven; the fulfilment consumer
  (who listens and instantiates) is not yet traced.
- Whether `CLocalDensityVolumeObject` (AI) and the `LocalDensityVolume*` GPU voxel path share any data — the
  string families overlap but the code paths look distinct (VFX fog vs AI density). (speculative.)

## Appendix — decomp anchors

Strings/addresses used above, for re-verification against
`output/_ghidra_jc4/jc4_all_functions_decomp.txt`:

- Manager registry: `FUN_146cd0bea` — `"CSpawnSystem"` (hash `0x41d8de38`, `PTR_LAB_141d90548`) @~3209xxx;
  `"CPlayerSpawnPointManager"` (`PTR_LAB_141d90908`) @~3209500.
- Spawn-system load: `FUN_140b8f4e0` @0x140b8f4e0 — `"settings/spawn_budget_pools.bin"` @1374072 (ADF
  `0x83647b76`/`0x8eb4f892`); `"packages/main/spawn_expentity_defs.bin"` @1374597 (ADF `0x4fdd5ddf`, `0x188`
  records, count@`+0x0a`, pool `(count+0x3f)>>6`@1374630); `"requested_supply_spawned"` @1382170.
- Spawn commands: `FUN_140b91210` — `"spawn.reset"` @1375249, `"spawn.reset_all_spawn_rules"` @1375251,
  `"spawn.post_reset_all_spawn_rules"` @1375252. Owner ctor `FUN_140697bc0` @0x140697bc0 (same commands
  @655563-655566; consts @655542-655558; vtable `PTR_LAB_141d578f0`).
- Spawn-rule enumeration/reset: `FUN_140b9d840` @0x140b9d840 (vtable predicate `+0x10` vs `DAT_141d919f0`,
  reset `+0x108`).
- Player spawn: `FUN_1498a9520` @0x1498a9520 — `"playerspawnpoint.triggered"` @4396480, transform
  @4396484-4396495 → `FUN_140b23690`. `FUN_14916d950` @0x14916d950 — `"playerspawnpoint_loading_done"`
  @4176918.
- AI tuning: `FUN_1403c89a0` — `"ai/traffic.aisystunec"` @332017, `"ai/worldsim.aisystunec"`,
  `"ai/heat.aisystunec"` @332018-332021.
- Component registrations (via `FUN_14085fd00`→`FUN_140f27f60`; second arg = **name strlen**, not struct
  size): `CSpawnRule`, `CSpawnTagConditional`, `CObjectSourceSpawners`, `CObjectTrackerFilterSpawnSources`,
  `CObjectTrackerFilterSpawnTags`, `CSpawnParam_ObjectCritical`, `CSpawnParam_ObjectiveCritical`,
  `CAiWorldSimSpawnPoint`, `CPlayerSpawnPoint`, `CPlayerSpawnPointTrigger`, `CFastTravelSpawnPoint`,
  `CRespawnAtPoint`, `CReparentOnSpawn`, `CDespawnVolume`, `CEncounterDespawnSettings`,
  `CFrontlineSpawnBlocker`, `CConveyorBeltSpawnRule`, `CLocalDensityVolumeObject`, `CAiHeatDensityVolume`,
  `CAITrafficSpline`, `CAiTrafficObstacle`, `CTrafficLight`, `CBirdSpawner`, `CBulletSpawner`,
  `CDemonWaveSpawner`, `CTornadoSpawnPoint`, `CConditional_SpawnTags` (all @~860200-869800 cluster).
- Reinforcement cross-ref: `AiHeatManager` `DAT_142cb1d20`, `FUN_140b80530`, `FUN_140437…` (see
  `faction_frontline_chaos.md`).

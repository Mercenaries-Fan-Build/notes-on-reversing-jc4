# Wind & extreme weather — tornadoes, storms, wind volumes, force pulses, lightning

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

> "proven" = read directly from the decomp (`output/_ghidra_jc4/jc4_all_functions_decomp.txt`).
> All `FUN_`/`DAT_`/`PTR_LAB_` addresses below are from that dump. Names in `"quotes"` are the
> engine's own C++ class/action strings that survive as string constants.

## Overview

Just Cause 4's signature weather is built from three cooperating layers, all visible in the decomp:

1. **World subsystems (managers).** The game world registers a fixed set of singleton managers, among
   them `CWindTunnelManager`, `CTopographicalWind`, `CPfxWindPhysicsSystem` and `CLightningManager`
   (proven — registration table `FUN_148f960c0`, a parallel table near line 3209275, and the
   `thunk_FUN_148f6e780` manager installs). These own the ambient/topographical wind field, the particle-effect
   wind physics, and lightning scheduling.
2. **RTPC entity components (placed objects).** Concrete weather is authored as entities carrying
   components: `CTornadoObject`, `CTornadoController`, `CTornadoFan`, `CTornadoReachPoint`,
   `CTornadoSpawnPoint`, `CTornadoTrigger`, `CStormObject`, `CStormRadiusControl`, `CLocalWindObject`,
   `CWindTunnelObject`, `CForcePulse`/`CForceField`/`CForcePoint`, `CLightningObject`,
   `CLightningStrikeController`, and the `CWeatherPreset` / `CEnvironmentPresets` /
   `CEnvironmentPresetTrigger` presets (all proven — each is registered by name-hash through
   `FUN_140f27f60("<Name>", <len>)` with a per-class descriptor `PTR_LAB_*` installed into the
   component factory `FUN_14085fd00` / `thunk_FUN_14cfef490`).
3. **Behavior-VM conditions (gating).** The behavior/condition VM (see `behavior_system.md`) exposes
   weather-aware conditions that gate player animation/state transitions:
   `CVerticalWindInAirHeightCondition`, `CVerticalWindInAirSpeedCondition`,
   `CTornadoOriginWithinHorizontalDistance`, `CTornadoOriginWithinVerticalDistance`,
   `CConditional_IsPlayerInWind`, `CConditional_IsPlayerInWeather`, plus the mech-weapon behavior state
   `CMechWindCannonSweepState` (all proven — type-id getters via `thunk_FUN_14aadec10` / registrar table
   `thunk_FUN_147cafcd0`).

There is also a **volumetric render pipeline** dedicated to the tornado (`TORNADO_COMPUTE`,
`TORNADO_RAYMARCH`, `TornadoMesh`, `TornadoPostprocess`, `Atmosphere_Tornado`), and a small
**gameplay/reward layer** that tracks "player is inside extreme weather X" for challenges (proven).

The four extreme-weather **types** are enumerated in the decomp: **sandstorm, tropical storm, tornado,
blizzard** (proven; see Notable constants).

## Key classes & functions

### World subsystems (managers)
| Class string | Registered at | Role |
|---|---|---|
| `CWindTunnelManager` | `FUN_148f960c0` @0x148f960c0 (line 4138757); parallel table line 3209275; class-tag `0x41cf8ea8` | Owns/updates the global wind-tunnel field (proven registration; role inferred) |
| `CTopographicalWind` | `FUN_148f960c0` (line 4138774); parallel line 3209292; class-tag `0x41ce3e70` | Terrain-driven ambient wind field (proven registration; role inferred) |
| `CPfxWindPhysicsSystem` | `FUN_148f960c0` (line 4138470); class-tag `0x41cb1660` | Applies wind to particle/props physics (proven registration; role inferred) |
| `CLightningManager` | `thunk_FUN_148f6e780(world,"CLightningManager",PTR_LAB_141d90888)` line 3209480 / 4138962 | Schedules & places lightning strikes (proven registration; role inferred) |

### RTPC entity components (placed weather objects)
| Class string (len) | Registrar `FUN_` | Name-hash slot / descriptor | Role |
|---|---|---|---|
| `CTornadoObject` (0xe) | `FUN_1407dbf30` @0x1407dbf30 | `DAT_142cb8e94` | The tornado entity/volume |
| `CTornadoController` (0x12) | `FUN_1407dbe10` @0x1407dbe10 | `DAT_142cb8e8c` | Drives tornado state/movement |
| `CTornadoFan` (0xb) | `FUN_1407dbea0` @0x1407dbea0 | `DAT_142cb8ec8` | Directed wind emitter ("fan") |
| `CTornadoReachPoint` (0x12) | `FUN_1407dbfc0` @0x1407dbfc0 | `DAT_142cb8ed0` | Path/target node the tornado moves toward |
| `CTornadoSpawnPoint` (0x12) | `FUN_1407dc050` @0x1407dc050 | `DAT_142cb8ed8` | Where a tornado can spawn |
| `CTornadoTrigger` (0xf) | `FUN_1407dc0e0` @0x1407dc0e0 | `DAT_142cb8e9c` | Volume that starts/gates a tornado |
| `CStormObject` (0xc) | `FUN_1407dbb40` @0x1407dbb40 | `DAT_142cb8dc0` | Storm-area entity |
| `CStormRadiusControl` (0x13) | `FUN_1407dbbd0` @0x1407dbbd0 | `DAT_142cb8e64` | Controls storm radius |
| `CLocalWindObject` (0x10) | `FUN_1407dadc0` @0x1407dadc0 | `DAT_142cb8d58` | Local (placed) wind volume |
| `CWindTunnelObject` (0x11) | `FUN_1402ac4e0` @0x1402ac4e0 | `DAT_142cb0b60` | Placed wind-tunnel volume |
| `CForcePulse` (0xb) | `FUN_14080c5b0` @0x14080c5b0 | `DAT_142cb91e0` / `PTR_LAB_141d9b5e8` | One-shot radial force impulse |
| `CForceField` (0xb) | `FUN_14080c430` @0x14080c430 | `DAT_142cb959c` / `PTR_LAB_141d9b610` | Persistent force volume |
| `CForcePoint` (0xb) | `FUN_14080c4f0` @0x14080c4f0 | `DAT_142cb95a4` / `PTR_LAB_141d9b638` | Point force source |
| `CLightningObject` (0x10) | `FUN_1407dac10` @0x1407dac10 | `DAT_142cb8d48` | A single lightning strike instance |
| `CLightningStrikeController` (0x1a) | `FUN_1407daca0` @0x1407daca0 | `DAT_142cb8d50` | Controls lightning strike emission |
| `CEnvironmentPresets` (0x13) | `FUN_14080c070` @0x14080c070 | `DAT_142cb0abc` / `PTR_LAB_141d9e068` | Set of environment presets |
| `CEnvironmentPresetTrigger` (0x19) | `FUN_1406b7450` @0x1406b7450 | `DAT_142cb80d8` / `PTR_LAB_141d9e040` | Trigger volume that switches preset |
| `CWeatherPreset` (0xe) | `FUN_140844920` @0x140844920 | `DAT_142cb9180` | A named weather preset |
| `CObjectiveParam_TornadoWidget` (0x1d) | `FUN_1408445b0` @0x1408445b0 | `DAT_142cb9514` | UI/objective widget for tornado missions |

(All rows proven: the class name + length string is passed to `FUN_140f27f60` in the cited function.)

### Behavior-VM conditions & states
There are **two distinct condition frameworks** (proven): (A) behavior-tree `…Condition` classes, interned
via `thunk_FUN_14aadec10("<name>")`, registered `{ctor,size}` in table `FUN_1485b9730` and torn down in
`FUN_1485e07e0`; and (B) quest/DSF `CConditional_*` classes, interned via `FUN_140f27f60`, mapped
name→factory in `FUN_148f99a90` via `thunk_FUN_147cafcd0`.

**Framework A — threshold conditions** (ctor label + object size read directly from `FUN_1485b9730`):
| Class string | Type-id getter | Slot | ctor stub | obj size | Role |
|---|---|---|---|---|---|
| `CVerticalWindInAirHeightCondition` | `FUN_14054de90` @0x14054de90 | `DAT_142cb4294` | `LAB_14052a3b0` | **0x20** | Gate on player height while airborne in wind |
| `CVerticalWindInAirSpeedCondition` | `FUN_14054df10` @0x14054df10 | `DAT_142cb41cc` | `LAB_14052a3f0` | **0x18** | Gate on player vertical speed while airborne in wind |
| `CTornadoOriginWithinHorizontalDistance` | `FUN_14054ce10` @0x14054ce10 | `DAT_142cb4564` | `LAB_140529c30` | **0x18** | True if tornado origin within horizontal range |
| `CTornadoOriginWithinVerticalDistance` | `FUN_14054ce90` @0x14054ce90 | `DAT_142cb456c` | `LAB_140529c60` | **0x18** | True if tornado origin within vertical range |
| `CIsInWeatherCondition` (sibling) | `FUN_14054a010` @0x14054a010 | `DAT_142cb43d4` | — | — | Weather-type gate |

The **Height** condition is 8 bytes larger than the others (0x20 vs 0x18) — consistent with it carrying an
extra config field (inferred). `CVerticalWindInAirHeightCondition` also has an explicit `GetClassName`-style
accessor returning the literal string at `FUN_14304da6f` @0x14304da6f (proven).

**Framework B — boolean player-state queries** (registered as **size-8, vtable-pointer-only** objects — no
config fields — in `FUN_148f99a90`):
| Class string | Type-id getter | Slot | vtable | Role |
|---|---|---|---|---|
| `CConditional_IsPlayerInWind` | `FUN_140652c70` @0x140652c70 | `DAT_142cb5d78` | `PTR_LAB_141d92378` (installed line 4140062) | True when the player is inside a wind volume |
| `CConditional_IsPlayerInWeather` | `FUN_140652bd0` @0x140652bd0 | `DAT_142cb5d70` | `PTR_LAB_141d92658` (installed line 4140177) | True when player is inside weather |

**Mech behavior state**: `CMechWindCannonSweepState` — `FUN_14054ae10` @0x14054ae10 — `DAT_142cb40ac`
(wind-cannon sweep anim/behavior state).

## How it works (from the decomp)

### Component registration & pooling
Every weather object is an RTPC entity component. Registration is uniform (proven, e.g. `CForcePulse`
`FUN_14080c5b0`): allocate a 0x10-byte class descriptor whose vtable is `PTR_LAB_141d9b5e8`, lazily
compute the class name-hash via `FUN_140f27f60("CForcePulse", 0xb)` (this is `lookup3`/`hashlittle`, our
cracked name-hash — see `docs/formats/name_hash.md`), then install `(hash → descriptor)` into the
component factory via `thunk_FUN_14cfef490`. The factory master list is `FUN_14085fd00` (proven: it is
the common caller of the whole `FUN_14080c…` registrar family).

`CForcePulse` is additionally **object-pooled**: the gameplay/faction init `FUN_14085a3a0`
@0x14085a3a0 reserves a pool via `FUN_1402a78b0(pool, hash("CForcePulse",0xb), 0x1e)` and
`… hash("CBulletSpawner",0xe), 0x0a` (proven, lines 913153–913156). **Force-pulse pool size = 0x1e = 30**
instances; bullet-spawner pool = 10.

### Wind volumes
`CLocalWindObject` (`FUN_1407dadc0`) and `CWindTunnelObject` (`FUN_1402ac4e0`) are the *placed* wind
volumes; `CTopographicalWind` and `CWindTunnelManager` are the *global* wind fields; `CPfxWindPhysicsSystem`
couples wind into particle/prop physics (all proven as registered classes/subsystems). The precise field
layout (direction vector, strength, radius/extents, falloff) and the per-frame sampling/aggregation math
were **not resolved in this pass** — see Open questions. Mechanistically, the behavior condition
`CConditional_IsPlayerInWind` (`FUN_140652c70`) is the query the gameplay layer uses to ask "is the player
currently inside any wind volume" (proven the class exists and is a behavior condition; the body of the
runtime evaluate was not captured here).

### Force application — "external force generators"
`CForcePulse` / `CForceField` / `CForcePoint` are the generic force primitives shared with the Havok
destruction system (they appear alongside `CHavokDestruction*` in the same registrar block — proven,
lines ~863961–864025, and `CForcePulse` is also cited in `destruction.md`). Architecturally, wind/force
sources are consumed by the physics tick as **external force generators**: the physics job has two
profiled phases, `CollectExternalForceGenerators` (near line ~2411273) and `ApplyExternalForces`
(near line ~2411758) — i.e. each frame the active force/wind volumes are gathered into a generator list
and then integrated onto the physics bodies (proven from the profiling-scope strings; the per-body impulse
math itself was not decoded). A tornado/fan/wind volume drives bodies by contributing to this generator
list (inferred from the architecture + the pooled 30 `CForcePulse` slots). The concrete impulse math
(magnitude · radius · falloff curve) was not decoded in this pass — see Open questions.

### Tornado & storm
The tornado is a **component graph**, not a single object (proven from the class family): a
`CTornadoObject` volume, a `CTornadoController` that drives it, `CTornadoFan` directed-wind emitters, and
navigation nodes `CTornadoReachPoint` / `CTornadoSpawnPoint` (so the tornado *travels* between authored
points) plus a `CTornadoTrigger` volume. Storms use `CStormObject` + `CStormRadiusControl` (an explicit
radius controller — proven by name). The controller/update bodies with the movement-speed, pull/lift force
and radius constants were not captured in this pass (Open questions).

The tornado has a **dedicated volumetric render path** (proven, all string constants present):
`TornadoCompute::LocalResources` (line 142620), `Tornado::LocalResources` (142864),
`TornadoMesh::LocalResources` (143025), `TornadoPostprocess::LocalResources` (143123), render passes
`"TORNADO_COMPUTE"` (1550901) and `"TORNADO_RAYMARCH"` (1551149), `"Atmosphere_Tornado"` (3321622), and a
`"LowResVolumetricsCompositeTornado"` composite (1505588). So the funnel is a raymarched low-res volumetric
with its own compute/mesh/postprocess stages.

### Mech "Wind Cannon" weapon
A mech mounts a **Wind Cannon** weapon that parallels the Gravity Gun. In the mech action-dispatch
`FUN_140609d80` @0x140609d80 (size 3136), when the engage flag `local_116` is set it queues the behavior
action `ACT_WIND_CANNON_ENGAGE` (`FUN_140f27f60("ACT_WIND_CANNON_ENGAGE",0x16)`, line 583902), and when
`local_115` is set it queues either `ACT_GRAVITY_GUN_ABORT` or `ACT_WIND_CANNON_DISENGAGE`
(`…"ACT_WIND_CANNON_DISENGAGE",0x19`, line 583915) chosen by the char at `[weapon+0x31e]` (proven). The
animation/behavior state for the sweep is `CMechWindCannonSweepState` (`FUN_14054ae10`), sitting next to
`CMechStrafingBlendState` in the mech state table (proven). Separately, **"WindCannon"** appears as a
three-tier **vehicle/gadget module** name-hash set — `FUN_143130710` @0x143130710 registers
`"WindCannon"` ×3 (lines 3125171–3125173), mirroring `SideBooster1..3` (`FUN_143130360`) and
`Shotgun1..3` — i.e. an upgradeable vehicle gadget slot with 3 tiers (proven strings; "3 tiers"
inferred from the ×3 pattern shared with the other modules).

### Lightning strikes
Lightning is **scan/event-driven, not a random countdown** (proven). The controller
(`CLightningStrikeController`, runtime field-registration `FUN_14080d790`, descriptor
`PTR_LAB_141d9cb00`) each tick scans a global candidate-object list (`DAT_142cb1988`…`DAT_142cb1990`,
lock `DAT_142cb19a0`) and picks the **nearest eligible object within radius**:

- dispatch chain `FUN_140344190` → `FUN_1407f7d80` → `FUN_1407e4ba0`, and `FUN_1407f7ed0` →
  `FUN_1407d5140`; strike executed by `FUN_1407f7320` @0x1407f7320 (all proven).
- Per-candidate eligibility (proven, read in the scan loops `FUN_1407f7ed0` / `FUN_1407e4ba0`):
  `*(byte*)(obj+0x1ca) & 0x40` set (lightning-targetable), `*(byte*)(obj+0x1e8) != 0` (enabled),
  `*(float*)(obj+0x1ec) > 0` (strike weight). 2D distance from `(obj+0x134 = X, obj+0x13c = Z)` to the
  scan origin; candidate accepted when `dist < *(float*)(obj+0x1f0)` (**per-object strike radius**) **and**
  `dist < DAT_141ca71fc` (**global max search distance**); nearest wins. `FUN_1407f7ed0` adds a controller
  gate `dist < *(float*)(controller+0x2f8)`.
- A per-target **active-strike registry map `DAT_142cb8140`** (keyed by object handle) prevents duplicate
  concurrent strikes on the same target (proven).
- `FUN_1407f7320` fires gameplay event `"on.lightning.strike.target"` (`thunk_FUN_147625370`, flag 0x106,
  line 851946), writes the strike xyz into controller `+0x36c/+0x370/+0x374` (busy byte `+0x368`), and
  plays the strike VFX via the environment/effects singleton `DAT_142cada68`
  (`thunk_FUN_147a50f10(DAT_142cada68,…)`). Damage-on-hit is applied by the *listener* of that event,
  subscribed in `FUN_149139c10` @0x149139c10 (`thunk_FUN_14762d8a0(obj+0x30,"on.lightning.strike.target",
  0x106,1)`); the damage value itself was not reached (inferred path). The `CLightningManager` world
  subsystem owns the controller set (registered `thunk_FUN_148f6e780(...,"CLightningManager",
  PTR_LAB_141d90888)`; its own vtable/update is data-only and not walkable from this code dump).

### Weather presets & environment switching
`CWeatherPreset` (`FUN_140844920`), `CEnvironmentPresets` (`FUN_14080c070`) and
`CEnvironmentPresetTrigger` (`FUN_1406b7450`) form the preset system: an `CEnvironmentPresets` set holds
presets, and an `CEnvironmentPresetTrigger` volume switches the active one when entered. The
`CEnvironmentPresets` *instance* constructor is `FUN_14029c9b0` @0x14029c9b0 (vtables
`PTR_LAB_141cf0e50` / `PTR_FUN_141cf0e68`): it holds a **blend-alpha field at instance offset 0x1e0
(`param_1[0x3c]`), default `1.0`** (`0x3f800000`), with `0xdeadbeef` init sentinels at `+0x22c`/`+0x230`,
and builds a debug name `"CEnvironmentPresets <counter>"` (proven). The preset "should-apply" dispatch
`FUN_1402996f0` @0x1402996f0 mirrors the lightning dispatch shape (a virtual query
`(**(obj+0x10))(obj,&DAT_141cf0a38)`). The **trigger volume enter/exit hooks** are registered in the world
init `FUN_1401bef80` @0x1401bef80: `"weather_enter_volume"` (flag 0x505, line 125794) and
`"weather_exit_volume"` (line 125795) — these are how a `CEnvironmentPresetTrigger` drives a preset change.
The exact blend *duration* source and preset *selection* on enter are reached through the enter-volume
virtual and were not fully traced (Open questions). The active
environment/weather state lives on a **world-environment singleton `DAT_142cada68`**, created in
`FUN_14020d610` @0x14020d610 (size 1228) and cached at `gameworld+0x4c8` (proven, line 146162). Observed
fields on that singleton: `+0x91c` a sky/weather color block (line 144716), `+0x1150` the current
extreme-weather-type enum (line 1247246), `+0x117c` a clamped normalized weather-intensity float written by
the ramp in the function ending at line 821863 (`fVar5 = clamp(...) ; [singleton+0x117c] = fVar5 *
DAT_141ca9c7c`), and `+0x11c0` a subsystem pointer (line 3209-adjacent uses). (Singleton *identity* as the
environment/weather manager is inferred; the individual offset reads/writes are proven at the cited lines.)

### Extreme-weather type tracking (gameplay/reward)
`FUN_140aa7610` @0x140aa7610 reads the enum at `*(int *)(DAT_142cada68 + 0x1150)` and records a
challenge/reward-progress id per type (proven):

```
enum extreme_weather_type at [env_singleton + 0x1150]:
  3 -> "rp_extreme_weather_in_sandstorm"       (line 1247254)
  4 -> "rp_extreme_weather_in_tropical_storm"  (line 1247263)
  5 -> "rp_extreme_weather_in_tornado"         (line 1247272)
  6 -> "rp_extreme_weather_in_blizzard"        (line 1247282)
```

So the four authored extreme-weather types are **sandstorm(3), tropical storm(4), tornado(5),
blizzard(6)**, and simply *being inside* each is tracked for challenges (`rp_` = reward-progress). An
audio/event hook `"player.near.tornado"` fires from `FUN_140b344d0` @0x140b344d0 (line 1327106) and is
registered with threshold `0xff` near line 4380345 (proven). The campaign finale is tornado-themed:
achievement `"ach_complete_campaign_tornado"` (lines 2358, 990526) (proven).

## Data & config integration

- Every class above is an **RTPC (.epe) entity component / behavior node** keyed by its `lookup3`
  name-hash (`FUN_140f27f60`), consistent with `docs/formats/name_hash.md` and memory
  `[[rtpc-entity-assembly]]` / `[[composite-assets]]`. A tornado/storm/wind/force/lightning "object" in a
  world archive is an entity carrying one of these components, configured by ADF/RTPC properties. (proven
  registration path; property field mapping not decoded here.)
- The `CWeatherPreset` / `CEnvironmentPresets` data is authored as preset assets applied to the
  `DAT_142cada68` environment singleton (inferred).
- Behavior conditions come in two flavors (see Key classes): Framework-A threshold conditions
  (`CVerticalWindInAir*`, `CTornadoOriginWithin*`) are non-trivial-size objects (0x18–0x20) whose
  threshold is a **data-driven ADF/RTPC config field** on the condition instance — there is **no hardcoded
  `< DAT_float` compare in the exported code** for them, so the actual updraft height / vertical-speed /
  tornado-distance limits live in the behavior-tree asset, not the binary (inferred: the ctor is a bare
  allocate-`size` + install-vtable, and no inline literal was found). Framework-B queries
  (`CConditional_IsPlayerInWind`, `CConditional_IsPlayerInWeather`) are **size-8, field-less** — pure
  boolean runtime lookups against wind/weather volumes (proven size; the lookup helper was not located).
- `CForcePulse`/`CForceField`/`CForcePoint` are shared with `destruction.md` (Havok destruction) — the
  same force primitives serve both weather and chaos-object destruction (proven co-registration).

## Notable constants / tunables

| Constant | Value | Where (FUN_ / line) | Grade |
|---|---|---|---|
| `CForcePulse` pool size | `0x1e` = 30 | `FUN_14085a3a0` line 913154 | proven |
| `CBulletSpawner` pool size | 10 | `FUN_14085a3a0` line 913156 | proven |
| Extreme-weather enum: sandstorm | 3 | `FUN_140aa7610` line 1247247/1247254 | proven |
| Extreme-weather enum: tropical storm | 4 | `FUN_140aa7610` line 1247256/1247263 | proven |
| Extreme-weather enum: tornado | 5 | `FUN_140aa7610` line 1247265/1247272 | proven |
| Extreme-weather enum: blizzard | 6 | `FUN_140aa7610` line 1247275/1247282 | proven |
| Extreme-weather-type field offset | `env_singleton + 0x1150` | `FUN_140aa7610` line 1247246 | proven |
| Weather-intensity field offset | `env_singleton + 0x117c` | ramp fn ending line 821863 | proven |
| Weather color/sky block offset | `env_singleton + 0x91c` | line 144716 | proven |
| `"WindCannon"` gadget tiers | 3 (×3 registration) | `FUN_143130710` lines 3125171–73 | proven (count); tiers inferred |
| Mech wind-cannon engage action | `ACT_WIND_CANNON_ENGAGE` | `FUN_140609d80` line 583902 | proven |
| Mech wind-cannon disengage action | `ACT_WIND_CANNON_DISENGAGE` | `FUN_140609d80` line 583915 | proven |
| World-subsystem class-tags | WindTunnelMgr `0x41cf8ea8`, TopoWind `0x41ce3e70`, PfxWind `0x41cb1660` | `FUN_148f960c0` lines 4138470–4138774 | proven |
| Threshold-condition object sizes | Height 0x20; Speed/TornadoH/TornadoV 0x18 | table `FUN_1485b9730` | proven |
| Framework-B condition object size | 8 (vtable ptr only, no config) | `FUN_148f99a90` | proven |
| Env-preset blend-alpha field & default | offset 0x1e0, default `1.0` (`0x3f800000`) | `FUN_14029c9b0` | proven |
| Lightning candidate strike-radius field | `obj + 0x1f0` (float) | `FUN_1407f7ed0`/`FUN_1407e4ba0` | proven |
| Lightning candidate strike-weight field | `obj + 0x1ec` (float, must be >0) | scan loops | proven |
| Lightning eligible flag | `obj + 0x1ca` bit `0x40` | scan loops | proven |
| Lightning global max search distance | `DAT_141ca71fc` (value in rodata, not in dump) | scan loops | proven (addr) |
| Lightning controller max-distance gate | `controller + 0x2f8` (float) | `FUN_1407f7ed0` | proven |
| Weather trigger enter/exit events | `"weather_enter_volume"` / `"weather_exit_volume"` (flag 0x505) | `FUN_1401bef80` lines 125794–95 | proven |
| Lightning strike event | `"on.lightning.strike.target"` (flag 0x106) | `FUN_1407f7320` line 851946 | proven |

> No force magnitudes, wind speeds, tornado radii/pull, lightning distances/damage, or condition thresholds
> are given as literal numbers above: they live either in `.rodata` (the `DAT_141ca…` float symbols the code
> only names) or in ADF/RTPC config assets (data-driven), neither present in this code-only dump. Field
> *offsets* and *symbol addresses* are proven; their runtime *values* need x64dbg/asset reads. Nothing is
> guessed.

## Call-graph highlights

- Component factory master: `FUN_14085fd00` → the `FUN_14080c…` registrar family (`CForcePulse`
  `FUN_14080c5b0`, `CForceField` `FUN_14080c430`, `CForcePoint` `FUN_14080c4f0`, `CEnvironmentPresets`
  `FUN_14080c070`) → name-hash `FUN_140f27f60` + install `thunk_FUN_14cfef490`. (proven)
- World-manager registration table `FUN_148f960c0` installs `CWindTunnelManager`, `CTopographicalWind`,
  `CPfxWindPhysicsSystem`; `thunk_FUN_148f6e780` installs `CLightningManager`. (proven)
- Tornado/storm/lightning component getters `FUN_1407db…`/`FUN_1407dac…` are self-contained lazy
  name-hash initializers (`callers=[]` — reached indirectly through the factory). (proven)
- Behavior-condition (Framework A) type-id getters (`FUN_14054de90` etc.) are called by the registration
  table `FUN_1485b9730` (installs `{ctor,size}`) and the teardown pass `FUN_1485e07e0`. Framework-B
  `CConditional_*` are name→factory-mapped in `FUN_148f99a90` via `thunk_FUN_147cafcd0`. (proven)
- Lightning: `FUN_140344190` → `FUN_1407f7d80` → `FUN_1407e4ba0` (scan); `FUN_1407f7ed0` →
  `FUN_1407d5140` (registry lookup `DAT_142cb8140`); `FUN_1407f7320` (execute + event); listener
  `FUN_149139c10`. (proven)
- Presets: instance ctor `FUN_14029c9b0`; should-apply dispatch `FUN_1402996f0`; trigger enter/exit
  events registered in world init `FUN_1401bef80`. (proven)
- Physics consumes force/wind volumes via `CollectExternalForceGenerators` → `ApplyExternalForces`
  (profiled phases in the physics job, lines ~2411273 / ~2411758). (proven)
- Mech input/action driver `FUN_140609d80` queues the wind-cannon actions via `thunk_FUN_149f7ddd0`.
  (proven)
- Extreme-weather challenge recorder `FUN_140aa7610` reads the env singleton and dispatches through the
  record system (`RecordUpgradePurchased`-style, `FUN_140aab200` nearby). (proven)

## Open questions / lower-confidence

1. **Wind-volume field layout & sampling math.** The direction vector / strength / radius / falloff fields
   of `CLocalWindObject`, `CWindTunnelObject`, and how `CTopographicalWind` / `CWindTunnelManager` /
   `CPfxWindPhysicsSystem` sample and aggregate wind at a world position — not decoded. Route: walk the
   descriptor vtables (`DAT_142cb8d58` etc.) to the constructor/reflection and the per-frame update, and
   the `CollectExternalForceGenerators` phase (~line 2411273).
2. **Force impulse math & constants.** The magnitude/radius/falloff-curve applied to physics bodies during
   `ApplyExternalForces` (and thus the tornado's pull/lift on the player and objects) — architecture proven
   but per-body math not captured. Cross-check with `destruction.md`.
3. **Tornado controller update.** `CTornadoController` movement speed between `CTornadoReachPoint`s, funnel
   radius, lift force, and damage; `CStormRadiusControl` growth/curve — undecoded. Route: descriptors
   `DAT_142cb8e8c` / `DAT_142cb8e64`.
4. **Vertical-wind-in-air / tornado-distance thresholds.** Now known to be **data-driven** (ADF/RTPC config
   fields, not inline constants): the updraft height (`CVerticalWindInAirHeightCondition`, obj size 0x20),
   vertical-speed (`…SpeedCondition`, 0x18) and `CTornadoOriginWithin*Distance` (0x18) limits are read from
   the behavior-tree asset. To get default values, read the ADF type descriptor for these class names from
   game data. The evaluate virtuals live in `.rdata` vtables installed by stubs `LAB_14052a3b0/3f0`,
   `LAB_140529c30/c60` — resolvable in Ghidra's listing view but absent from this code-only dump.
5. **Lightning damage value & interval.** Selection is fully resolved (nearest eligible object within
   radius, per-tick, dedup via registry `DAT_142cb8140` — no countdown timer found). Still open: the literal
   values of `DAT_141ca71fc` / `DAT_141cca7dc` / `DAT_141ca7170` / `DAT_141ca70ac` (rodata floats) and the
   damage applied by the `"on.lightning.strike.target"` listener `FUN_149139c10`.
6. **Preset apply/blend.** Enter/exit hooks (`weather_enter_volume`/`_exit_volume`, `FUN_1401bef80`) and the
   blend-alpha field (env-preset +0x1e0, default 1.0) are known; the blend *duration* and *which-preset*
   selection run through the enter-volume virtual and were not fully traced.
7. **`CConditional_IsPlayerInWind` evaluate body.** Confirmed size-8 field-less condition
   (`FUN_140652c70`, `PTR_LAB_141d92378`); the runtime wind-volume lookup helper it calls was not located.
8. **`CLightningManager` update loop.** Manager vtable `PTR_LAB_141d90888` is data-only; its per-frame tick
   that drives the controllers was not walkable from the code dump.

## Appendix — decomp anchors

Strings and addresses used above (all in `output/_ghidra_jc4/jc4_all_functions_decomp.txt`):

**Managers / subsystems**
- `"CWindTunnelManager"` — `FUN_148f960c0` line 4138757; table line 3209275; tag `0x41cf8ea8`
- `"CTopographicalWind"` — line 4138774 / 3209292; tag `0x41ce3e70`
- `"CPfxWindPhysicsSystem"` — line 4138470; tag `0x41cb1660`
- `"CLightningManager"` — line 3209480 / 4138962 (`PTR_LAB_141d90888`); also 4238045, 4333943

**RTPC components** (registrar `FUN_` → name-hash slot)
- `"CLocalWindObject"` 0x10 — `FUN_1407dadc0` line 840194 — `DAT_142cb8d58`
- `"CWindTunnelObject"` 0x11 — `FUN_1402ac4e0` line 199708 — `DAT_142cb0b60`
- `"CForcePulse"` 0xb — `FUN_14080c5b0` line 864025 — `DAT_142cb91e0` / `PTR_LAB_141d9b5e8`; usage `FUN_14085a3a0` line 913153; `FUN_140f27f60("CForcePulse",0xb)` also line 1232201
- `"CForceField"` 0xb — `FUN_14080c430` line 863961 — `DAT_142cb959c` / `PTR_LAB_141d9b610`
- `"CForcePoint"` 0xb — `FUN_14080c4f0` line 863993 — `DAT_142cb95a4` / `PTR_LAB_141d9b638`
- `"CTornadoObject"` 0xe — `FUN_1407dbf30` line 840909 — `DAT_142cb8e94`
- `"CTornadoController"` 0x12 — `FUN_1407dbe10` line 840863 — `DAT_142cb8e8c`
- `"CTornadoFan"` 0xb — `FUN_1407dbea0` line 840886 — `DAT_142cb8ec8`
- `"CTornadoReachPoint"` 0x12 — `FUN_1407dbfc0` line 840932 — `DAT_142cb8ed0`
- `"CTornadoSpawnPoint"` 0x12 — `FUN_1407dc050` line 840955 — `DAT_142cb8ed8`
- `"CTornadoTrigger"` 0xf — `FUN_1407dc0e0` line 840978 — `DAT_142cb8e9c`
- `"CStormObject"` 0xc — `FUN_1407dbb40` line 840748 — `DAT_142cb8dc0`
- `"CStormRadiusControl"` 0x13 — `FUN_1407dbbd0` line 840771 — `DAT_142cb8e64`
- `"CLightningObject"` 0x10 — `FUN_1407dac10` line 840125 — `DAT_142cb8d48`
- `"CLightningStrikeController"` 0x1a — `FUN_1407daca0` line 840148 — `DAT_142cb8d50`
- `"CEnvironmentPresets"` 0x13 — `FUN_14080c070` line 863801 — `DAT_142cb0abc` / `PTR_LAB_141d9e068`; also lines 198701, 192313
- `"CEnvironmentPresetTrigger"` 0x19 — `FUN_1406b7450` line 675476 — `DAT_142cb80d8` / `PTR_LAB_141d9e040`; also line 863769
- `"CWeatherPreset"` 0xe — `FUN_140844920` line 899960 — `DAT_142cb9180`; also line 4140687
- `"CObjectiveParam_TornadoWidget"` 0x1d — `FUN_1408445b0` line 899860 — `DAT_142cb9514`

**Behavior conditions / states** (Framework A reg table `FUN_1485b9730`, teardown `FUN_1485e07e0`;
Framework B name→factory map `FUN_148f99a90`)
- `"CVerticalWindInAirHeightCondition"` — `FUN_14054de90` line 506329 — `DAT_142cb4294`; ctor stub `LAB_14052a3b0`, size 0x20; class-name accessor `FUN_14304da6f` line 3109545
- `"CVerticalWindInAirSpeedCondition"` — `FUN_14054df10` line 506348 — `DAT_142cb41cc`; ctor `LAB_14052a3f0`, size 0x18
- `"CTornadoOriginWithinHorizontalDistance"` — `FUN_14054ce10` line 505702 — `DAT_142cb4564`; ctor `LAB_140529c30`, size 0x18
- `"CTornadoOriginWithinVerticalDistance"` — `FUN_14054ce90` line 505721 — `DAT_142cb456c`; ctor `LAB_140529c60`, size 0x18
- `"CIsInWeatherCondition"` (sibling) — `FUN_14054a010` — `DAT_142cb43d4`
- `"CConditional_IsPlayerInWind"` — `FUN_140652c70` line 619466; registrar line 4140062 (vtable `PTR_LAB_141d92378`, size 8) — `DAT_142cb5d78`; also 5293686
- `"CConditional_IsPlayerInWeather"` — `FUN_140652bd0` line 619441 — `DAT_142cb5d70`; registrar line 4140177 (vtable `PTR_LAB_141d92658`, size 8)
- `"CMechWindCannonSweepState"` — `FUN_14054ae10` line 504486 — `DAT_142cb40ac`

**Lightning pipeline**
- runtime field-reg: `CLightningObject` `FUN_14080d6d0` (`PTR_LAB_141d9cad8`); `CLightningStrikeController` `FUN_14080d790` (`PTR_LAB_141d9cb00`)
- dispatch: `FUN_140344190` → `FUN_1407f7d80` → `FUN_1407e4ba0`; `FUN_1407f7ed0` → `FUN_1407d5140`; execute `FUN_1407f7320`
- candidate list `DAT_142cb1988`/`DAT_142cb1990`, lock `DAT_142cb19a0`; active-strike registry `DAT_142cb8140`
- object fields: `+0x134` X, `+0x13c` Z, `+0x1ca`&0x40 eligible, `+0x1e8` enabled, `+0x1ec` weight, `+0x1f0` radius; controller `+0x2f8` max-dist, `+0x368` busy, `+0x36c/0x370/0x374` strike xyz
- constants (rodata, addr only): `DAT_141ca71fc`, `DAT_141cca7dc`, `DAT_141ca7170`, `DAT_141ca70ac`, `DAT_141ca6cac`
- event `"on.lightning.strike.target"` (flag 0x106) `FUN_1407f7320` line 851946; listener `FUN_149139c10`

**Presets / physics**
- `CEnvironmentPresets` instance ctor `FUN_14029c9b0` (vtables `PTR_LAB_141cf0e50`/`PTR_FUN_141cf0e68`; blend-alpha +0x1e0 default 1.0; debug name line 192313)
- should-apply dispatch `FUN_1402996f0`; env-preset trigger events `FUN_1401bef80` lines 125794–95 (`weather_enter_volume`/`weather_exit_volume`, flag 0x505)
- physics external-force phases: `CollectExternalForceGenerators` ~line 2411273; `ApplyExternalForces` ~line 2411758

**Mech / gadget / weather-type / render**
- `"ACT_WIND_CANNON_ENGAGE"` line 583902; `"ACT_WIND_CANNON_DISENGAGE"` line 583915 — `FUN_140609d80`
- `"WindCannon"` ×3 — `FUN_143130710` lines 3125171–3125173
- extreme-weather ids — `FUN_140aa7610` lines 1247254 / 1247263 / 1247272 / 1247282; enum read line 1247246
- env singleton — created `FUN_14020d610` line 146162; reads `DAT_142cada68` at lines 143131, 144716, 821855–821863, 1247246
- volumetric tornado render — `TornadoCompute::LocalResources` 142620, `Tornado::LocalResources` 142864, `TornadoMesh::LocalResources` 143025, `TornadoPostprocess::LocalResources` 143123, `"TORNADO_COMPUTE"` 1550901, `"TORNADO_RAYMARCH"` 1551149, `"Atmosphere_Tornado"` 3321622, `"LowResVolumetricsCompositeTornado"` 1505588
- `"player.near.tornado"` — `FUN_140b344d0` line 1327106; register line 4380345
- `"ach_complete_campaign_tornado"` — lines 2358, 990526

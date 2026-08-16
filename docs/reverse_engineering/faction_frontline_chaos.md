# Frontline / Agency / Chaos metagame — JC4's "Army of Chaos" region-liberation loop

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

## Overview

The "Army of Chaos" metagame is JC4's persistent region-control layer: the player generates **chaos**
(destruction + activities), which advances a moving **frontline** across the map, converting hostile
**tactical nodes** into **secured territory**, unlocking content and progression tracks. From the
functions-only decomp this system presents as a **cluster of singleton managers plus RTPC entity
components plus behaviour-VM conditionals**, all installed through the same three registrars documented in
`behavior_system.md`. Concretely the decomp proves:

- **The manager roster** — `CFrontlineManager`, `CTacticalNodeManager`, `CAgencyBaseManager`,
  `CDiscoveryManager`, `CHeatManager`, `CRewardSystem`, `CMediaRevolutionManager`, `CDemonAreaManager` —
  is registered as 8-byte singleton prototypes in the manager registrar `FUN_148f960c0`. (proven)
- **The frontline itself is a spatial component network** — `CFrontlineConnection` (edges),
  `CFrontlinePoint` (nodes), `CFrontlineEffectsAreaObject` (VFX band), `CFrontlineSpawnBlocker`, routed by
  `CFrontlineEventRouter` — registered as RTPC entity components (0x10-byte prototypes) in the component
  registrar `FUN_14085fd00`. (proven)
- **Chaos is accrued in two buckets** — *occupied territory* and *frontlines* — tracked as `i64`
  accumulators and flushed to a "chaos" milestone telemetry event; this schema is read directly from the
  decomp (`FUN_140a926a0`, `FUN_140a20c40`). (proven)
- **Content gating is done by the behaviour conditional VM** — a ~24-strong `CConditional_*` node/territory
  vocabulary plus `HasEnoughChaos`, `HeatUnitCount`, `DemonDomePercentage`, `IsAgencyRingCourse*` —
  registered in the conditional master registry `FUN_148f99a90`. (proven registration; evaluate bodies walled)

**Methodology caveat (important):** this is a *functions-only* export. The manager and component **tick
bodies live behind data-section vtables** (`PTR_LAB_141d90*` for managers, `PTR_LAB_141d9b*/9e*` for
components) that are **not** in the dump. So the *frontline advance/retreat integrator, the node
state-transition rules, and every tunable magnitude* (chaos-per-kill, secure thresholds, heat-decay timers)
are **walled** — they sit in those vtables, in RTPC `.epe` property tables, or in ADF config. What IS fully
recovered: the class/registration skeleton, the name→hash identities, and the reachable **telemetry/UI
glue** code, which incidentally exposes the chaos economy's data model. Every walled item below is tagged
with its resolution route.

## Key classes & functions

### Singleton managers (registered in `FUN_148f960c0`, 8-byte vtable-only prototypes)

| Class string | vtable (data-section) | Role (inferred from name) |
|---|---|---|
| `CFrontlineManager` | `PTR_LAB_141d907e8` | Owns/advances the frontline network; the metagame's spine. (proven reg) |
| `CTacticalNodeManager` | `PTR_LAB_141d90748` | Owns the tactical-node graph (secure/clear/discover state). (proven reg) |
| `CAgencyBaseManager` | `PTR_LAB_141d90f48` | Owns agency-ring courses / agency bases. (proven reg) |
| `CDiscoveryManager` | `PTR_LAB_141d90d88` | Owns discovery volumes / location discovery. (proven reg) |
| `CHeatManager` | `PTR_LAB_141d90808` | Wanted/heat level + heat AOO. (proven reg) |
| `CRewardSystem` | `PTR_LAB_141d90768` | Grants rewards (incl. `CChaosReward`). (proven reg) |
| `CMediaRevolutionManager` | `PTR_LAB_141d90788` | "Media Revolution" backer-progression track fed by chaos. (proven reg) |
| `CDemonAreaManager` | `PTR_LAB_141d90f28` | DLC demon-dome / infestation areas. (proven reg) |

The registrar body is 3-line-idiom repeats: `alloc(8)` → `*p = &PTR_LAB_…` → `thunk_FUN_148f6e780(tbl,
"CName", p)` (name→prototype insert). The same roster is re-registered by a second near-identical registrar
in the `0x1490…`/`~line 3209030` region (same vtables). (proven — `FUN_148f960c0` @ decomp line 4138520+.)

### Frontline RTPC entity components (registered via `FUN_14085fd00`, 0x10-byte prototypes)

| Class string | reg thunk `FUN_` | vtable | name-hash cache DAT_ |
|---|---|---|---|
| `CFrontlineConnection` | `FUN_14080c670` | `PTR_LAB_141d9bed0` | `DAT_142cb97ac` |
| `CFrontlineEffectsAreaObject` | `FUN_14080c730` | `PTR_LAB_141d9ba48` | `DAT_142cb8a18` |
| `CFrontlinePoint` | `FUN_14080c7f0` | `PTR_LAB_141d9bef8` | `DAT_142cb97b4` |
| `CFrontlineSpawnBlocker` | `FUN_14080c8b0` | `PTR_LAB_141d9ec20` | `DAT_142cb9914` |
| `CFrontlineEventRouter` | getter `FUN_140845d80` | (via registrar) | `DAT_142cb9634` |

Each reg thunk: `p = alloc(0x10)` → `*p = &PTR_LAB_…` → cache `hash("CFrontline…")` → register into the
component table via `thunk_FUN_14cfef490(table, hash, p)`. Note `CFuelLineConnection` (`FUN_14080c970`,
vtable `PTR_LAB_141d9c948`) is registered **immediately adjacent** — the frontline "connection" reuses the
same wire/edge component family as fuel lines. (proven — decomp lines 864035–864191.)

### Agency-ring components / classes

| Class string | getter `FUN_` | name-hash cache DAT_ | Role |
|---|---|---|---|
| `CAgencyRing` | `FUN_140844af0` | `DAT_142cb9e30` | RTPC component: one ring in a course. (proven reg) |
| `CAgencyRingGroup` | `FUN_140844b80` | `DAT_142cb9e28` | RTPC component: a ring-course group. (proven reg) |
| `CAgencyRingEffectData` | `FUN_140843ac0` | `DAT_142cb9574` | Ring VFX/data descriptor. (proven reg) |
| `CObjectiveParam_AgencyRingCourse` | (conditional/objective registry) | — | Objective param binding a ring course to an objective. (proven reg, line 899685) |

Component consumer/construction sites: `CAgencyRing` getter is called from `FUN_148d6e1a0` + a `0x148ea53…`
reflection site; `CAgencyRingGroup` from `FUN_148d6e490` + `0x148ea54f0`. (proven callers.)

### Chaos / reward / progression classes

| Class string | getter `FUN_` | name-hash cache | Role |
|---|---|---|---|
| `CChaosReward` | `FUN_1409478f0` | `DAT_142cba720` | ADF reward descriptor granted at chaos milestones. (proven reg) |
| `CBookmarkChaosDescription` | (bookmark reg block, line 900262) | — | Map-UI bookmark describing a chaos location. (proven reg) |
| `CMediaRevolutionMilestone` / `…Reward` / `…Threshold` | reg block lines 865531/986608 | — | Backer "Media Revolution" progression fed by chaos. (proven reg) |

### Behaviour-VM conditionals (registered in `FUN_148f99a90`, 8-byte prototypes; evaluate bodies walled)

Node / territory / faction vocabulary (all confirmed present in the registry body, decomp lines
4139962–4140782, and each with a per-class name-hash getter `FUN_1406…`):

`CConditional_FactionStatus`, `…IsInSecuredTerritory`, `…IsLocationInSecuredTerritory`,
`…IsNodeSecured`, `…IsNodeCleared`, `…IsNodeDiscovered`, `…IsNodeTracked`, `…IsNodeUnsafe`,
`…IsNodeNeighborSecured/Cleared/Unsafe`, `…IsInClearedNode`, `…IsLocationComplete`,
`…IsLocationDiscovered`, `…LocationTypeProgression`, `…TacticalConnectionStatus`,
`…IsNearActiveFrontline` (getter `FUN_140843c90`, cache `DAT_142cb94dc`), `…IsPOILocked`,
`…IsPlayerInsideNode`, `…IsPlayerInsideNodeDiscoveryVolume`, `…IsPlayerInsideBiome`,
`…IsPlayerInsideInfestationZone`, `…DemonDomePercentage`, `…IsHoveringOverDiscoveryLocation`.

Economy / heat / agency gates: `CConditional_HasEnoughChaos` (getter `FUN_14063d6e0`),
`CConditional_HeatUnitCount`, `CConditional_IsPlayerInHeatAOO`, `CConditional_IsAgencyRingCourseActive`,
`CConditional_IsAgencyRingCourseComplete`. (proven registration; the `bool Evaluate(ctx)` body is behind
the prototype vtable — not in the dump.)

## How it works (from the decomp)

### The chaos economy — two point buckets, proven from the telemetry path

The clearest reachable window into the loop is the **stats/telemetry emitter** `FUN_140a926a0` (size 5892)
and its paired **schema table** `FUN_140a20c40` (size 7377). `FUN_140a926a0` reacts to game events; when the
"chaos milestone" event fires it builds a payload from two `i64` accumulators held on its owner object:

```c
// FUN_140a926a0  (decomp lines ~1233998–1234022)
uVar13 = *(undefined8 *)(param_1 + 800);    // 0x320  -> chaos points, OCCUPIED territory
uVar14 = *(undefined8 *)(param_1 + 0x318);  //        -> chaos points, FRONTLINES
local_698 = 0x3f679bf0;                      // event type id  ("chaos")
FUN_140a819f0(&local_698,"s_chaos_milestone_id",       puVar26);
FUN_140a81c20(&local_698,"i64_chaos_points_frontlines", uVar14);
FUN_140a81c20(&local_698,"i64_chaos_points_occupied",   uVar13);
FUN_140aab200(param_1 + -0x60, local_6e0, "chaos", &local_698);  // emit event "chaos"
if (*(char *)(param_1 + 0x518) != '\0') {    // if flagged, reset the accumulators
  *(undefined8 *)(param_1 + 0x318) = 0;
  *(undefined8 *)(param_1 + 800)  = 0;
}
```

The matching schema in `FUN_140a20c40` keys the same event by id **`0x3f679bf0`** and declares the field
set `{s_chaos_milestone_id, i64_chaos_points_occupied, i64_chaos_points_frontlines}` (decomp lines
1169663–1169681). So it is **proven** that JC4 accounts chaos in **two independent buckets — territory you
*occupy* and territory along the *frontline*** — as running 64-bit counters, emitted (and reset) at each
milestone. (proven — `FUN_140a926a0`, `FUN_140a20c40`.)

The float-looking constant `0x3f679bf0` is used here as a 32-bit **event-type id**, not a float; other
events in the same schema table use ids like `0x64bdce69` (operation-mission), `0x4618e3ea` (player-death),
`0x59701920` (achievement). (proven.)

### Chaos milestones → challenges → achievement/backer progression

A reachable string-builder (`FUN_1409…` region, decomp ~906549) constructs a dynamic milestone id
`"chaos_challenge_" + <N>` (N from an object field), i.e. chaos milestones are enumerated as
`chaos_challenge_0,1,2…`. (proven string construction.)

`FUN_1408e9290` (size 1333) pushes the **`ach_max_chaos_milestone`** achievement/stat: it computes a
progress ratio `fVar18 = ((float)current / (float)total) * DAT_141ca9c7c` and reports it to the stats
backend `DAT_142cbb0d0` via `thunk_FUN_149762700(backend, hash("ach_max_chaos_milestone"), fVar18)`.
(proven — decomp lines ~906549–906560.) The chaos meter also drives UI/audio:
`sfx_gui_general_chaos_meter_pre_increase` → `…_meter_increase` → `chaos_meter_milestone_hit` /
`…_milestone_reached` (functions `FUN_140e05830`, `FUN_140e0a200`). (proven strings.)

Spend/gate side: **`CConditional_HasEnoughChaos`** (getter `FUN_14063d6e0`) is the predicate that unlocks
content once accrued chaos crosses a data-side threshold, and **`CChaosReward`** (getter `FUN_1409478f0`,
granted through `CRewardSystem`) is the payout. The **`CMediaRevolutionManager`** + `CMediaRevolution*`
classes are the "backer / Media Revolution" progression ladder fed by the same chaos flow. (proven that
these classes exist and are registered; the threshold *values* and the earn→spend wiring are data-side.)

### The frontline network — spatial components, walled advance logic

The frontline is a **graph of RTPC entity components**: `CFrontlinePoint` (nodes) joined by
`CFrontlineConnection` (edges), with a `CFrontlineEffectsAreaObject` band of VFX and
`CFrontlineSpawnBlocker` volumes that suppress enemy spawns on the secured side, all coordinated by a
`CFrontlineEventRouter`. Access to the point/component set is via reflection lookups such as
`FUN_14096a340` (size 566), which fetches the `CFrontlinePoint` type (`FUN_140845e10`) from a manager
registry `DAT_142cb0a48` and runs a spatial query through the type's vtable — but the **query result
handling and the frontline advance/retreat integrator are behind that vtable call and are not in the
dump**. (proven that the network is component-based and spatially queried; the movement rule is walled.)

Notably, **no reachable function in the dump contains the literal string "frontline" other than these
registration getters and the one conditional** — confirming the entire frontline *mechanism proper* lives
in `CFrontlineManager`'s data-section vtable (`PTR_LAB_141d907e8`) and in RTPC/ADF data. (proven by
exhaustion — 11 total `Frontline` string sites, all registration/getter/conditional.)

### Territory / node state model (from the conditional vocabulary)

The `CConditional_*` node family names out the **node lifecycle** even though evaluate bodies are walled:
a tactical node is `Discovered` → `Tracked` → `Cleared` → `Secured` (with `Unsafe` as the hostile state),
and these states propagate to **neighbours** (`IsNodeNeighborSecured/Cleared/Unsafe`) — i.e. securing a node
changes its neighbours' predicates, which is the mechanism by which the frontline "spreads" node-to-node.
`IsInSecuredTerritory` / `IsLocationInSecuredTerritory` test whether a world position sits behind the
secured line; `TacticalConnectionStatus` tests node-graph connectivity; `LocationTypeProgression` gates by
location completion; `FactionStatus` reads the owning faction. `IsNearActiveFrontline` tests proximity to a
live frontline edge (used to gate combat spawns / effects). (inferred from the registered name set +
`FUN_148f99a90`; the transition thresholds are data-side.)

### Heat / wanted escalation (proven partial)

`FUN_140e2f540` (size 510) is the **heat-state UI/audio state machine**. It reads an underlying heat level
`*DAT_142cb1d38` (an int) and folds it into a 4-phase UI state, playing the matching SFX on transition:

| UI state | condition on heat level | SFX |
|---|---|---|
| **spotted** (1) | a targeting/detection flag set | `sfx_gui_general_heat_spotted` |
| **hot** (2) | heat level `== 8` | `sfx_gui_general_heat_hot` |
| **evading** (4) | heat level `== 2 or 3`, transitioning down from hot | `sfx_gui_general_heat_evading` |
| **escaped** (0) | heat level `== 0` | `sfx_gui_general_heat_escaped` |

(proven — `FUN_140e2f540`, decomp lines 1646260–1646294.) So the wanted feedback loop is
**spotted → hot → evading → escaped**, driven by a discrete heat-level integer owned by the heat singleton.

Heat also drives **AI reinforcement spawning**: `AiHeatManager` (singleton at `DAT_142cb1d20`) is exercised
by the reachable `FUN_140437…` (decomp lines 381120–381270) which walks spawned entities and registers them
into three heat pools — **`AiHeatManager/"Vehicles"`, `/"Character"`, `/"Props"`** — each drawing from a
budgeted resource allocator `FUN_140b80530(DAT_142cb1d20, entityId, budget, …)` with budget ids `0x10940`
(vehicles/characters) and `0x109c4` (props). (proven — the three `thunk_FUN_1499676d0(…, "AiHeatManager",
"Vehicles"/"Character"/"Props", 0)` sites.) The heat-density/spawn data classes
(`CAiHeatData`, `CAiHeatDensityVolume`, `CAiHeatSpawner`, `CAiHeatAOOSpawnPoint`) are registered as
components (decomp lines 366408–366477 / 860473–860569). The conditionals `HeatUnitCount` and
`IsPlayerInHeatAOO` read this system for behaviour gating. (proven reg.)

### Agency rings — content-course gating

`CAgencyRing` + `CAgencyRingGroup` are RTPC components describing ring-course challenges (wingsuit / vehicle
rings), with `CAgencyRingEffectData` for VFX and `CObjectiveParam_AgencyRingCourse` binding a course to an
objective. The behaviour gates `CConditional_IsAgencyRingCourseActive` / `…Complete` let mission/UI logic
branch on course state; `CAgencyBaseManager` (`PTR_LAB_141d90f48`) owns them. (proven registration; course
progression rules data-side.)

## Data & config integration

- **Identity is the cracked lookup3 name hash.** Every class/conditional/component above is reduced to a
  32-bit key by `FUN_140f27f60` → `FUN_14aadeee0` (the same `hashlittle` used for TAB/ADF names,
  `[[name-hash-cracked]]`). So each `CFrontline*` / `CAgencyRing*` / `CConditional_*` string here hashes to
  the exact value an RTPC `.epe` component blob or ADF config uses to name it — the bridge to the data side.
  (proven hash identity; specific RTPC round-trip not yet done.)
- **Frontline/agency = RTPC entity components.** Registered through the component registrar `FUN_14085fd00`
  as 0x10-byte prototypes → they are instantiated per-entity from `.epe` RTPC property tables
  (`[[rtpc-entity-assembly]]`, `[[composite-assets]]`). The per-component tunables (connection strength,
  spawn-blocker radius, ring trigger radius) live in those property tables. (inferred — same component
  mechanism as vehicles/weapons.)
- **Managers = singletons** from `FUN_148f960c0`; their config (chaos thresholds, secure-node counts, heat
  decay) is loaded into the manager instance from ADF at boot. (inferred.)
- **Conditionals = objective/RTPC operands.** Co-registered with the `CObjectiveGoal_*` / `CObjectiveParam_*`
  family in `FUN_148f99a90`, so a mission goal embeds e.g. `CConditional_IsNodeSecured` or
  `CConditional_HasEnoughChaos` as a predicate (see `missions_progression.md`, `behavior_system.md`).
  (proven co-registration; embed inferred.)
- **Chaos → telemetry/stats.** The chaos economy is mirrored to the analytics backend as the `"chaos"`
  event (`i64_chaos_points_occupied` + `i64_chaos_points_frontlines`) and to the achievements backend as
  `ach_max_chaos_milestone`. (proven.)

## Notable constants / tunables (present in the decomp)

- **Chaos accumulator offsets** on the stats-owner object: `+0x318` = frontlines points, `+0x320` (=800)
  = occupied points; reset gate flag at `+0x518`. (proven — `FUN_140a926a0`.)
- **Event-type ids** (32-bit): `"chaos"` milestone = `0x3f679bf0`; operation-mission = `0x64bdce69`;
  player-death = `0x4618e3ea`; achievement = `0x59701920`. (proven — `FUN_140a20c40`.)
- **AI-heat spawn budget ids**: `0x10940` (vehicles/characters), `0x109c4` (props); pool labels
  `"Vehicles"/"Character"/"Props"` under `"AiHeatManager"`. (proven — `FUN_140437…`.)
- **Heat UI phase values**: level `8` → hot; `2/3` → evading; `0` → escaped; detection flag → spotted.
  (proven — `FUN_140e2f540`.)
- **Achievement progress scale** `DAT_141ca9c7c` in the `ach_max_chaos_milestone` ratio. (proven the
  formula `current/total * DAT_141ca9c7c`; the scalar value is a data-section float, not in the text.)
- **Name-hash cache DATs** (see tables above) — e.g. `CFrontlineConnection`→`DAT_142cb97ac`,
  `CChaosReward`→`DAT_142cba720`, `ach_max_chaos_milestone`→`DAT_142cba278`. (proven.)
- **NOT present** (walled): chaos-per-kill / chaos-per-activity values, node-secure thresholds, frontline
  advance speed, heat gain/decay rates, agency-ring counts. These are in the manager vtables / RTPC / ADF.

## Call-graph highlights

- Manager registrar `FUN_148f960c0` → `thunk_FUN_1496a12b0(8)` (alloc) + `thunk_FUN_148f6e780` (insert)
  ×roster, incl. `CFrontlineManager` (`PTR_LAB_141d907e8`), `CHeatManager` (`…808`),
  `CTacticalNodeManager` (`…748`), `CAgencyBaseManager` (`…f48`), `CDiscoveryManager` (`…d88`). (proven.)
- Component registrar `FUN_14085fd00` → per-component reg thunks `FUN_14080c670/730/7f0/8b0` (frontline)
  → `thunk_FUN_14cfef490(table, hash, proto)`. (proven.)
- Conditional master registry `FUN_148f99a90` → 8-byte prototypes for the whole node/territory/faction/
  chaos/heat/agency conditional set. (proven.)
- Telemetry: game event → `FUN_140a926a0` (build "chaos" payload) → `FUN_140a54c20` (schema helper) and
  schema table `FUN_140a20c40`. Achievement push: `FUN_1408e9290` → `thunk_FUN_149762700(statsBackend,
  hash, value)`. (proven.)
- Heat: `FUN_140e2f540` reads `*DAT_142cb1d38`, plays heat SFX; `FUN_140437…` registers entities into
  `AiHeatManager` (`DAT_142cb1d20`) pools via `FUN_140b80530`. (proven.)
- Frontline spatial query: `FUN_14096a340` (caller `FUN_14096e1f0`) fetches `CFrontlinePoint` type via
  `FUN_140845e10`, queries registry `DAT_142cb0a48` through its vtable. (proven.)

## Open questions / lower-confidence

- **Frontline advance/retreat integrator** — the actual rule that moves `CFrontlinePoint`/`Connection`
  positions and flips node ownership is inside `CFrontlineManager`'s vtable (`PTR_LAB_141d907e8`), not in
  the dump. Resolve via live x64dbg vtable read at that PTR, or by decoding a frontline `.epe`/ADF config.
  (walled.)
- **Node state-transition thresholds** — how many activities/how much chaos secures a node, and how
  "secured" propagates to neighbours, are behind the `CConditional_IsNode*` evaluate vtables + ADF. Resolve
  via `jc4_adf` on the tactical-node config type + RTPC property tables. (walled.)
- **Chaos earn rates** — the chaos-per-destruction/activity magnitudes that increment the `+0x318`/`+0x320`
  accumulators are set elsewhere (likely per-object "chaos value" in RTPC/ADF). Which system writes those
  two counters (the owner object of `FUN_140a926a0`) is unconfirmed — likely `CFrontlineManager` or a
  dedicated chaos manager. (inferred owner.)
- **Heat gain/decay & AOO shape** — `CHeatManager` decay timers and the heat-AOO geometry are data-side;
  only the 4-phase UI mapping and the AI-spawn pool wiring are proven. Confirm the heat-level range (the
  `== 8` hot / `== 2/3` evading suggests a small enum) via `CHeatManager` vtable or ADF. (walled.)
- **`CChaosReward` / `CMediaRevolution*` thresholds** — the milestone→reward table is ADF; not decoded.
  Resolve via `jc4_adf` on the reward/media-revolution config. (walled.)
- **Agency-ring course completion rule** — `IsAgencyRingCourseActive/Complete` evaluate bodies + the ring
  count/timer are data-side. (walled.)

## Appendix — decomp anchors (re-verifiable)

| Anchor | FUN_ / DAT_ / line |
|---|---|
| Manager registrar (roster incl. CFrontlineManager) | `FUN_148f960c0` @ line 4138520; dup near line 3209030 |
| — CFrontlineManager / CHeatManager vtables | `PTR_LAB_141d907e8` / `PTR_LAB_141d90808` (lines 4138937/4138942) |
| — CTacticalNode/AgencyBase/Discovery/DemonArea | `…90748` / `…90f48` / `…90d88` / `…90f28` |
| Component registrar | `FUN_14085fd00` |
| — CFrontlineConnection reg | `FUN_14080c670`, vtable `PTR_LAB_141d9bed0`, hash `DAT_142cb97ac` (line 864057) |
| — CFrontlineEffectsAreaObject / Point / SpawnBlocker | `FUN_14080c730`/`14080c7f0`/`14080c8b0` (lines 864089/864121/864153) |
| — CFrontlineEventRouter getter | `FUN_140845d80`, hash `DAT_142cb9634` (line 900862) |
| — CFrontlinePoint getter (consumer path) | `FUN_140845e10`; consumer `FUN_14096a340` (line 900885 / 1077622) |
| CAgencyRing / RingGroup / RingEffectData getters | `FUN_140844af0`/`140844b80`/`140843ac0` (lines 900037/900062/899510) |
| CConditional_IsNearActiveFrontline getter | `FUN_140843c90`, hash `DAT_142cb94dc` (line 899535) |
| CConditional_HasEnoughChaos getter | `FUN_14063d6e0`, line 605589 |
| CConditional master registry (node/territory/chaos/heat/agency set) | `FUN_148f99a90` (registry body lines 4139962–4140782) |
| CChaosReward getter | `FUN_1409478f0`, hash `DAT_142cba720` (line 1055691) |
| Chaos telemetry emitter (2-bucket accumulators) | `FUN_140a926a0` (lines 1233998–1234022) |
| Chaos event schema (`0x3f679bf0`) | `FUN_140a20c40` (lines 1169663–1169681); helper `FUN_140a54c20` |
| `chaos_challenge_<N>` builder | ~line 906549 |
| `ach_max_chaos_milestone` push | `FUN_1408e9290`, hash `DAT_142cba278` (lines 906549–906560) |
| Chaos meter SFX | `FUN_140e05830` / `FUN_140e0a200` (lines 1621080 / 1623565) |
| Heat UI state machine (spotted/hot/evading/escaped) | `FUN_140e2f540`, level `*DAT_142cb1d38` (lines 1646260–1646294) |
| AiHeatManager reinforcement pools | `FUN_140437…` (lines 381120–381270), singleton `DAT_142cb1d20`, alloc `FUN_140b80530` |
| Name hash (identity) | `FUN_140f27f60` → `FUN_14aadeee0` (lookup3 `hashlittle`) |

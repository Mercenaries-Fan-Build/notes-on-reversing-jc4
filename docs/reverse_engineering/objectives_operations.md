# Objectives, Missions & Operations — the mission-runtime layer

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

All FUN_/DAT_ addresses are from `output/_ghidra_jc4/jc4_all_functions_decomp.txt` (Ghidra decomp of
`JustCause4.exe`, 162,115 functions). "proven" = read directly from that decomp text. This doc is the
**runtime** companion to `missions_progression.md` (which covers the high-level quest/progression layer and
the global type-registry factory `FUN_140f27f60`). Read that first; this one focuses on *how one objective is
instantiated, tracked, completed/failed, and how scripted missions drive actors and the HUD.*

## Overview

A JC4 mission is a **data graph of reflected gameplay types**, not compiled code. The runtime pieces are:

* **Named singleton managers** — `CMissionManager`, `CObjectiveManager`, `CObjectiveContentManager`,
  `COperationManager`, `CObjectiveDebugTextManager` — all registered into the service container by the
  manager registrar `FUN_148f960c0` (and its sibling at ~line 3209415 / ~4138900). (proven)
* **An objective type schema** — `CObjectiveGoal_*` (completion rules), `CObjectiveParam_*` (HUD widgets),
  `CObjectiveTime*`, `CObjectiveContent*`, plus checkpoint/daredevil scoring types — all registered as
  **reflected data types** in one giant schema registrar `FUN_148f99a90` (size 7930), each with a lazy
  type-descriptor getter that calls `FUN_140f27f60("<ClassName>", len)`. (proven)
* **Objective-controlled entity components** — `CObjectiveVehicleController` / `CObjectiveCharacterController`
  (+ `…ControllerSeat`, `…VehicleOverride`) are reflected **entity components** registered via the *component*
  registrar `FUN_14085fd00`. These are how a scripted mission drives a specific vehicle/character actor. (proven)
* **Completion/fail predicates** — objective/mission state is surfaced to the behavior/condition VM as a
  `CConditional_*` family (`IsObjectiveCompleted`, `IsObjectiveFailed`, `MissionStatus`,
  `ObjectiveGoalStatus`, …). Their Evaluate reads a manager's state by **type-id hash binary-search**. (proven)
* **A lifecycle/telemetry spine** — objective start/complete/fail/retry/abandon/quit transitions flow through a
  shared content-event reporter (`FUN_140aa3dd0` / emitter `FUN_140aa5c10`) that maps enum → string
  (`"objective_failed"`, `"objective_abandoned"`, …) and into the stats blob (`s_operation_mission_checkpoint_*`).
  (proven)

The mechanism (state machines, event names, enums, the component/condition skeleton) is fully recoverable.
The per-goal **completion math** (how `CObjectiveGoal_Counter` counts, how `_Area` tests containment) lives in
each goal's `Update` method behind a **data-section vtable** and is *walled* in this functions-only export
(see methodology caveat). Where a body is unreachable it is called out explicitly.

## Key classes & functions

| Class / string | FUN_ (registration / getter / impl) | Role |
|---|---|---|
| `CMissionManager` | reg `FUN_148f960c0` (`thunk_FUN_148f6e780`, line 3209465/4138947); type-id use `FUN_1408d25e0`, `FUN_140900c40` | Mission instances |
| `CObjectiveManager` | reg `FUN_148f960c0` (line 3209420/4138902); ctor `FUN_1409027a0`; type-id use `FUN_14090b610`, `FUN_1408ff4f0` | Live objective tracking + save data |
| `CObjectiveContentManager` | reg `FUN_148f960c0` (3209425/4138907); typename getter returns string at `FUN_149247310` / line 4195939 | Objective HUD content payloads |
| `COperationManager` | reg `FUN_148f960c0` (3209525/4139007) | Story "operations" (multi-mission arcs) |
| `CObjectiveDebugTextManager` | reg `FUN_148f960c0` (3209665/4139147) | Debug on-screen objective text |
| `CObjectiveVehicleController` | component `FUN_140811050` (vtable `PTR_LAB_141d9c060`); getter `FUN_140947d30` (`DAT_142cb97dc`) | Scripted control of a vehicle actor |
| `CObjectiveCharacterController` | component `FUN_140810f90` (vtable `PTR_LAB_141d9c038`); getter `FUN_140947a30` (`DAT_142cb97d4`) | Scripted control of a character actor |
| `CObjectiveVehicleControllerSeat` | getter `FUN_140947dc0` (`DAT_142cba6b0`) | Per-seat scripted occupancy |
| `CObjectiveVehicleOverride` | getter `FUN_140947e60` (`DAT_142cba6b8`) | Override vehicle params for an objective |
| `CObjectiveGoal[_*]` | getters `FUN_14090ead0…ee90`, `FUN_140844010` (see §2) | Objective completion rules |
| `CObjectiveParam_*` | getters `FUN_1408440b0…`, `FUN_1408445b0` (TornadoWidget) | Objective HUD widgets |
| `CObjectiveContentImage[_Vehicle]` | getters `FUN_?` (line 899585/899610, `DAT_142cb953c/…544`) | HUD content thumbnails |
| Schema registrar (all objective data types) | `FUN_148f99a90` (size 7930, via `thunk_FUN_147cafcd0`) | Registers the whole objective taxonomy |
| Content-event / telemetry reporter | `FUN_140aa3dd0` (size 2781), emitter `FUN_140aa5c10` (size 1301) | Objective/mission lifecycle → stats |
| HUD content preview updater | `FUN_140e59840` → fires `hud_objective_content_changed` | Objective HUD text hook |
| Global type-registry factory | `FUN_140f27f60(name, len)` (5-byte thunk, 6138 callers) | Name → cached type descriptor |

## How it works (from the decomp)

### 1. Managers are service singletons; `CObjectiveManager` carries the save blob  (proven)

The mission-runtime managers are registered alongside the rest of the progression managers in
`FUN_148f960c0` via `thunk_FUN_148f6e780(container, "<ClassName>", holder)` — `CObjectiveManager`,
`CObjectiveContentManager`, `CMissionManager`, `COperationManager`, `CObjectiveDebugTextManager`
(lines 3209420–3209665, mirrored at 4138902–4139147). (proven)

The objective-manager **constructor `FUN_1409027a0`** (caller `FUN_148e7fba0`) builds a large object:
installs vtables `PTR_LAB_141db7160` / `…7198` / `…71b0` / `PTR_LAB_141ca9c98`, allocates two hash-set
substructures (`FUN_140876c80` ×2 at `+0x11`/`+0x13`), lays out three intrusive linked lists
(`+0x15..0x17`, `+0x36..0x38`, `+0x73..0x75` — the tracked/active/pending objective lists, *inferred* from the
triple head/tail/sentinel pattern), and — crucially — creates a named save section:

```c
FUN_140b6cef0(param_1 + 4, "objectives_save_data", 2, ...);   // save-data blob, version 2
*(u64*)(param_1+0x42c) = 3;  *(u32*)(param_1+0x43c) = 5;       // capacity/limit constants
```

So live objective state is **persisted** under the key `"objectives_save_data"` (version `2`). (proven —
strings/layout read directly; the list semantics are inferred from the head/tail/sentinel triples.)

### 2. The objective type schema — one registrar, one getter per type  (proven)

Every objective data type is registered into a reflection registry inside `FUN_148f99a90` (the big data-type
registrar, size 7930) with `thunk_FUN_147cafcd0(registry, "<ClassName>", desc)`. The complete objective
taxonomy registered there (proven strings):

* **Goals (completion rules):** `CObjectiveGoal`, `CObjectiveGoal_Area`, `CObjectiveGoal_Counter`,
  `CObjectiveGoal_CategoryCounter`, `CObjectiveGoal_Distance`, `CObjectiveGoal_TimeLimit`,
  `CObjectiveGoal_StayNearObject`, `CObjectiveGoal_Conditional`.
* **Params (HUD widgets):** `CObjectiveParam`, `_ProgressBar`, `_HealthBar`, `_TimerPopup`, `_TornadoWidget`,
  `_DaredevilScoreAndTimer`, `_BundleBlips`, `_AgencyRingCourse`, `_StayInAreaHack`.
* **Time modifiers:** `CObjectiveTime`, `CObjectiveTimeCap`, `CObjectiveTimeSet`.
* **Content:** `CObjectiveContentImage`, `CObjectiveContentImage_Vehicle`, `CObjectiveContentObject`,
  `CObjectivesInfoDescription`, `CObjectiveDebugText`.
* **Checkpoints / scoring (co-registered in the same block):** `CTimeLimitCheckpoint`, `CTimeLimitBeat`,
  `CGroupHealthCheckpoint`, `CObjectHealthCheckpoint`, `CDaredevilCheckpoint`, `CDaredevilPointEvent`,
  `CDaredevilPointId`, `CDaredevilPointVehicleDestroyed`, `CDaredevilPointCharacterKilled`,
  `CDaredevilPointModelDestroyed`, `CDaredevilComboLevel`, `CDaredevilAnnouncement`.
* **Spawn criticality / tactical:** `CSpawnParam_ObjectiveCritical`, `CSpawnParam_ObjectCritical`,
  `CTacticalNodeResource`, `CTacticalNodeDiscover`.

Each type has a **lazy per-type getter** (thread-safe singleton) that resolves the descriptor once via
`FUN_140f27f60("<ClassName>", strlen)` and caches it in a `DAT_` slot — the standard bridge that lets
serialized RTPC/ADF asset data name a class and get a constructible descriptor back. The goal/param getter
cluster (proven addresses):

| Type | Getter | Cached DAT_ | len |
|---|---|---|---|
| `CObjectiveDebugText` | `FUN_14090ea30` | `DAT_142cba468` | 0x13 |
| `CObjectiveGoal` | `FUN_14090ead0` | `DAT_142cba460` | 0x0e |
| `CObjectiveGoal_Area` | `FUN_14090eb70` | `DAT_142cba498` | 0x13 |
| `CObjectiveGoal_CategoryCounter` | `FUN_14090ec10` | `DAT_142cba480` | 0x1e |
| `CObjectiveGoal_Conditional` | `FUN_14090ecb0` | `DAT_142cba488` | 0x1a |
| `CObjectiveGoal_Counter` | `FUN_14090ed50` | `DAT_142cba470` | 0x16 |
| `CObjectiveGoal_Distance` | `FUN_14090edf0` | `DAT_142cba4a0` | 0x17 |
| `CObjectiveGoal_TimeLimit` | `FUN_14090ee90` | `DAT_142cba478` | 0x18 |
| `CObjectiveGoal_StayNearObject` | `FUN_140844010` | `DAT_142cb94f4` | 0x1d |
| `CObjectiveParam` | `FUN_1408440b0` | `DAT_142cb94fc` | 0x0f |
| `CObjectiveParam_TornadoWidget` | `FUN_1408445b0` | `DAT_142cb9514` | 0x1d |
| `CObjectiveTime` | `FUN_140947b50` | `DAT_142cba4f0` | 0x0e |
| `CObjectiveTimeCap` | `FUN_140947bf0` | `DAT_142cba698` | 0x11 |
| `CObjectiveTimeSet` | `FUN_140947c90` | `DAT_142cba6a0` | 0x11 |

**Wall:** the getter only yields the *descriptor*. The goal's `Update`/`IsComplete` body (the actual counting,
area test, distance math) is dispatched through the descriptor's vtable in the data section and is **not
present** in this functions-only export. What the *taxonomy* proves is the set of completion primitives the
mission designer can select: count N things (`_Counter`/`_CategoryCounter`), be inside a volume (`_Area`), be
within a distance of / stay near an object (`_Distance`/`_StayNearObject`), beat a clock (`_TimeLimit`), or
satisfy an arbitrary condition tree (`_Conditional`). (proven that these are the primitives; math walled.)

### 3. Objective-controlled entities — the scripted actor components  (proven)

Scripted missions drive specific actors by attaching two **reflected entity components** — these register
through the *component* registrar `FUN_14085fd00`, not the manager registrar, which is what makes them
attachable to entities like any other component:

```c
// FUN_140810f90  (caller FUN_14085fd00 @0x14086372d) — CObjectiveCharacterController
puVar2 = thunk_FUN_1496a12b0(0x10);          // 0x10-byte component
*puVar2 = &PTR_LAB_141d9c038;                 // component vtable
DAT_142cb97d4 = FUN_140f27f60("CObjectiveCharacterController",0x1d,...);  // type desc
thunk_FUN_14cfef490(entitySystem, DAT_142cb97d4, puVar2);                 // register component

// FUN_140811050 (caller FUN_14085fd00 @0x140863735) — CObjectiveVehicleController
*puVar2 = &PTR_LAB_141d9c060;
DAT_142cb97dc = FUN_140f27f60("CObjectiveVehicleController",0x1b,...);
```

Supporting reflected types `CObjectiveVehicleControllerSeat` (getter `FUN_140947dc0`) and
`CObjectiveVehicleOverride` (getter `FUN_140947e60`) let an objective pin an occupant into a specific seat and
override a vehicle's parameters for the duration. Together these are the mechanism a mission uses to say "this
helicopter is *the* mission helicopter; put NPC X in seat 1; make it invulnerable." (proven the components
exist and are entity-registered; their per-frame drive logic is behind the component vtable → walled.)

### 4. Completion & fail predicates — the `CConditional_*` objective family  (proven)

Objective/mission progress is exposed to the behavior/condition VM (see `behavior_system.md`) as conditionals.
The mission-runtime-facing set, all registered in `FUN_148f99a90` and each with a lazy getter:

* **Boolean state:** `CConditional_IsMissionActive`, `_IsMissionCompleted`, `_IsObjectiveActive`,
  `_IsObjectiveCompleted`, `_IsObjectiveFailed`, `_IsObjectiveHidden`, `_IsObjectiveTracked`,
  `_IsObjectiveCurrentInSequence`, `_IsObjectiveContentAvailable`, `_IsAnyHardFailObjectiveActive`.
* **Enum/status:** `CConditional_MissionStatus` (getter `FUN_14065ffe0`, `DAT_142cb5f78`),
  `CConditional_ObjectiveGoalStatus` (getter `FUN_140660260`, `DAT_142cb5f88`),
  `CConditional_ObjectTrackerStatus` (getter `FUN_1406601c0`, `DAT_142cb5f80`).

The **Evaluate mechanism** is readable. Example `FUN_14090b610` (a conditional-evaluate) resolves the
`CObjectiveManager` type-id hash once —
`FUN_140f27f60("class CHashString __cdecl ArGetTypeId<class CObjectiveManager>(void)", 0x44)` cached in
`DAT_142cba4b8` — then calls `FUN_1408ff4f0`, which **binary-searches** the entity's sorted
`(typeId → value)` map for that hash and returns the stored byte:

```c
// FUN_1408ff4f0 — resolve objective-manager state by type-id hash (binary search)
while (n != 0) { n >>= 1; if (key[mid*2] < DAT_142cba4b8) { key += mid*2+2; n = ...; } }
if (found && key[0]==DAT_142cba4b8) return (char)key[1];
```

`FUN_14090b610` then sets a "value changed" bit into the conditional node (`param_2+0xc |= (old != new)`),
which is how the VM knows to re-fire dependent logic. Sibling `CMissionManager` consumers
(`FUN_1408d25e0`, `FUN_140900c40`) use the same idiom with the `ArGetTypeId<CMissionManager>` hash (len 0x42).
(proven — the type-id-keyed binary-search read is read directly; the *meaning* of the returned byte per
conditional, e.g. which enum value = "completed", is behind the descriptor and *inferred*.)

### 5. Objective lifecycle & telemetry — the shared content-event reporter  (proven)

Objective/mission transitions are reported through a shared content-lifecycle reporter,
**`FUN_140aa3dd0`** (size 2781) and its emitter **`FUN_140aa5c10`** (size 1301, callers pass an
**event-type id 0x10–0x15**). Both map small integer enums to canonical strings by binary-searching a
`(propertyHash → value)` map on the event payload:

* **Fail-reason enum** (property hash `0x47e97f2d`): `0 → "player_death"`, `1 → "objective_failed"`,
  `2 → "objective_abandoned"`, else `"unknown"`.
* **Status enum** (property hash `0x8a0b93c8`): `0 "start"`, `1 "complete"`, `2 "fail"`, `3 "retry"`,
  `4 "abandon"`, `5 "quit"`.
* Numeric checkpoint/id fields are read from hashes `0x8581b032`, `0x4d2d5b42` and rendered as decimal.

The emitter's event-type argument selects a name from a data-section string table
`PTR_s_empty_141df1297_1_142ae23d0[event_type]` (walled contents), and the six call sites form the
checkpoint sequence: `0x10` (`FUN_1493240b0`), `0x11` (`FUN_140945ee0`), `0x12` **+fail-reason**
(`FUN_140946640`), `0x13` (`FUN_149321c30`), `0x14`, `0x15`. `FUN_140946640` fires the `0x12` fail event only
when a guard passes, reading the checkpoint id from the driving object at `+0x28c`/`+0x290`:

```c
// FUN_140946640
if (guard && thunk_FUN_148ebeec0(*(u32*)(param_1+0x290))) {
    uVar5 = FUN_14094ad70(param_1);
    FUN_140aa5c10(DAT_142cbb0b8, *(u32*)(param_1+0x28c), uVar5, 0x12, failReason);
}
```

`FUN_140aa3dd0` writes the resulting strings into the OSDK stats blob under content-typed keys — proven keys in
that function include `s_quest_status`/`s_quest_fail_reason`/`s_quest_objective_id`,
`s_encounter_status`/`_fail_reason`/`_id`, and DLC variants
`s_dlc_demons_and_danger_mission_status`/`_fail_status`/`_objective_id` — the same schema whose
`operation_mission_*` counterparts are documented in `missions_progression.md` §5. (proven — enum→string maps,
property hashes, event-type ids and stat keys read directly; the string-table contents are walled.)

Additional proven runtime event/UI strings for the objective lifecycle: `objective_content`,
`hud_objective_content_changed`, `objective_abandoned`, `objective_failed`, `node_objective_return`
(`FUN_140f27f60(...,0x15)`), `node_objective_hardfail_default` (len 0x1f, `FUN_140f27f60` at ~line 253536),
plus UI intents `pause_objective_abandon` / `pause_objective_restart` and SFX
`sfx_gui_general_objective_complete` / `_objective_fail`.

### 6. Objective HUD content hook — `FUN_140e59840`  (proven)

`CObjectiveContentManager` drives the objective HUD panel. `FUN_140e59840` sets the panel's preview and
broadcasts a change event:

```c
(**(code**)(*param_1 + 0x20))(param_1,"SetPreviewText",local_130,1);      // push preview text into HUD
thunk_FUN_14a8a3850(param_1[2], "hud_objective_content_changed");         // notify listeners
```

It selects an icon/label token from `FUN_140088850(DAT_142c73ae0, hash)` with `hash = 0x753581b2` vs
`0x5f7b9ca4` depending on whether the current content matches the tracked content (`param_1[4]==param_1[7]`) —
i.e. "is this the actively tracked objective." A second HUD site at line 4764542 fires the same event.
(proven.) The content payload types are `CObjectiveContentImage` / `_Vehicle` /
`CObjectiveContentObject` / `CObjectivesInfoDescription` from §2.

### 7. Operations — story arcs  (proven registration; runtime inferred)

`COperationManager` is registered next to `CMissionManager` (line 3209525/4139007). "Operations" are the
game's replayable story missions; their telemetry footprint is the `operation_mission_*` family in the stats
serializer (`missions_progression.md` §5): counters `i32_operation_mission_completes_count`,
`i32_operation_mission_attempts`, durations `i64_operation_mission_duration_ms` /
`i64_operation_mission_checkpoint_duration_ms`, and per-run keys `s_last_operation_mission_completed`,
`s_operation_mission_checkpoint_id` / `_status` / `_fail_reason`. The UI surfaces them via the map panel
strings `map_panel_operations_header_active_title` / `map_panel_operations_header_completed_title`. The
checkpoint status/fail-reason enums an operation run reports are exactly the §5 enums. (proven — manager
registration, stat keys, UI strings; the manager's own per-frame arc-advancement tick is behind its vtable →
open.)

### 8. Debug text  (proven registration; body walled)

`CObjectiveDebugTextManager` (manager, line 3209665) and the reflected `CObjectiveDebugText` type (getter
`FUN_14090ea30`) are the developer on-screen objective readout. Registration is proven; the draw/format body
is behind the manager vtable and not present in this export. (open.)

## Data & config integration

* **A mission asset is a data graph naming these classes.** Objectives, goals, params, time modifiers,
  content images, checkpoints and the objective-controller components are all instantiated *by name* through
  `FUN_140f27f60`. In the asset pipeline those names are the class strings that RTPC/ADF entity components
  reference (cross-ref `[[rtpc-entity-assembly]]`, `[[composite-assets]]`), so a mission/objective asset is an
  RTPC/ADF graph whose nodes are `CObjectiveGoal_*` / `CObjectiveParam_*` / `CObjective*Controller*` /
  `CDaredevil*` types. (inferred — the naming/hash convention matches the entity-component scheme; not
  byte-verified here.)
* **Objective state ↔ condition VM.** Quest/objective availability, visibility, tracking and hard-fail are
  expressed as `CConditional_*` predicates whose Evaluate reads the relevant manager by
  `ArGetTypeId<manager>` hash (§4). Mission designers gate world content on these; the reverse binding
  (manager → conditional invalidation) is the `param_2+0xc` "changed" bit. (proven the read path; the exact
  enum semantics inferred.)
* **Persistence.** Live objective state serializes under `"objectives_save_data"` (version 2), built in the
  objective-manager ctor `FUN_1409027a0`. Lifecycle transitions additionally emit into the OSDK stats/telemetry
  JSON (`FUN_1409ff030`, see `missions_progression.md` §5). (proven.)

## Notable constants / tunables

| Constant | Where | Meaning |
|---|---|---|
| `"objectives_save_data"`, ver `2` | `FUN_1409027a0` | Objective-manager persistent save section |
| `*(u64*)(obj+0x42c)=3`, `*(u32*)(obj+0x43c)=5` | `FUN_1409027a0` | Objective-manager capacity/limit constants (meaning inferred) |
| `ArGetTypeId<CObjectiveManager>` len `0x44` → `DAT_142cba4b8` | `FUN_14090b610`, `FUN_1408ff4f0` | Objective-manager service key (hashed) |
| `ArGetTypeId<CMissionManager>` len `0x42` | `FUN_1408d25e0`, `FUN_140900c40` | Mission-manager service key |
| prop hash `0x47e97f2d` | `FUN_140aa3dd0` | fail-reason field (0 player_death,1 objective_failed,2 objective_abandoned) |
| prop hash `0x8a0b93c8` | `FUN_140aa3dd0` | status field (0 start,1 complete,2 fail,3 retry,4 abandon,5 quit) |
| prop hashes `0x8581b032`, `0x4d2d5b42` | `FUN_140aa3dd0` | numeric id fields (rendered decimal) |
| event-type ids `0x10`–`0x15` | callers of `FUN_140aa5c10` | Checkpoint event sequence (start…quit) |
| `param_4 == 0x12` | `FUN_140aa5c10` | Fail event carries a fail-reason `param_5` |
| HUD tokens `0x753581b2` / `0x5f7b9ca4` | `FUN_140e59840` | tracked vs. untracked objective content icon |
| component vtables `PTR_LAB_141d9c038` / `…c060` | `FUN_140810f90` / `FUN_140811050` | Char / Vehicle objective-controller components |
| `node_objective_hardfail_default` len `0x1f`; `node_objective_return` len `0x15` | ~line 253536 / 1052588 | Frontline-node objective fail/return keys |

## Call-graph highlights

* **Bootstrap:** engine init → `FUN_148f960c0` registers `CObjective/Mission/Operation/…Manager`; `FUN_14085fd00`
  registers the `CObjective*Controller` components; `FUN_148f99a90` registers the objective *data-type* schema
  (goals/params/checkpoints/daredevil). (proven)
* **Objective-manager construction:** `FUN_148e7fba0` → `FUN_1409027a0` (builds lists + `objectives_save_data`).
* **Type creation (any objective node):** per-type getter (e.g. `FUN_14090ed50` for `_Counter`) →
  `FUN_140f27f60(name,len)` → cached `DAT_` descriptor → data-side vtable (walled Update). (proven to the wall)
* **Condition evaluate:** VM → conditional-evaluate `FUN_14090b610` → resolve `ArGetTypeId<CObjectiveManager>`
  (`DAT_142cba4b8`) → `FUN_1408ff4f0` binary-search entity type-map → set changed-bit. (proven)
* **Lifecycle/telemetry:** checkpoint object → `FUN_140945ee0`/`FUN_140946640`/`FUN_1493240b0`/`FUN_149321c30`
  → `FUN_140aa5c10(...,0x10..0x15[,failReason])` → `FUN_140aa3dd0` (enum→string, write stat keys). (proven)
* **HUD:** `FUN_140e59170`/`FUN_14a78d9f0` → `FUN_140e59840` (SetPreviewText + `hud_objective_content_changed`).

## Open questions / lower-confidence

* **Per-goal completion math is walled.** `CObjectiveGoal_Counter/_Area/_Distance/_TimeLimit/_StayNearObject`
  Update/IsComplete bodies dispatch through data-section vtables not present in the functions-only export. We
  have the *taxonomy of primitives* and the descriptor getters, not the thresholds/tests. (open)
* **Enum-value semantics per conditional.** `FUN_1408ff4f0` returns a byte per objective/mission type-id; which
  concrete value each `CConditional_*` treats as "completed/failed/hidden" is behind the descriptor. (inferred)
* **`COperationManager` tick.** Operation arc advancement (how a completed mission unlocks the next in an
  operation) is named and telemetried but its per-frame body was not located. (open)
* **`CObjectiveDebugTextManager` / event-name string table.** Draw body and the
  `PTR_s_empty_141df1297_1_142ae23d0` string table contents are in walled data. (open)
* **Objective-manager list layout.** The three head/tail/sentinel triples in `FUN_1409027a0` are read as the
  tracked/active/pending objective lists by pattern; exact roles unconfirmed. (inferred)

## Appendix — decomp anchors

**Managers (proven):** registrar `FUN_148f960c0` @0x148f960c0 lines 3209420(`CObjectiveManager`),
3209425(`CObjectiveContentManager`), 3209465(`CMissionManager`), 3209525(`COperationManager`),
3209665(`CObjectiveDebugTextManager`); sibling registrar ~4138902–4139147. Objective-manager ctor
`FUN_1409027a0` @0x1409027a0 (caller `FUN_148e7fba0`; string `"objectives_save_data"` line 1009261).
Mission-manager consumers `FUN_1408d25e0` @0x1408d25e0, `FUN_140900c40` @0x140900c40. Objective-manager
consumers `FUN_14090b610` @0x14090b610, `FUN_1408ff4f0` @0x1408ff4f0 (`ArGetTypeId<CObjectiveManager>` lines
1007496/1016124, cached `DAT_142cba4b8`).

**Type schema (proven):** registrar `FUN_148f99a90` @0x148f99a90 (size 7930), objective block via
`thunk_FUN_147cafcd0` ~lines 4140547–4140732. Goal/param getters `FUN_14090ea30`–`FUN_14090ee90`
(cluster @ ~line 1018749–1018924), `FUN_140844010` (`_StayNearObject`), `FUN_1408440b0` (`CObjectiveParam`),
`FUN_1408445b0` (`_TornadoWidget`); time getters `FUN_140947b50/bf0/c90`. Content getters at lines
899585/899610 (`DAT_142cb953c`/`…544`), `FUN_140947ac0` (`CObjectiveContentObject`, `DAT_142cba690`),
line 901033 (`CObjectivesInfoDescription`, `_DAT_142cb9c58`).

**Objective-controller components (proven):** `FUN_140810f90` @0x140810f90 (`CObjectiveCharacterController`,
vtable `PTR_LAB_141d9c038`, `DAT_142cb97d4`), `FUN_140811050` @0x140811050 (`CObjectiveVehicleController`,
vtable `PTR_LAB_141d9c060`, `DAT_142cb97dc`) — both called from component registrar `FUN_14085fd00`. Getters
`FUN_140947a30` / `FUN_140947d30`; `…Seat` `FUN_140947dc0` (`DAT_142cba6b0`); `…VehicleOverride`
`FUN_140947e60` (`DAT_142cba6b8`).

**Conditionals (proven):** `CConditional_MissionStatus` getter `FUN_14065ffe0` (`DAT_142cb5f78`, line 628003),
`CConditional_ObjectiveGoalStatus` `FUN_140660260` (`DAT_142cb5f88`, line 628103),
`CConditional_ObjectTrackerStatus` `FUN_1406601c0` (`DAT_142cb5f80`, line 628078); boolean `IsObjective*` /
`IsMission*` family registered in `FUN_148f99a90` ~lines 4139900–4140290.

**Lifecycle/telemetry (proven):** reporter `FUN_140aa3dd0` @0x140aa3dd0 (size 2781, caller `FUN_1435108bf`);
emitter `FUN_140aa5c10` @0x140aa5c10 (size 1301, callers `FUN_140945ee0`/`FUN_140946640`/`FUN_1493240b0`/
`FUN_149321c30`/`FUN_149321900`/`FUN_1492c7780`); enum→string maps at lines 1245092–1245145 & 1246558–1246574;
event-name table `PTR_s_empty_141df1297_1_142ae23d0` (lines 1234338/1245735/1246584/…).

**HUD (proven):** `FUN_140e59840` @0x140e59840 (`SetPreviewText`, `hud_objective_content_changed`; second site
line 4764542).

**Class-name string inventory (proven):** managers `CMissionManager`, `CObjectiveManager`,
`CObjectiveContentManager`, `COperationManager`, `CObjectiveDebugTextManager`; components
`CObjectiveVehicleController`, `CObjectiveCharacterController`, `CObjectiveVehicleControllerSeat`,
`CObjectiveVehicleOverride`; goals `CObjectiveGoal[_Area/_Counter/_CategoryCounter/_Distance/_TimeLimit/
_StayNearObject/_Conditional]`; params `CObjectiveParam[_ProgressBar/_HealthBar/_TimerPopup/_TornadoWidget/
_DaredevilScoreAndTimer/_BundleBlips/_AgencyRingCourse/_StayInAreaHack]`; time `CObjectiveTime[/Cap/Set]`;
content `CObjectiveContentImage[/_Vehicle]`, `CObjectiveContentObject`, `CObjectivesInfoDescription`,
`CObjectiveDebugText`; checkpoints/scoring `CTimeLimitCheckpoint`, `CTimeLimitBeat`, `CGroupHealthCheckpoint`,
`CObjectHealthCheckpoint`, `CDaredevilCheckpoint`, `CDaredevilPoint[Event/Id/VehicleDestroyed/CharacterKilled/
ModelDestroyed]`, `CDaredevilComboLevel`, `CDaredevilAnnouncement`; spawn `CSpawnParam_[Objective/Object]Critical`;
conditionals `CConditional_[IsMissionActive/IsMissionCompleted/MissionStatus/IsObjectiveActive/IsObjectiveCompleted/
IsObjectiveFailed/IsObjectiveHidden/IsObjectiveTracked/IsObjectiveCurrentInSequence/IsObjectiveContentAvailable/
IsAnyHardFailObjectiveActive/ObjectiveGoalStatus/ObjectTrackerStatus]`. Runtime strings `objectives_save_data`,
`objective_content`, `hud_objective_content_changed`, `objective_failed`, `objective_abandoned`,
`node_objective_return`, `node_objective_hardfail_default`, `operation_mission_*` (stats), map-panel
`map_panel_operations_header_[active/completed]_title`.

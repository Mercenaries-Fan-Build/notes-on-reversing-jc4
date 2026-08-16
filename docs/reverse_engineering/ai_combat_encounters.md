# AI, Combat & Encounters — how enemies think and how fights are staged

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

## Overview

Just Cause 4's enemy AI is built from a small set of **singleton systems** plus a large family of
**reflected entity components**, all instantiated by string name through the one global reflection
factory the survey already identified. Three systems form the perception/combat spine —
**`CAiSystem`** (agent thinking), **`CTargetSystem`** (who is shooting at whom), and
**`CCombatCoordinator`** (group-level fight orchestration) — while **`CEncounterManager`** is the
"director" that registers, weights, spawns and despawns the *groups* of enemies you actually fight.
Positioning is served by **`CTacticalNodeManager`** (a graph of hand/tool-authored `CTacticalNode`s with
`CTacticalConnection`s) and a **cover** layer (`CCoverVolume` + a "Cover point raycast" LOS validator).
All of the moment-to-moment decisions are gated by `CConditional_*` / `C*Condition` predicates evaluated
by the behavior VM documented in [behavior_system.md](behavior_system.md).

**Methodology caveat that dominates this doc (proven):** the decomp is a *functions-only* export. The
present, reachable code is the **registration skeleton** (which systems/components/conditions exist, their
type-ids and factory vtables), plus a few leaf helpers with real geometry (the cover raycast) and the
**save/load serializers** for encounters and tactical nodes. The **per-frame tick bodies** of `CAiSystem`,
`CTargetSystem`, `CCombatCoordinator` and `CEncounterManager` are reached **through data-section vtables
(`PTR_LAB_*`)** and are therefore *not* in this export — every one of their update/selection loops is a
**walled** item below, tagged with a resolve route. Tunable magnitudes (perception ranges, alert timers,
aggro weights, spawn budgets) live in RTPC/ADF data, not in the code.

## Key classes & functions

### Singleton systems — registered by `FUN_148f960c0` (and a twin, `FUN_146cd0bea`)

Each manager is installed with an `ArGetTypeId` reflection hash and a factory vtable pointer. (proven — read
directly from the registrar body around lines 3209030–3209670 / 4138520+.)

| Class | ArGetTypeId (hash) | Factory vtable | Role (inferred from name) |
|---|---|---|---|
| `CAiSystem` | `0x41d07588` | `PTR_LAB_141d90568` | Per-agent AI think/tick host |
| `CTargetSystem` | `0x41d8df70` | `PTR_LAB_141d8df88` | Target registry + acquisition/selection |
| `CCoverageManager` | `0x41ce7b98` | `PTR_LAB_141d90608` | Coverage grid (see caveat — likely terrain/veg coverage, not AI cover) |
| `CTacticalNodeManager` | — (via `thunk_FUN_148f6e780`) | `PTR_LAB_141d90748` | Tactical positioning-node graph |
| `CEncounterManager` | — (via `thunk_FUN_148f6e780`) | `PTR_LAB_141d90c48` | Encounter director (spawn/weight/despawn) |

`FUN_148f960c0` installs the whole world of managers in one linear body; the first ~15 use an inline
type-id + vtable write, the remainder (including `CTacticalNodeManager` and `CEncounterManager`) go through
the helper `thunk_FUN_148f6e780(registry,"<Name>",factory)`. `CCombatCoordinator` is **not** here — it is a
*component*, not a manager (below). (proven)

### Combat/encounter/tactical components — registered by `FUN_14085fd00`

Each is a tiny stub called from `FUN_14085fd00` that (a) resolves the type-id via
`FUN_140f27f60("<Name>",strlen,…)` into a `DAT_*`, and (b) installs a factory object (its `PTR_LAB_*`
vtable) into the component registry via `thunk_FUN_14cfef490(registry,typeId,factory)`. (proven — this is
the same reflected-component mechanism as vehicles/weapons.)

| Component | Registrar stub | Type-id storage | Factory vtable |
|---|---|---|---|
| `CCombatCoordinator` | `FUN_140808d00` | `DAT_142cb237c` | `PTR_LAB_141d9ba70` |
| `CComboScorer` | `FUN_140808dc0` | `DAT_142cb88b8` | `PTR_LAB_141d9c6c8` |
| `CBroadcastAwarenessEvent` | (near L861337) | `DAT_142cb2358` | `PTR_LAB_141d9ba70`-family |
| `CEncounterDependency` | `FUN_14080b4e0` | `DAT_142cb9bd8` | `PTR_LAB_141d9e5e0` |
| `CEncounterDespawnSettings` | `FUN_14080b5a0` | `DAT_142cb9be0` | `PTR_LAB_141d9e608` |
| `CEncounterEngagedController` | (L863385/947377) | `DAT_142cb97cc` | — |
| `CEncounterGroup` / `CEncounterObject` / `CEncounterRule` / `CEncounterSettings` / `CEncounterWeightModifier` / `CEncounterDropOffPoint` | siblings in same block (L863417–863545, L900812) | — | — |
| `CCoverVolume` | `FUN_14041a4c0` | `DAT_142cb2390` (strlen `0xc`) | — |
| `CCoverageObject` (L199133) | sibling | — | — |
| `CTacticalNode` / `CTacticalConnection` / `CTacticalConnectionPoint` / `CTacticalNodeResource` / `CTacticalNodeDiscover` | block L869117–869181, L1080947–1081041 | — | — |
| AI world-sim: `CAiControlObject`, `CAiHeatData`, `CAiHeatSpawner`, `CAiHeatAOOSpawnPoint`, `CAiHeatDensityVolume`, `CAiWorldSimSpawnPoint`, `CAiWorldSimModifier`, `CAiInteraction`, `CAiInteractionScene`, `CAiRaceData`, `CAiRoadGraphFilter`, `CAiTrafficObstacle` | block L366385–366638 / L860441–860793 | — | — |

### Combat behavior predicates — registered by `FUN_1485b9730` / `FUN_1485e07e0` / `FUN_1485e1370`

These are `C*Condition` type-id resolvers using `thunk_FUN_14aadec10` = our cracked **lookup3 `hashlittle`**
(the `[[name-hash-cracked]]` hash). Their eval bodies live behind the condition-VM vtables
(see [behavior_system.md](behavior_system.md)); only the registration skeleton is present. (proven)

| Condition string | Resolver | Type-id | What it gates (inferred) |
|---|---|---|---|
| `CAcquireCloseCombatTargetCondition` | `FUN_140545f90` | `DAT_142cb40cc` | Melee/close-combat target acquire |
| `CAiAimTargetDirectionCondition` | `FUN_140546110` | `DAT_142cb4544` | Aim aligned to target dir |
| `CAiUsingCoverCondition` | `FUN_140546190` | `DAT_142cb4104` | Agent currently in cover |
| `CAimingCondition` | `FUN_140546290` | `DAT_142cb41ec` | Agent is aiming |
| `CAngleBetweenGrappleAnchorAndLookCombatTarget` | `FUN_140546410` | `DAT_142cb455c` | Grapple-vs-combat-target angle |
| `CDistanceToTargetCondition` (L502358) / `CDistanceToCoverCondition` (L502339) | siblings | — | Range gates to target / to cover |
| `CFacingCoverCondition` (L502586) / `CInfrontOfCoverCondition` (L503631) | siblings | — | Cover facing/positioning |
| `CIsLeftCoverCondition` (L503973) / `CIsRightCoverCondition` (L504087) | siblings | — | Which side of cover (peek dir) |
| `CRelativeTargetCondition` (L505227) / `CRelativeNextMoveAndTargetCondition` (L505208) | siblings | — | Relative target geometry for moves |
| `CTargetOnFootMovementSpeedCondition` (L505645) | sibling | — | Target's on-foot speed band |
| `CLinkTargetEventCondition` (L504277) / `CLinkTargetInStateCondition` (L504296) | siblings | — | Linked-target event/state checks |
| `CIsTargetStuntingOnMyVehicleCondition` (L504125) | sibling | — | Player stunting on this vehicle |
| `CCombatantNotTetherableCondition` (L501978) | sibling | — | Combatant tether eligibility |
| `CIsPlayerNonCombat` (L504030) | sibling | — | Player not in combat state |
| `CBlackboardValueCompareCondition` (L501864) / `CBlackboardCursorState` (L501845) | siblings | — | Blackboard read/compare (the AI memory store) |
| `CDemonEngagedCharacterHasBlackboardValue` (L502149) | sibling | — | Scripted-"Demon" engaged check via blackboard |

## How it works (from the decomp)

### Instantiation & identity (proven)
Everything above is created **by name** through the single reflection factory `FUN_140f27f60` (the 6,138-call
thunk over the name→type lookup). Managers get an `ArGetTypeId<...>` hash (e.g. `CAiSystem = 0x41d07588`);
components get a `strlen`-checked name lookup (`"CCombatCoordinator",0x12` → `0x12` = 18 = its length);
conditions get a `hashlittle` id. The gameplay-code namespace therefore shares the **same hashing identity**
as asset paths and TAB/ADF names — the cross-cutting finding of the survey. This is the only fully-present
layer of the AI system.

### The perception → target → coordinate loop (structure proven; bodies walled)
The name inventory pins the intended data flow even though the tick bodies are behind vtables:

1. **Perception / awareness.** `CBroadcastAwarenessEvent` (a component, `DAT_142cb2358`) is the
   awareness-propagation event — an agent that sees/hears something *broadcasts* it, rather than every
   agent polling. A dedicated engine profiler zone **`"LineOfSight"` / `"StLineOfSight"`** wraps the LOS
   query work (rdtsc-timed scope around L2288572–2288577), confirming LOS is a discrete, per-frame,
   batched subsystem. (proven that the event type and the LOS timing zone exist; the sensing math is walled.)
2. **Target acquisition/selection.** `CTargetSystem` (`0x41d8df70`) owns the target set; the
   `C*TargetCondition` family (`CDistanceToTargetCondition`, `CAiAimTargetDirectionCondition`,
   `CRelativeTargetCondition`, `CTargetOnFootMovementSpeedCondition`, `CAcquireCloseCombatTargetCondition`)
   are the predicates the behavior VM uses to *choose* and *validate* a target each frame. (inferred from
   the condition names + the manager's presence.)
3. **Group coordination.** `CCombatCoordinator` (`DAT_142cb237c`) is a **component**, i.e. it is attached
   to an entity (a squad/encounter owner) rather than being a global singleton — the mechanism for
   coordinating fire, flanking and role assignment across a group. `CComboScorer` (`DAT_142cb88b8`) is a
   sibling component that scores combat "combos" (attack sequencing/scoring). Both are installed by
   `FUN_14085fd00`; their coordination logic is walled. (proven they are entity components; behavior inferred.)
4. **Blackboard as shared memory.** `CBlackboardValueCompareCondition` / `CBlackboardCursorState` /
   `CDemonEngagedCharacterHasBlackboardValue` show the AI uses a **blackboard** key/value store that both
   the behavior VM and encounter scripting read/write — the standard Apex-lineage pattern. (inferred)

### Cover selection — the one place the geometry is present (proven)
This is the most concrete reachable AI logic in the export.

- **`FUN_140384db0` — "Cover point raycast."** Given a cover node (`param_1`), a length (`param_2`) and a
  direction/offset, it computes a world-space start and end point, builds a ray-query object
  (vtable `PTR_LAB_141d07478`, tagged with the literal string `"Cover point raycast"` via `FUN_1400d07c0`),
  fires it `(**(code**)(local_1a8+0x10))(...)`, and **returns `true` when nothing was hit** (`local_138 ==
  '\0'`) — i.e. the cover position has clear line of fire / is a valid peek. (proven)
- **`FUN_14800d840` (size 1643) and `FUN_14800eaa0` (size 1117) — cover-point evaluators.** They transform a
  cover node into world space (a 3×4 matrix multiply of the node basis at `param_2+0x14…0x3c`, scaled by
  `*(param_2+0x48) * DAT_141ca71f0`), then probe **four discrete directions** using unit vectors
  `(0,0,-1)`, `(-1,0,0)`, `(1,0,0)`, `(1,0,0)`/`(0,0,1)` and gate each probe on a per-side availability
  **flag byte**: `param_2+0xa6` (up/over), `param_2+0xa5` (left), `param_2+0xa7` (right). Each enabled side
  runs `FUN_140384db0`/`FUN_140421d20` (the raycast) and records whether that peek is clear. These map
  one-to-one onto the `CIsLeftCoverCondition` / `CIsRightCoverCondition` / `CInfrontOfCoverCondition` /
  `CFacingCoverCondition` predicates. (proven for the probing structure; the exact per-flag semantics inferred
  from the matching condition names.) Both have `callers=[]` → they are invoked **only through a vtable**
  (the walled AI tick), which is why the caller list is empty.

### Encounter director — registration + persistence present, spawn loop walled
`CEncounterManager` (`PTR_LAB_141d90c48`) is the director. Its runtime spawn/weight/despawn loop is walled,
but two things are present:

- **Data model.** An encounter is authored as a graph of components: `CEncounterGroup` (a spawnable group),
  `CEncounterObject`, `CEncounterRule` + `CEncounterWeightModifier` (which groups are eligible and how they
  are weighted for selection), `CEncounterSettings`, `CEncounterDependency` (ordering/prereqs),
  `CEncounterDespawnSettings` + `CEncounterDropOffPoint` (how/where they leave), and
  `CEncounterEngagedController` (`DAT_142cb97cc`) — the controller that runs once the player **engages** an
  encounter. (proven these types exist as reflected components.)
- **Save/load.** `FUN_14087c680` (caller `FUN_14088e7d0`) serializes the encounter state keyed by
  `ArGetTypeId<CEncounterManager>` (`DAT_142cba088`) and a `std::vector<SEncounterSaveData>`
  (`DAT_142cba0b8`), iterating the live encounter list at `param_2+0x78…0x80` in `0x18`-byte strides and
  calling `FUN_14087b940` per entry. So **`SEncounterSaveData`** is the persisted per-encounter record.
  (proven.)

The **`CAiHeat*` world-sim family** (`CAiHeatData`, `CAiHeatSpawner`, `CAiHeatAOOSpawnPoint`,
`CAiHeatDensityVolume`, `CAiWorldSimSpawnPoint`) is the ambient-population/"heat" driver that decides where
and how densely to spawn AI in the open world — the encounter system's world-sim counterpart. Only their
registration is present. (proven they exist; roles inferred from names.)

### Tactical nodes — persistence present (proven)
`FUN_1409602d0` serializes the `CTacticalNodeManager`: it resolves `ArGetTypeId<CTacticalNodeManager>`
(`0x47`-char string → `DAT_142cba960`), then walks a **linked list of nodes at `param_2+0xa8`**, copying each
node's id (`*(node+0x14)`) into a vector for save. This confirms tactical nodes are a live, per-node,
persisted graph consulted by combat positioning. The node graph itself (`CTacticalNode` +
`CTacticalConnection`/`CTacticalConnectionPoint`, discovered at runtime via `CTacticalNodeDiscover`, loaded
from `CTacticalNodeResource`) is authored data. (proven for the manager's serialize; graph traversal for
combat is walled.)

## Data & config integration

- **Type-id ↔ entity component.** Each component class name here hashes (via `FUN_140f27f60`/`hashlittle`) to
  the same id used on the RTPC/ADF data side. To resolve tunables, tie e.g. `CEncounterWeightModifier`,
  `CEncounterDespawnSettings`, `CCombatCoordinator`, `CAiHeatDensityVolume`, `CTacticalNodeResource` to their
  entity-component property blobs in `[[rtpc-entity-assembly]]` / `[[composite-assets]]`. (route, not yet done.)
- **Conditions are data-driven edges.** The `C*Condition` predicates are referenced by hash id from behavior
  graphs; their *thresholds* (the distances in `CDistanceToTargetCondition`, the speed band in
  `CTargetOnFootMovementSpeedCondition`, etc.) are parameters carried in the graph/RTPC data, not constants
  in code. See [behavior_system.md](behavior_system.md) for how a condition id is dispatched. (inferred)
- **Encounters are save-backed.** `SEncounterSaveData` (per-encounter) is written through the ADF/serialize
  path keyed on the `CEncounterManager` type-id — encounter progress is part of the save game. (proven)

## Notable constants / tunables (present in code)

Almost all AI tunables are data-side; the few *code* constants present are identity hashes and geometry
scales, not gameplay magnitudes:

| Constant | Where | Meaning |
|---|---|---|
| `0x41d07588` | `FUN_148f960c0` | `ArGetTypeId(CAiSystem)` |
| `0x41d8df70` | `FUN_148f960c0` | `ArGetTypeId(CTargetSystem)` |
| `0x41ce7b98` | `FUN_148f960c0` | `ArGetTypeId(CCoverageManager)` |
| `DAT_142cb237c` | `FUN_140808d00` | `ArGetTypeId(CCombatCoordinator)` (runtime-resolved) |
| `DAT_142cba088` | `FUN_14087c680` | `ArGetTypeId(CEncounterManager)` (save key) |
| `DAT_142cba960` | `FUN_1409602d0` | `ArGetTypeId(CTacticalNodeManager)` (save key) |
| `DAT_141ca71f0` | `FUN_14800d840` | cover-offset scale applied to node basis before probing |
| `DAT_142c73aa8` / `DAT_142c73ab0` | `FUN_14800d840` | offset vector fed into the cover raycast start |
| cover probe dirs `(0,0,-1)(-1,0,0)(1,0,0)(0,0,1)` | `FUN_14800d840`/`FUN_14800eaa0` | the four cover peek directions |

No perception ranges, alert timers, aggro weights, damage/health or spawn budgets appear as literals in the
reachable code — consistent with the project-wide "mechanism in code, magnitudes in data" result. (proven by
absence across the read functions.)

## Call-graph highlights

- `FUN_148f960c0` (twin `FUN_146cd0bea`) → installs `CAiSystem`, `CTargetSystem`, `CCoverageManager`,
  and via `thunk_FUN_148f6e780` → `CTacticalNodeManager`, `CEncounterManager`. (manager world-install)
- `FUN_14085fd00` → `FUN_140808d00` (`CCombatCoordinator`), `FUN_140808dc0` (`CComboScorer`),
  `FUN_14080b4e0`/`FUN_14080b5a0`/… (encounter component family), cover/tactical/AI-heat stubs. (component install)
- `FUN_1485b9730` / `FUN_1485e07e0` / `FUN_1485e1370` → the `C*Condition` resolvers (`FUN_140545f90`,
  `FUN_140546110`, `FUN_140546190`, …) via `thunk_FUN_14aadec10` (hashlittle). (condition install)
- Cover: `FUN_14800d840` / `FUN_14800eaa0` (evaluators, called via vtable) → `FUN_140384db0`
  ("Cover point raycast") → ray-query vtable `PTR_LAB_141d07478`.
- Encounter save: `FUN_14088e7d0` → `FUN_14087c680` → `FUN_14087b940` (per `SEncounterSaveData`).
- Tactical save: `FUN_1409638b0` → `FUN_1409602d0` (walks node list at `+0xa8`).

## Open questions / lower-confidence

All are **walled** (data-section vtable / omitted body / data-side value). Resolve routes given.

1. **`CAiSystem` per-agent think loop** — vtable `PTR_LAB_141d90568`. *Route:* in x64dbg, dump the vtable at
   `PTR_LAB_141d90568`, set a read/exec bp on the Update slot, walk the sensing/decision code live.
2. **`CTargetSystem` acquisition & threat scoring** — vtable `PTR_LAB_141d8df88`. Same route; correlate with
   the `C*TargetCondition` ids to find where each predicate reads target geometry.
3. **`CCombatCoordinator` group logic** (fire discipline, flanking, role assignment) — component factory
   `PTR_LAB_141d9ba70`; walled. *Route:* trace an instance's vtable during a live firefight.
4. **`CEncounterManager` spawn/weight/despawn loop** — `PTR_LAB_141d90c48`. *Route:* bp on the
   `CEncounterRule`/`CEncounterWeightModifier` reads; decode the `CEncounter*` RTPC components for budgets.
5. **Perception model magnitudes** — LOS ranges, FOV, hearing radius, alert-escalation timers. Not in code.
   *Route:* RTPC entity components for `CAiHeat*` / character perception + the `"LineOfSight"` job body.
6. **`CCoverageManager` disambiguation** — the `CoverageResolve`/`CoverageUVMap`/`CoverageOverlay` GPU-pass
   strings (L3542001+) strongly suggest this manager is a **render/terrain coverage grid**, *not* the AI
   cover system (which is `CCoverVolume` + the raycast). Treat as separate until proven. (speculative)
7. **Blackboard schema** — the set of AI blackboard keys is data-side; only the compare/cursor condition
   types are visible. *Route:* dump keys via a bp on `CBlackboardValueCompareCondition`'s eval.

Character/creature/player lifecycle (spawning, health, ragdoll) is a **separate doc**; this doc only touches
it where combat intersects (target eligibility, `CAiHeat*` population) — see that doc for the agent bodies.

## Appendix — decomp anchors

Strings and addresses used, all in `output/_ghidra_jc4/jc4_all_functions_decomp.txt`:

- Manager registrar: `FUN_148f960c0` @L4138520 (twin `FUN_146cd0bea` @L3209030). `"CAiSystem"` L3209207/4138689
  (id `0x41d07588`, vtable `PTR_LAB_141d90568`); `"CTargetSystem"` L3209241 (id `0x41d8df70`,
  `PTR_LAB_141d8df88`); `"CCoverageManager"` L3209309 (id `0x41ce7b98`, `PTR_LAB_141d90608`);
  `"CTacticalNodeManager"` L3209430 (`PTR_LAB_141d90748`); `"CEncounterManager"` L3209630 (`PTR_LAB_141d90c48`).
- Component registrar hub: `FUN_14085fd00`. `CCombatCoordinator` stub `FUN_140808d00` @L861731
  (`DAT_142cb237c`, `PTR_LAB_141d9ba70`); type accessor `FUN_14041a430` @L366694 (`"CCombatCoordinator",0x12`).
  `CComboScorer` `FUN_140808dc0` @L861763 (`PTR_LAB_141d9c6c8`). `CBroadcastAwarenessEvent` `DAT_142cb2358`
  L366684/861337. `CCoverVolume` `FUN_14041a4c0` @L366717 (`"CCoverVolume",0xc`).
- Encounter components: `"CEncounterDependency"` `FUN_14080b4e0` @L863300 (`DAT_142cb9bd8`, `PTR_LAB_141d9e5e0`);
  `"CEncounterDespawnSettings"` `FUN_14080b5a0` @L863331 (`DAT_142cb9be0`, `PTR_LAB_141d9e608`);
  `"CEncounterEngagedController"` L863385/947377 (`DAT_142cb97cc`, `"…",0x1b`); `CEncounterGroup/Object/Rule/
  Settings/WeightModifier` L863417–863545; `CEncounterDropOffPoint` L900812.
- Tactical: `"CTacticalNode/Connection/ConnectionPoint"` L869117–869181; `CTacticalNodeResource/Discover`
  L1081016–1081041. Save: `FUN_1409602d0` @L1072012 (`ArGetTypeId<CTacticalNodeManager>` `0x47`, `DAT_142cba960`).
- AI world-sim: `CAiControlObject/HeatAOOSpawnPoint/HeatData/HeatDensityVolume/HeatSpawner/Interaction/
  InteractionScene/RaceData/RoadGraphFilter/TrafficObstacle/WorldSimModifier/WorldSimSpawnPoint`
  L366385–366638 (dup L860441–860793).
- Conditions (`thunk_FUN_14aadec10` = hashlittle; registrars `FUN_1485b9730`/`FUN_1485e07e0`/`FUN_1485e1370`):
  `CAcquireCloseCombatTargetCondition` `FUN_140545f90` @L501494; `CAiAimTargetDirectionCondition` `FUN_140546110`;
  `CAiUsingCoverCondition` `FUN_140546190`; `CAimingCondition` `FUN_140546290`;
  `CAngleBetweenGrappleAnchorAndLookCombatTarget` `FUN_140546410`. Sibling strings:
  `CBlackboardCursorState` L501845, `CBlackboardValueCompareCondition` L501864,
  `CCombatantNotTetherableCondition` L501978, `CDemonEngagedCharacterHasBlackboardValue` L502149,
  `CDistanceToCoverCondition` L502339, `CDistanceToTargetCondition` L502358, `CFacingCoverCondition` L502586,
  `CInfrontOfCoverCondition` L503631, `CIsLeftCoverCondition` L503973, `CIsPlayerNonCombat` L504030,
  `CIsRightCoverCondition` L504087, `CIsTargetStuntingOnMyVehicleCondition` L504125,
  `CLinkTargetEventCondition` L504277, `CLinkTargetInStateCondition` L504296,
  `CRelativeNextMoveAndTargetCondition` L505208, `CRelativeTargetCondition` L505227,
  `CTargetOnFootMovementSpeedCondition` L505645.
- Cover geometry: `"Cover point raycast"` `FUN_140384db0` @L304771 (vtable `PTR_LAB_141d07478`); evaluators
  `FUN_14800d840` @L3708692, `FUN_14800eaa0` @L3709022; scale `DAT_141ca71f0`, offset
  `DAT_142c73aa8`/`DAT_142c73ab0`; side flags `param_2+0xa5/0xa6/0xa7`.
- Encounter save: `FUN_14087c680` @L925196 (`ArGetTypeId<CEncounterManager>` L925216 `DAT_142cba088`;
  `std::vector<SEncounterSaveData>` L925248 `DAT_142cba0b8`); per-entry `FUN_14087b940`; caller `FUN_14088e7d0`.
- Perception profiling: `"StLineOfSight"`/`"LineOfSight"` L2288572–2288577.
- Reflection factory: `FUN_140f27f60` (name→type). Hash thunk: `thunk_FUN_14aadec10` (lookup3 hashlittle).

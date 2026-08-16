# Characters, Creatures & the Character Controller — the NPC/player/creature manager layer

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative). "Proven" = read
directly from the Ghidra decomp (`output/_ghidra_jc4/jc4_all_functions_decomp.txt`); a `FUN_<addr>` is cited so
any claim is re-verifiable.

> Scope. This doc covers the **manager layer** for living entities: how characters/creatures are
> created, pooled and torn down; the health/damage model; the player-vs-NPC distinction; creature/wildlife
> spawning; faction membership; and the ragdoll/skeleton object layer.
> The shared **state-task / behavior machinery** (`NStateTask_*`, `ACT_*`, `C*Condition`) is documented in
> [behavior_system.md](behavior_system.md); traversal-side character locomotion in
> [traversal_movement.md](traversal_movement.md); combat AI in [ai_combat_encounters.md](ai_combat_encounters.md);
> the region-control metagame in [faction_frontline_chaos.md](faction_frontline_chaos.md). This doc references
> those and does not duplicate them.

## Overview

Living entities in JC4 (Apex engine) are split across three singleton **managers** — `CCharacterManager`
(humanoid NPCs + the shared character path), `CPlayerManager` (the player avatar and player-specific state),
and `CCreatureManager` (wildlife/animals, backed by a fixed pool of lightweight rigid bodies). All three are
registered by **name** into one global manager registry through the same primitive
(`thunk_FUN_148f6e780` inside registrar `FUN_148f960c0`), each getting an 8-byte stub whose only field is a
data-section vtable pointer. (proven)

The engine is uniformly **name→hash→factory reflective**: gameplay classes (`CCharacter`, `CHealthControl`,
`CDamageController`, `CRagdollObject`, …) are registered as `type-id → factory` pairs via `FUN_140f27f60`
("intern this class name, return its type id") and the component builder `FUN_14085fd00`; AI **condition**
classes (`CIsPlayerCondition`, `CHealthCondition`, `CNpcIsRunningCondition`, …) via `thunk_FUN_14aadec10`.
The hash is our already-cracked lookup3 `hashlittle` (`docs/formats/name_hash.md`). (proven)

The recurring wall (per the project methodology caveat): the **registration skeletons, health/death tick,
struct field offsets, collision layers, ragdoll bone map and spawn-ring distances are recoverable**, but the
per-frame **manager tick bodies** and per-hit **damage-apply logic** sit behind data-section `PTR_LAB_*`
vtables and string-keyed message dispatch, so they are not in the functions-only export. Those are tagged
**OPEN** with a resolve route.

## Key classes & functions

| Name string | Registration `FUN_` | Vtable / id | Role |
|---|---|---|---|
| `CCharacterManager` | `FUN_148f960c0` @0x148f960c0 (line 4138972); dup `FUN_146cd0bea` @0x146cd0bea (3209490) | `PTR_LAB_141d908e8` | NPC/humanoid character manager (singleton) |
| `CPlayerManager` | `FUN_148f960c0` (4138977); dup `FUN_146cd0bea` (3209495) | `PTR_LAB_141d90908` | Player-avatar manager (singleton) |
| `CCreatureManager` | system entry in `FUN_146cd0bea` (3209705); pool init `FUN_14067ccd0` @0x14067ccd0 | `PTR_LAB_141d90e48` | Wildlife/animal manager, 64-slot rigid-body pool |
| `CPlayerSpawnPointManager` | `FUN_146cd0bea` (3209500) | `PTR_LAB_141d90928` | Player spawn-point registry |
| `CPlayerReportingManager` | `FUN_148f960c0` (4139042); `FUN_146cd0bea` (3209560) | — | Player telemetry/reporting |
| `CDamageCoordinator` | `FUN_148f960c0` (4139237); `FUN_146cd0bea` (3209755) | — | Global damage subsystem (area/omni) |
| `CCharacter` (type-id) | getter `FUN_14053f2a0` @0x14053f2a0 (`FUN_140f27f60("CCharacter",10)`) | `DAT_142cb3d30` | The character game-object class |
| `CHealthControl` | getter `FUN_140b0a210` @0x140b0a210 | `DAT_142cb964c` | Per-character health object |
| `CHealthbarComponent` | `FUN_1408e0760` @0x1408e0760 | `DAT_142cba284` | Health-bar UI component |
| `CDamageController` | `FUN_140809c00` @0x140809c00 (via `FUN_14085fd00`) | `PTR_LAB_141d9dac8` / `DAT_142cb91c0` | Per-entity damage router |
| `CDamageControllerHitAnalyze` | `FUN_140809cc0` @0x140809cc0 | `PTR_LAB_141d9db18` / `DAT_142cb95fc` | Damage sub-controller (analyze) |
| `CDamageControllerHitApply` | `FUN_140809d80` @0x140809d80 | `PTR_LAB_141d9daf0` / `DAT_142cb95f4` | Damage sub-controller (apply) |
| `CAreaDamage` | `FUN_140677850` @0x140677850 | `DAT_142cb7de0` | Radial/area damage object |
| `CRagdollObject` | helper `FUN_140811ff0` @0x140811ff0 | `PTR_LAB_141d9b700` / `DAT_142cb95c4` | Character ragdoll body |
| `CRagdollAttachment` | getter `FUN_14053f330` @0x14053f330 | `PTR_LAB_141d9bb60` / `DAT_142cb3d58` | Ragdoll attachment |
| `CRagdollBoneProxyObject` | helper `FUN_1408463b0` @0x1408463b0 | `_DAT_142cb9604` | Ragdoll bone-proxy collision body |
| `CObjectiveCharacterController` | `FUN_140810f90` @0x140810f90 | `DAT_142cb97d4` | Mission-scripted character controller |
| `CPlayerGameplayConfiguration` | reg @867419/1055937 | `DAT_142cb92a8` | Player tuning component |
| `CCreatureRigidObject` (type-id) | `FUN_140f27f60("CCreatureRigidObject",0x14)` | — | Pooled creature rigid body |
| `CBirdSpawner` | `FUN_140794d60` @0x140794d60 | `DAT_142cb88a0` | Bird-flock spawner |
| `CAiWorldSimSpawnPoint` | `FUN_14041a280` @0x14041a280 | `DAT_142cb2350` | AI-world-sim spawn point |
| `SAiWorldSim_Animals` (config) | ctor `FUN_1404674b0` @0x1404674b0 | `PTR_LAB_141ceba60` | Ambient-wildlife population config |
| `CSpawnSystem` | `FUN_146cd0bea` (3209190, hash 0x41d8de38) | `PTR_LAB_141d90548` | Data-driven spawn engine |

**Condition classes** (player/NPC/health discriminators, registered via `thunk_FUN_14aadec10` in the
condition-RTTI builders `FUN_1485e07e0` / `FUN_1485b9730`):

| Condition | Getter `FUN_` | id |
|---|---|---|
| `CIsPlayerCondition` | `FUN_14054a190` @0x14054a190 | `DAT_142cb431c` |
| `CIsPlayerNonCombat` | `FUN_14054a210` @0x14054a210 | `DAT_142cb435c` |
| `CPlayerIsAutoAimingCondition` | `FUN_14054b690` @0x14054b690 | `DAT_142cb4324` |
| `CNpcIsRunningCondition` | `FUN_14054b110` @0x14054b110 | `DAT_142cb424c` |
| `CCharacterIsAliveCondition` | reg @501921 | `DAT_142cb42fc` |
| `CHealthCondition` | `FUN_140549090` @0x140549090 | `DAT_142cb458c` |
| `CLinkHealthCondition` | `FUN_14054a810` @0x14054a810 | `DAT_142cb4584` |
| `CIsTakingDamageCondition` | `FUN_14054a410` @0x14054a410 | `DAT_142cb448c` |
| `CVehicleDamageState` | `FUN_14054d510` @0x14054d510 | `DAT_142cb409c` |

## How it works (from the decomp)

### Manager registration & the registry

All three managers ride the same registration primitive. In registrar `FUN_148f960c0` (and a byte-identical
copy inside `FUN_146cd0bea`), each manager is created by the uniform triple (proven):

```c
puVar3 = thunk_FUN_1496a12b0(8);              // alloc an 8-byte manager stub
if (puVar3) *puVar3 = &PTR_LAB_141d908e8;     // store the manager's data vtable (here: CCharacterManager)
thunk_FUN_148f6e780(param_1,"CCharacterManager",puVar3);   // register name -> object
```

`thunk_FUN_148f6e780` (`FUN_148f6e780`, size 61) appends a 16-byte `{name_ptr, object_ptr}` pair to a growable
vector on the registry object — `*(registry+0x48)` = end pointer, `*(registry+0x50)` = capacity end, grown by
`FUN_140077480` when full. So the manager registry is effectively `vector<pair<const char*, IManager*>>` at
`registry+0x40..0x50`. (proven) The manager stub itself is 8 bytes = *just a vtable pointer*; all state lives
in the object the vtable methods build.

**OPEN — manager ticks.** `PTR_LAB_141d908e8` (CCharacterManager), `PTR_LAB_141d90908` (CPlayerManager) and
`PTR_LAB_141d90e48` (CCreatureManager) are data-section labels that are only ever *assigned* in the dump — no
defining block, no resolvable method addresses. The per-frame pool/update/tick logic of all three managers is
therefore not in the functions-only export. Resolve route: read `.rdata` at those vtable addresses in
`JustCause4.exe` (or set a breakpoint on the vtable slot in x64dbg while PAUSED).

### The CCreatureManager pool builder (fully readable) — `FUN_14067ccd0`

Unlike the character/player managers, the creature manager's **init** routine is present and readable. It
pre-allocates a **fixed 64-slot pool of `CCreatureRigidObject`** and wires the manager into the engine update
registry (proven):

- `uVar4 = FUN_140f27f60("CCreatureRigidObject",0x14)` gets the class id; then a `do…while` loop of **64**
  iterations (`lVar10 = 0x40`) walks `plVar11 = manager + 0x9340`, advancing `+= 2` pointers (0x10 bytes) each
  pass. So **`manager+0x9340`** is an array of 64 ref-counted creature handles (`{object*, controlblock*}`),
  ending near `0x9740`. Each slot is instantiated via
  `FUN_1402fa680(DAT_142cafdf8, &handle, classId, "CCreatureManager")` — the instance factory
  (`operator new` + ctor via `FUN_1417cde00`, wrapped in a refcounted smart handle); the `"CCreatureManager"`
  string is the owner/debug tag.
- Per slot it builds a stack template seeded from `_DAT_142abe0a0…` with `0xdeadbeef` handle sentinels,
  allocates a 0x120-byte sub-object (`thunk_FUN_1496a12b0(0x120)` → `FUN_1400ceb10`), ORs flag `4` into
  `*(obj+0x20)`, calls `FUN_140b30890`, and sets `*(slot+0x570) |= 0x20`. (per-creature component init; body
  behind `FUN_140b30890` OPEN)
- Builds a global creature service object `DAT_142cb6078` (0xb0 bytes, vtable `PTR_FUN_141d511d8`) and
  registers it into the global update registry `DAT_142c84be8` (`thunk_FUN_14a524650`). Acquires engine
  subsystem/channel **#0x5b** at `manager+0x9788` (`FUN_140d5edf0(DAT_142c84be8, 0x5b, 1)`), and a companion
  listener object at `manager+0x9790` (vtable `PTR_LAB_141d512a8`). (proven it registers; service bodies OPEN)
- Second pass over the 64 slots calls a per-object virtual init (`(*(obj+2 vtbl+0x38))(obj,0)`) and sets
  `*(slot+0x1da) |= 0x40` (status flag — inferred "pooled/available").

The matching destructor **`FUN_14067d2e0`** @0x14067d2e0 confirms this layout: it unregisters channel `+0x9788`,
destroys four global creature lists (`DAT_142cb6040/6060/6080/6028`), and frees `*(manager+0x9740)`. (proven)

Readable `CCreatureManager` offsets:

| Offset | Meaning |
|---|---|
| `+0x9340` | base of 64× `CCreatureRigidObject` handle array (0x10 each) |
| `+0x9740` | end of pool block |
| `+0x9788` | engine subsystem/update channel handle (#0x5b) |
| `+0x9790` | companion listener object (`PTR_LAB_141d512a8`) |
| slot `+0x1da` | status flag (bit 0x40 = pooled/available) |
| slot `+0x570` | init flag bit 0x20 |

**Character/creature split (inferred):** creatures are a fixed pool of lightweight *rigid-body* creature
objects with a separate memory budget category `"Characters_Animals"` (resource-category enum case 0x12,
line 3321580), distinct from `"Characters"` (case 0x11). The character manager handles the heavier humanoid
NPC path (full `CCharacter` objects, skeleton + ragdoll + AI). This split is inferred from the registration
+ budget categories; the manager tick bodies that would prove it are OPEN.

### Health & the death tick (fully readable) — `FUN_140e60340`

The per-character health/death driver `FUN_140e60340` (size 935) reads the health object at
`hc = *(*(DAT_142cb2388 + 0x30) + 0x1c8)` and broadcasts state changes through the component **message bus**
`(**(code**)(*this + 0x20))(this, "MsgName", …)`. It is gated by `DAT_142aeacd8 != 2 && != 5` (paused/loading
states). Readable health-object fields and the exact broadcasts (all proven, read directly):

- `hc+0x78` (bool near-death) → on change vs cache `this+0x1a`, sends **`"EnterNearDeath"`** / `"ExitNearDeath"`.
- `hc+0x54` (bool dead) → on transition vs cache `this+0x1b`, sends **`"EnterDeath"`** / `"ExitDeath"`; while
  dead it sends **`"HandleDeath"`** with payload `= *(float)(hc+0x64) / *(float)(hc+0x58)` (current HP / max HP).
- `hc+0x74` (float current health) → when `*(short)(hc+0x3ac)` differs from cached `this[3]`, sends
  **`"UpdateHealth"`** (2 args) and updates the cache.
- Also builds and sends **`"UpdateCharacterDmgIndicators"`** from `hc+0x90` each tick.

So the health struct layout is: `+0x54` dead flag, `+0x58` max HP (float), `+0x64` current HP (float),
`+0x74` current health value (float), `+0x78` near-death flag, `+0x3ac` health short (dirty-check). The
component caches last-seen values at `this+0x1a` (near-death), `this+0x1b` (dead), `this[3]` (health short).
(proven)

**Death → ragdoll is a message hop.** `"HandleDeath"`/`"EnterDeath"` cross the string-message vtable boundary;
the handler that actually activates the ragdoll (`RagdollInstanceImplTryActivate`, string @38608) is dispatched
by hash and is **OPEN**. The killing hit's impulse is applied through the physics-behavior hooks `"OnDamage"`,
`"RagdollForceToBone"`, `"ImpulseToBone"` (all registered as a physics behavior name table in
`FUN_147759a20`); ragdoll params load from `animations/ragdoll/ragdoll_params.brdd` (string @509145). The
death→ragdoll linkage is inferred (medium confidence) from these anchors; the direct call is OPEN.

Related health entry points (all proven forwarders onto the message bus):

- **`FUN_140e539c0`** — `"InitHealth"`: seeds `this[3]=0xffff`, reads max-HP short `*(short)(hc+0x3aa)`.
- **`FUN_140de8800`** — `"SetHealth"` (mech/vehicle variant): computes `ratio = health_short/max_short`, scales
  by float `DAT_141ca9c7c` (inferred ~100.0), also drives `"SetMechHealthState"` (0/1/2, 2=destroyed).
- **`FUN_140e14900`** — `"OnHealthbarHit"` (packs a hit id, 1 arg).
- **`FUN_140e53480`** — `"OnOmniDamage"`: sets `*(float)(DAT_142cb7dc8+0x8c)=2.0f`, broadcasts to player global
  `DAT_141eef470`. (a full/global damage pulse)
- **`FUN_14a8a0ca0`** — `"OnPlayerDidDamage"`: bare notifier (score/UI), no payload.

### The damage pipeline

`CDamageController` (`FUN_140809c00`, vtable `PTR_LAB_141d9dac8`) is the per-entity damage router; it owns two
sub-controllers registered immediately alongside it: `CDamageControllerHitAnalyze` (`FUN_140809cc0`,
`PTR_LAB_141d9db18`) and `CDamageControllerHitApply` (`FUN_140809d80`, `PTR_LAB_141d9daf0`). Naming + shared
parent imply the order **Analyze → Apply** (inferred). Apply mutates the `CHealthControl` fields above; the
`FUN_140e60340` tick then reads them and broadcasts. The global **`CDamageCoordinator`** (registered as a
subsystem in `FUN_146cd0bea`/`FUN_148f960c0`) orchestrates area/omni damage, fed by `CAreaDamage`
(`FUN_140677850`) and `OnOmniDamage`. The Analyze/Apply per-hit bodies live behind their data vtables — **OPEN**.

`"DefaultFireDamageConstants"` is built as a named 240-byte (`0xf0`) constant block in `FUN_140d04e00`
(`this+0x28`); the actual fire-damage float values are in the referenced data resource, not inline — **OPEN**.
`"PlayerDamageEffect"` (+`…Passthrough`, `…HdrTv`) is the red-screen post-process feedback shader chain
(`FUN_140ce7040`), not health math. (proven)

### Player-vs-NPC distinction

The player/NPC distinction is expressed as a **class-level split**, not a single runtime flag on `CCharacter`
(inferred). The player path uses dedicated condition/controller classes — `CIsPlayerCondition`
(`FUN_14054a190`), `CIsPlayerNonCombat` (`FUN_14054a210`), `CPlayerIsAutoAimingCondition` (`FUN_14054b690`),
`CPlayerInKillVolumeChecker` (`FUN_140846320`), plus the `CPlayer*` managers/config — while NPC logic uses
`CNpc*` classes such as `CNpcIsRunningCondition` (`FUN_14054b110`). Mission-scripted characters get a
`CObjectiveCharacterController` (`FUN_140810f90`). The concrete `Evaluate()` bodies that test "is this character
the player" sit behind the condition vtables and are **OPEN**. The existence of `CIsPlayerNonCombat` implies the
player character also carries a combat-state the conditions read. (proven registration; inferred mechanism)

### Ragdoll & skeleton object layer

Characters bind to a Havok ragdoll via a small proxy skeleton (proven):

- **Ragdoll factory classes:** `CRagdollObject` (`FUN_140811ff0`, factory vtable `PTR_LAB_141d9b700`),
  `CRagdollAttachment` (`FUN_14053f330`, `PTR_LAB_141d9bb60`), `CRagdollBoneProxyObject` (`FUN_1408463b0`).
- **Bone map:** a magic-static block in `FUN_14050f3a0` (@0x14050f3a0, line 479478) interns lookup3 hashes for
  a **10-bone proxy skeleton** — `ragdoll_Hips, ragdoll_Spine, ragdoll_Spine1, ragdoll_Spine2, ragdoll_Head,
  ragdoll_RightShoulder, ragdoll_LeftShoulder, ragdoll_RightUpLeg, ragdoll_LeftUpLeg` (`+ ragdoll_Spine4` in a
  second variant) → `DAT_142cb3d80…3dc0`.
- **Bone binding:** comparator `FUN_1478ad7b0` does `strncmp(a,"ragdoll_",8)` on *both* operands, skips the
  prefix, then char-compares — i.e. it matches a ragdoll proxy bone to its skeleton bone ignoring the
  `ragdoll_` prefix. This is the character↔ragdoll bone binding.
- **Skeleton pose job:** `FUN_141b88110` (size 3309) is the Havok `hkaQuantizedSampleAndCombineJob`, profiled
  `"StAllocateAndDMASkeleton"` (@2928983); reads `+0x40` bone count, `+0x48` reference bones, `+0x50` pose
  buffers — the character skeleton evaluation feeding the ragdoll.
- **Collision layers:** `FUN_1400cf270` (@0x1400cf270, via `FUN_1410eb630(tbl, idx, name, mask)`) declares the
  character physics collision-layer table: `CHARACTER_COLLISION`=6, `CHARACTER_GHOST_MODE`=7,
  `CHARACTER_RAGDOLL`=8, `CHARACTER_RAGDOLL_KEYFRAMED`=9, `CHARACTER_ATTACHED_DYNAMIC_RB`=10,
  `CHARACTER_RAGDOLL_PROXIES`=11 (lines 38293–38319), each with a 32-entry collision bitmask.

The ragdoll/hit-react `ACT_*` action set (queued through the shared behavior graph, see behavior_system.md):
`ACT_HITREACT_RAGDOLL`, `ACT_HITREACT_BULLET/FLY/GRAPPLE/STUMBLE(_BULLET/_MELEE)`,
`ACT_HITREACT_VEHICLE_IMPACT_*`, `ACT_RAGDOLL_IMPACT`, `ACT_RAGDOLL_INAIR_STABILIZATION_FWD/BWD`,
`ACT_FULL_RAGDOLL`, `ACT_GRAPPLE_RAGDOLL`, `ACT_FORCE_DEATH_RAGDOLL` (registered as hash IDs via
`FUN_140f27f60`, e.g. `ACT_HITREACT_RAGDOLL` @3082893). (proven)

### Faction membership

Per-NPC faction membership is **data-driven** and mostly walled (proven registration; logic OPEN):

- `CConditional_FactionStatus` — the behavior/quest conditional that gates on a character's faction. Interned by
  `FUN_14065ee60`; class-registered in the conditional registrar `FUN_148f99a90` with vtable
  `PTR_LAB_141d92338`. The `Evaluate()` (reads faction, returns pass/fail) is behind that data vtable — **OPEN**.
- `CObjectTrackerFilterFaction` — HUD/object-tracker predicate that shows/hides blips by faction; component
  registered by `FUN_1408100b0` (getter `FUN_14090df80`), vtable `PTR_LAB_141d9d2d0` — predicate **OPEN**.
- `change_character_faction` — the faction-mutation **event**. `FUN_140554d40` (line 508620) tokenizes,
  lowercases and hashes the name (`thunk_FUN_14762d8a0`) and subscribes a slot at `param+0x2560` to the entity
  event bus. This is a name-hashed message subscription; the handler that mutates the faction field is
  hash-dispatched and **OPEN** (`+0x2560` is the subscription slot, not the field). (proven mechanism)
- **The faction relationship matrix is a binary asset.** Global gameplay init `FUN_14085a3a0` preloads
  `settings/factions.bin` via `FUN_140302dc0` (line 913152). Strong evidence that faction↔faction
  friendly/neutral/hostile and the faction id list live there, not in code — hostility between two characters is
  a `(factionA_hash, factionB_hash)` lookup in this table. Table contents **OPEN** (binary asset). (proven the
  file loads; inferred it holds the matrix)
- `"SetFaction"` (`FUN_140e114e0`) is a Scaleform/HUD call in a DLC results screen — cosmetic, not the NPC
  system. Boundary note vs [faction_frontline_chaos.md](faction_frontline_chaos.md): that doc covers the
  **region-control metagame** (frontline advance/retreat, tactical nodes, chaos economy). This section covers
  **per-NPC faction allegiance + the inter-faction hostility matrix** — distinct concerns that only intersect in
  sharing the faction-name hashes and the registrars (`FUN_148f99a90`, `FUN_14085fd00`).

### Creature / wildlife / ambient spawning

Wildlife and data-driven spawning are reflected-class singletons the `CSpawnSystem` instantiates (proven types;
runtime spawn/cull OPEN behind `PTR_LAB_141d90548`):

- **`SAiWorldSim_Animals`** (`FUN_1404674b0`, caller `FUN_1403c89a0`) — the ambient-wildlife population config.
  Its ctor names the config block (`FUN_1400d07c0(this+0x6b,"SAiWorldSim_Animals")`) and writes float
  **distance rings** `100.0` (`0x42c80000`), `200.0` (`0x43480000`), `400.0` (`0x43c80000`), `500.0`
  (`0x43fa0000`) — the spawn/despawn radii for animal density around the player — plus per-species/zone
  sub-lists. Consumer tick behind `PTR_LAB_141ceba60` — OPEN. (proven config; inferred ring semantics)
- **`CBirdSpawner`** (`FUN_140794d60`) — bird-flock spawner. **`CAiWorldSimSpawnPoint`** (`FUN_14041a280`) — the
  AI-world-sim spawn-point type.
- Spawn/despawn config types (all reflected singletons via `FUN_140f27f60`): `CObjectSourceSpawners`
  (`DAT_142cb9a28`), `CDespawnVolume` (`DAT_142cb96e4`), `CEncounterDespawnSettings` (`DAT_142cb9be0`),
  `CAmbientEffects` (`DAT_142cb8870`). Despawn mechanism (inferred): `CDespawnVolume` = a world trigger region,
  `CEncounterDespawnSettings` = per-encounter distance/timer tunables the `CSpawnSystem` uses to cull entities.
- **No dedicated `CPedestrian`/ambient-ped class surfaced.** Ambient population is handled through the AI World
  Sim (`SAiWorldSim_Animals` + `CAiWorldSimSpawnPoint`) and `CObjectSourceSpawners`, not a separate pedestrian
  manager. (`"Population of Solis"` @4717171 is a credits/cast string, not a feature.) (proven-by-absence for the
  searched strings)

## Data & config integration

- **Manager identity** — `CCharacterManager`/`CPlayerManager`/`CCreatureManager` names hash via lookup3
  `hashlittle` (`docs/formats/name_hash.md`); this is the same hash used for asset paths and ADF/TAB names, so
  gameplay-code and asset namespaces share one identity.
- **Component config → RTPC/ADF.** `CPlayerGameplayConfiguration`, `CObjectiveCharacterController`,
  `CCharacterRule`, `CCharacterInfoDescription`, `CHealthbarComponent` and the damage/health controllers are
  registered as reflected components (`FUN_14085fd00`); their tunable **values** live in RTPC entity components
  (`.epe`) and ADF configs (memory `[[rtpc-entity-assembly]]`, `[[composite-assets]]`). Map each class name here
  to its entity-component class hash there to recover the magnitudes.
- **Faction data** → `settings/factions.bin` (loaded by `FUN_140302dc0` @913152). Resolve with `jc4_adf` /
  binary inspection.
- **Ragdoll data** → `animations/ragdoll/ragdoll_params.brdd` (string @509145); the proxy skeleton is the AMF
  model skeleton (memory `[[model-amf]]`) bound via the `ragdoll_` prefix comparator.
- **Wildlife density** → the `SAiWorldSim_Animals` ring distances are inline (100/200/400/500 m), but per-species
  entries are in the AI-world-sim config data (OPEN).

## Notable constants / tunables (read straight from the decomp)

| Value | Where | Meaning |
|---|---|---|
| 64 (`0x40`) | `FUN_14067ccd0` | creature rigid-body pool size |
| `+0x9340` / `+0x9788` / `+0x9790` | `FUN_14067ccd0` | creature pool base / update channel / listener |
| channel `0x5b` | `FUN_14067ccd0` | creature manager engine subsystem id |
| `hc+0x54/+0x58/+0x64/+0x74/+0x78/+0x3ac` | `FUN_140e60340` | dead / max HP / cur HP / health / near-death / health-short |
| `this+0x1a/+0x1b/[3]` | `FUN_140e60340` | component caches: near-death / dead / health-short |
| `2.0f` (`0x40000000`) | `FUN_140e53480` | omni-damage pulse magnitude at `DAT_142cb7dc8+0x8c` |
| `0xf0` (240 B) | `FUN_140d04e00` | `DefaultFireDamageConstants` block size (values OPEN) |
| `100/200/400/500` m | `FUN_1404674b0` | `SAiWorldSim_Animals` spawn/despawn distance rings |
| layers 6–11 | `FUN_1400cf270` | character collision layers incl. `CHARACTER_RAGDOLL`=8, `…_PROXIES`=11 |
| 10-bone proxy | `FUN_14050f3a0` | ragdoll skeleton (Hips/Spine{,1,2,4}/Head/{L,R}Shoulder/{L,R}UpLeg) |

## Call-graph highlights

- Manager install: `FUN_148f960c0` / `FUN_146cd0bea` → `thunk_FUN_148f6e780` (registry vector append) →
  managers keyed by name, each an 8-byte vtable stub.
- Creature bring-up: `FUN_14067ccd0` → `FUN_140f27f60("CCreatureRigidObject")` + `FUN_1402fa680` (×64) →
  `thunk_FUN_14a524650(DAT_142c84be8, …)` (register into update registry); teardown `FUN_14067d2e0`.
- Health tick: `FUN_140e60340` → component message bus (`this+0x20`) → `"EnterNearDeath"`/`"HandleDeath"`/
  `"EnterDeath"`/`"UpdateHealth"`/`"UpdateCharacterDmgIndicators"`.
- Damage: `CDamageController` `FUN_140809c00` → `HitAnalyze` `FUN_140809cc0` → `HitApply` `FUN_140809d80`
  (bodies OPEN) → mutate `CHealthControl` → read by `FUN_140e60340`. Global: `CDamageCoordinator`,
  `CAreaDamage` `FUN_140677850`, `OnOmniDamage` `FUN_140e53480`.
- Death → ragdoll (inferred hop): `"HandleDeath"` → death handler (OPEN) → `RagdollInstanceImplTryActivate`
  + physics `"OnDamage"`/`"RagdollForceToBone"` (`FUN_147759a20`) → `CRagdollObject` `FUN_140811ff0`.
- Player/NPC discriminators built by condition-RTTI builders `FUN_1485e07e0` / `FUN_1485b9730`.

## Open questions / lower-confidence

1. **Manager ticks (OPEN).** `PTR_LAB_141d908e8` (character), `PTR_LAB_141d90908` (player),
   `PTR_LAB_141d90e48` (creature) — pool/update/spawn methods behind data vtables. Resolve from `.rdata` at
   those addresses / x64dbg vtable breakpoint (PAUSED).
2. **Damage-apply logic (OPEN).** HitAnalyze/HitApply per-hit bodies behind `PTR_LAB_141d9db18` /
   `PTR_LAB_141d9daf0`; damage-type → hit-react mapping. Same resolve route.
3. **Player-vs-NPC test (inferred).** Whether the split is purely class-level or also a per-object flag —
   the `CIsPlayerCondition::Evaluate` body is OPEN.
4. **Faction matrix + NPC faction field (OPEN).** Contents of `settings/factions.bin`; which character struct
   offset holds the faction id (`FUN_140554d40` writes several defaults near `+0x27c8`, not proven to be it).
   Resolve with `jc4_adf` on the bin + x64dbg trace of the `change_character_faction` handler.
5. **Fire/health magnitudes (OPEN).** `DefaultFireDamageConstants` values, `DAT_141ca9c7c` health scale, max-HP
   defaults — all data-side (RTPC/ADF). Resolve via `[[rtpc-entity-assembly]]`.
6. **Wildlife per-species config (OPEN).** `SAiWorldSim_Animals` per-species/zone sub-lists behind
   `PTR_LAB_141ceba60`.

## Appendix — decomp anchors

Strings and their `FUN_`/`DAT_` (re-verify by grepping the string, then reading the enclosing `==== FUN_ ====`):

- Managers: `"CCharacterManager"` (3209490/4138972, vtable `PTR_LAB_141d908e8`), `"CPlayerManager"`
  (3209495/4138977, `PTR_LAB_141d90908`; name getter `FUN_14e69d02b` @6756015), `"CCreatureManager"`
  (3209705; pool init `FUN_14067ccd0` @640597; dtor `FUN_14067d2e0`), `"CPlayerSpawnPointManager"` (3209500),
  `"CPlayerReportingManager"` (3209560), `"CDamageCoordinator"` (3209755/4139237).
- Registration primitives: `thunk_FUN_148f6e780` @0x148f6e780, registrar `FUN_148f960c0` @0x148f960c0 /
  `FUN_146cd0bea` @0x146cd0bea, alloc `thunk_FUN_1496a12b0` @0x1496a12b0, factory `FUN_1402fa680` @0x1402fa680,
  class intern `FUN_140f27f60` @0x140f27f60, condition intern `thunk_FUN_14aadec10`, component builder
  `FUN_14085fd00`.
- Character class: `"CCharacter"` getter `FUN_14053f2a0` (`DAT_142cb3d30`); `"CCharacterRule"` (861497/1369527);
  `"CCharacterInfoDescription"` `FUN_1408455a0` (900512); `"CObjectiveCharacterController"` `FUN_140810f90`
  (867099); `"CPlayerGameplayConfiguration"` (867419); `"CPlayerSpawnPoint(Trigger)"` (867451/867483).
- Health/damage: `"CHealthControl"` `FUN_140b0a210`; `"CHealthbarComponent"` `FUN_1408e0760`;
  `"CDamageController"` `FUN_140809c00`/`PTR_LAB_141d9dac8`; `"CDamageControllerHitAnalyze"` `FUN_140809cc0`;
  `"CDamageControllerHitApply"` `FUN_140809d80`; `"CAreaDamage"` `FUN_140677850`; health tick `FUN_140e60340`
  (1676590+); `"InitHealth"` `FUN_140e539c0`; `"SetHealth"` `FUN_140de8800`; `"OnHealthbarHit"` `FUN_140e14900`;
  `"OnOmniDamage"` `FUN_140e53480`; `"OnPlayerDidDamage"` `FUN_14a8a0ca0`; `"DefaultFireDamageConstants"`
  `FUN_140d04e00` (1528000); `"PlayerDamageEffect"` `FUN_140ce7040`; physics `"OnDamage"`/ragdoll behaviors
  `FUN_147759a20`.
- Conditions: `"CIsPlayerCondition"` `FUN_14054a190` (504011); `"CIsPlayerNonCombat"` `FUN_14054a210` (504030);
  `"CPlayerIsAutoAimingCondition"` `FUN_14054b690`; `"CPlayerInKillVolumeChecker"` `FUN_140846320` (901106);
  `"CNpcIsRunningCondition"` `FUN_14054b110` (504600; name getter @7138017); `"CCharacterIsAliveCondition"`
  (501921); `"CHealthCondition"` `FUN_140549090` (503365); `"CLinkHealthCondition"` `FUN_14054a810`;
  `"CIsTakingDamageCondition"` `FUN_14054a410`; `"CVehicleDamageState"` `FUN_14054d510`. Builders `FUN_1485e07e0`
  / `FUN_1485b9730`.
- Ragdoll/skeleton: `"CRagdollObject"` `FUN_140811ff0`/`PTR_LAB_141d9b700`; `"CRagdollAttachment"`
  `FUN_14053f330`/`PTR_LAB_141d9bb60`; `"CRagdollBoneProxyObject"` `FUN_1408463b0`; bone map `FUN_14050f3a0`
  (479478, `"ragdoll_Hips"`…); bone comparator `FUN_1478ad7b0`; skeleton job `FUN_141b88110`
  (`"StAllocateAndDMASkeleton"` @2928983); collision layers `FUN_1400cf270` (`"CHARACTER_RAGDOLL"` @38303);
  ragdoll params `animations/ragdoll/ragdoll_params.brdd` (509145); `RagdollInstanceImplTryActivate` (@38608).
- Faction: `"CConditional_FactionStatus"` `FUN_14065ee60`/registrar `FUN_148f99a90`/`PTR_LAB_141d92338`;
  `"CObjectTrackerFilterFaction"` `FUN_1408100b0`/`FUN_14090df80`/`PTR_LAB_141d9d2d0`;
  `"change_character_faction"` `FUN_140554d40` (508620); `settings/factions.bin` (`FUN_140302dc0` @913152,
  init `FUN_14085a3a0`); UI `"SetFaction"` `FUN_140e114e0`.
- Creatures/spawn: `"CCreatureRigidObject"` (`FUN_140f27f60`, in `FUN_14067ccd0`); `"SAiWorldSim_Animals"`
  `FUN_1404674b0` (401796); `"Characters_Animals"` (3321580); `"CBirdSpawner"` `FUN_140794d60`;
  `"CAiWorldSimSpawnPoint"` `FUN_14041a280`; `"CSpawnSystem"` (3209190, hash 0x41d8de38, `PTR_LAB_141d90548`);
  `"CObjectSourceSpawners"`, `"CDespawnVolume"`, `"CEncounterDespawnSettings"`, `"CAmbientEffects"`
  (reflected singletons via `FUN_140f27f60`).
- Ragdoll/death `ACT_*`: `ACT_HITREACT_RAGDOLL` (3082893), `ACT_RAGDOLL_IMPACT` (3086107),
  `ACT_RAGDOLL_INAIR_STABILIZATION_FWD/BWD` (3086120/3086143), plus `ACT_FULL_RAGDOLL`, `ACT_GRAPPLE_RAGDOLL`,
  `ACT_FORCE_DEATH_RAGDOLL`, `ACT_HITREACT_*` (registered via `FUN_140f27f60`).

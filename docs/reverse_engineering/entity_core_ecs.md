# Entity core / ECS & the reflection registry — the object model everything is built on

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

This is the foundational pillar. `behavior_system.md` proved the *shared spine* — one lookup3-keyed
name→type registry (`FUN_140f27f60`, 6,138 call sites) plus the family registrars. This doc goes deeper
on the **entity / game-object object model itself**: what an object is, how classes are reflected and
registered, how components attach, what the transform component bridges, and how data (RTPC `.epe`)
instantiates a live object graph through the registry.

## Overview

JC4 (Avalanche **Apex** engine) does **not** use a dense array-of-structs ECS. It uses a **reflected
object-composition model**:

1. A single global **reflection registry** — an open-addressed hash map keyed by the cracked
   **lookup3 `hashlittle`** name hash — maps every gameplay/engine class name to a **prototype**
   (a heap object that is just a **vtable pointer** plus a few bytes). (proven — `FUN_14cfef490`,
   `FUN_14cfefcc0`, `FUN_140f27f60`.)
2. **Three registrars** populate it at startup: the **component/object** registrar `FUN_14085fd00`
   (~404 insert sites, DECOMP FAIL body but its per-class helpers decompiled), the **manager/system**
   registrar `FUN_148f960c0` (92 singleton managers), and the **conditional/objective** registrar
   `FUN_148f99a90` (from `behavior_system.md`). A fourth table registers animation-graph conditions
   (`FUN_1485b9730`). (proven.)
3. A **game object** (`CGameObject` / `CEntityObject`) is a heap object whose vtable carries its
   behaviour; it aggregates a **small set of core components** (`CTransformComponent`,
   `CInspectableComponent`, model/health/mounted) rather than an open component array. References/handles
   are wrapped by **`CGameObjectReferenceObject`**; collections by **`CGameObjectList`**; the manager
   **`CGameObjectManager`** owns lifetime. (proven that these classes exist and are reflected; the
   internal handle/pool byte layout is behind data-section vtables — inferred.)
4. At load, an RTPC `.epe` blob (memory `[[rtpc-entity-assembly]]`, `[[composite-assets]]`) names each
   class by a 32-bit hash; the loader hashes/looks up the name in the registry, gets the prototype, and
   clones/placement-constructs a typed instance whose vtable does the `Deserialize`. The **same lookup3
   hash** identifies the class in the `.epe` and in the code (`docs/formats/name_hash.md`). (proven that
   identity is shared; the exact deserialize call is behind the vtable — inferred.)

## Key classes & functions

| Name string / role | FUN_ | One-line role |
|---|---|---|
| **lookup3 name hash** (`hashlittle`) | `FUN_140f27f60` → `FUN_14aadeee0` | Hashes `(name,len)` → 32-bit key; the shared identity for classes, components, actions, asset paths. (proven) |
| **Reflection registry — insert** | `FUN_14cfef490` (`thunk_FUN_14cfef490`) | Open-hash insert `key=hash(name) → prototype*`; bucket `key % bucketCount`, 0x10-byte chained slots. (proven) |
| Registry — low-level bucket add | `thunk_FUN_14768f3e0` | Appends a new `{key,next,value}` slot when the key is absent. (proven) |
| **Reflection registry — container ctor** | `FUN_14cfefcc0` | Builds the map object: vtable `PTR_LAB_14241aa48`, slot-size `0x10`, **`0x2000` buckets**. (proven) |
| **Component/object registrar** | `FUN_14085fd00` (size 17436, **DECOMP FAIL**) | Registers ~404 object/component prototypes; body walled, but its ~340 per-class helpers decompiled. (proven skeleton) |
| Base-object sub-registrar | `FUN_1402b5080` (size 2144) | Clean example of the register loop: alloc 0x10 prototype, install vtable, insert by cached hash. (proven) |
| Per-component register helper (example) | `FUN_14080d250` → registers `CInspectableComponent` | alloc `0x10`, vtable `PTR_LAB_141d9d118`, `thunk_FUN_14cfef490(map, hash, proto)`. (proven) |
| **Manager/system registrar** | `FUN_148f960c0` (size 4805) | Registers **92** singleton managers as `{name,prototype}` records (incl. `CGameObjectManager`). (proven) |
| Manager list append | `FUN_148f6e780` (`thunk_FUN_148f6e780`) | `push_back {const char* name, void* singletonPrototype}` into the manager vector. (proven) |
| Conditional/objective registrar | `FUN_148f99a90` (size 7930) | 184 `CConditional_*`/`CObjective*` prototypes → name-hash map (see `behavior_system.md`). (proven) |
| Graph-condition registrar | `FUN_1485b9730` | 198 `C*Condition` `{ctor,size}` descriptors (animation graph). (proven) |
| `CTransformComponent` type-hash getter | `FUN_1407dc170` → `DAT_142cb8ee0` | The spatial-backbone component; bridges physics↔render↔entity (see `physics_constraints.md`). (proven) |
| `CInspectableComponent` register + getter | `FUN_14080d250` / `FUN_140f0f...` → `DAT_142cb98ec` | The reflection/property surface component (editor/debug inspect). (proven) |
| `CGameObjectReferenceObject` type-hash getter | `FUN_1402abe20` → `_DAT_142cb0b30` | The **reference/handle wrapper** class. 2nd caller in the 0x147c9 construct dispatcher. (proven) |
| `CGameObject` type-hash getter | `FUN_14027bd70` → `_DAT_142cafa58` | The base game-object class. (proven) |
| `CEntityObject` / `CGameObjectList` getters | `FUN_140795810` / `FUN_1407daaf0` | The entity object + object-list (collection/pool) classes. (proven) |

## How it works (from the decomp)

### 1. The reflection registry is one open-hash map keyed by lookup3

`FUN_14cfefcc0` constructs the registry container: it installs vtable `PTR_LAB_14241aa48`, sets the
per-slot size to `0x10`, and preallocates **`0x2000` (8192) buckets** (`param_1[9] = param_1[10] =
0x2000`), with the free-list head at `0xffffffff…`. (proven.)

`FUN_14cfef490` is the insert/lookup-or-add. Given the map and a 32-bit `key` (the name hash) it does a
textbook compact open-hash:

```c
bucket = *(u16*)(bucketArray + (key % bucketCount) * 2);   // param_1[2] = bucketCount (u16)
for (slot = bucket; slot != 0xffff; slot = slotArray[slot].next)   // param_1[1] = slotArray, 0x10-byte slots
    if (slotArray[slot].key == key) return;                // already present
// absent → append {key, value=prototype} and relink the bucket
thunk_FUN_14768f3e0(param_1, &key, &prototype);
```

So the registry is `HashMap<lookup3(className) → prototype*>` with 0x10-byte slots
`{u32 key, u16 next, …, u64 value}`. This is the *same* structure the conditional registry uses
(`FUN_147cafcd0` in `behavior_system.md`) — the engine has one reflection-map shape reused per family.
(proven.)

### 2. Registering a class = prototype + vtable + cached hash

`FUN_1402b5080` (a clean, fully-decompiled sub-registrar called by the walled `FUN_14085fd00`) shows the
exact idiom, repeated per class:

```c
map   = *(void**)(param_1 + 8);              // the reflection registry
proto = alloc(0x10);                         // 16-byte prototype object
*proto = &PTR_LAB_141cf4078;                 // install THIS class's vtable
hashp = FUN_1402ab760();                     // magic-static: cached hash of the class name literal
thunk_FUN_14cfef490(map, *hashp, proto);     // register: hash -> prototype
```

Each class's name hash is produced once by a **magic-static getter** (`_Init_thread_header/_footer`
guard) that calls `FUN_140f27f60("CClassName", len)` and caches the 32-bit result in a `DAT_` global —
e.g. `FUN_1407dc170` caches `hash("CTransformComponent")` (len `0x13`) in `DAT_142cb8ee0`, `FUN_14027bd70`
caches `hash("CGameObject")` (len `0xb`) in `_DAT_142cafa58`. The `len` argument is exactly the string's
`strlen`, confirming these are **name-string hashes**, not opaque IDs. (proven.)

**Prototype sizes encode the family:**
- **Managers**: `alloc(8)` — vtable-pointer only (a singleton has no per-instance reflected state).
- **Components/objects**: `alloc(0x10)` — vtable + 8 bytes (the reflected object base).
- **Conditionals**: `alloc(8)` (from `behavior_system.md`).

(proven — the immediate operands to `thunk_FUN_1496a12b0`.)

### 3. Three registrars install the world

- **Component/object registrar `FUN_14085fd00`** (size 17436) is **DECOMP FAIL** — the body is walled
  open (the methodology's "registration skeleton visible, body behind data-section vtables" case). But
  **340 helper functions carry `callers=[…(FUN_14085fd00)]`**, and each decompiled helper is the
  register idiom above (`FUN_14080d250` for `CInspectableComponent`, `FUN_1402b5080` for the base-object
  batch, …). There are **404 component-map insert sites** (`FUN_14cfef490`) across the binary. So the
  component/object family is ~400 reflected classes. (proven count; individual bodies inferred from the
  helpers.)
- **Manager/system registrar `FUN_148f960c0`** (size 4805) registers **92 singleton managers**. Each is:
  `proto = alloc(8); *proto = &PTR_LAB_141d90…; append {&"CManagerName", proto}` — the append is either
  the inline vector push (`*cur = &name; cur[1]=1; cur[2..3]=proto`) or `thunk_FUN_148f6e780(tbl, name,
  proto)`. The manager table is a **`vector<{const char* name, void* singletonPrototype}>`** (append,
  not hash-map). Registered names include `CGameObjectManager`, `CGameWorld`, `CConstraintFactory`,
  `CConditionalManager`, `CActionTokenManager`, `CSpawnSystem`, `CTargetSystem`, `CCharacterManager`,
  `CPlayerManager`, plus every gameplay system (full list in Appendix B). (proven.)
- **Conditional/objective registrar `FUN_148f99a90`** and **graph-condition registrar `FUN_1485b9730`**
  are documented in `behavior_system.md`; they register the predicate/objective and animation-transition
  families into their own name-hash maps. (proven, cross-ref.)

Note the `0x41cefc18`-style immediates in `FUN_148f960c0` are **low-32 of the name-string pointer**
(the binary is `0x140000000`-based; `0x141cefc18` → `&"CGameObjectManager"`), *not* hashes — the manager
table stores the literal name pointer, not a precomputed hash. (proven — reconciles with
`FUN_148f6e780`'s `{name, prototype}` record.)

### 4. What a game object / entity is

The reflected **object family** (all registered into the component/object map) is the entity core:

| Class | Getter FUN_ | Role (proven it's reflected; role from name — inferred) |
|---|---|---|
| `CGameObject` | `FUN_14027bd70` → `_DAT_142cafa58` | Base game object (vtable-driven; the ECS node). |
| `CEntityObject` | `FUN_140795810` → `DAT_142cb89ec` | The entity object (the RTPC `.epe` root maps here). |
| `CComponent` | `FUN_1402aab20` → `DAT_142cb0850` | Component base class. |
| `CGameObjectReferenceObject` | `FUN_1402abe20` → `_DAT_142cb0b30` | **Reference/handle wrapper** — the "safe pointer" to an object. |
| `CGameObjectList` | `FUN_1407daaf0` → `DAT_142cb8cf0` | **Collection / pool** of game objects. |
| `CGameObjectExtension` | (getter → `DAT_142cb8bb8`) | Object extension (attached sub-object). |
| `CEntitySpline{,Point,Segment,Occupant}` | `FUN_140795810` region | Spline entity family (paths). |

There is **no** flat "component array of hundreds of types" — instead each `CGameObject` aggregates a
**small, fixed set of core components** whose type hashes are the ones with dedicated getters:
`CTransformComponent` (`DAT_142cb8ee0`), `CInspectableComponent` (`DAT_142cb98ec`), plus
`CHealthbarComponent`, `CMountedComponent`, and the weapon-component set (`CScopeWeaponComponent`,
`CTurretWeaponComponent`, …, 31 `*Component` names total). A component is attached by looking up its type
hash in the registry and constructing/aggregating it onto the object. (proven that these components are
reflected and hash-keyed; the attach-list byte layout on `CGameObject` is behind its vtable — inferred.)

### 5. The transform component — the spatial backbone

`CTransformComponent` (`FUN_1407dc170` → `DAT_142cb8ee0`, name len `0x13`) is the bridge component: it
holds the object's world transform and is the meeting point of **physics ↔ render ↔ entity**.
`physics_constraints.md` already proved it is the write-back target that copies a Havok body's motion
into the entity world transform each step, and that constraints/render read it. This ECS doc's
contribution is the *registration* side: the transform component is one reflected component in the same
map as everything else, keyed by `hash("CTransformComponent")`, so physics, render and entity code all
resolve it through the **one** registry rather than a hard-coded slot. (proven registration;
the per-step motion copy is behind the vtable — see `physics_constraints.md`, inferred there.)

### 6. Data → live object: the construct/deserialize path

The per-class type-hash getters have **two** callers: their registrar (register-time) **and** a function
in the **`0x147c9xxxx`** region (construct-time) — e.g. `FUN_1402abe20` (`CGameObjectReferenceObject`) is
called from both `FUN_1402b5080` and `0x147c908c0`; the same double-caller pattern holds for ~15 core
object getters. That `0x147c9…` dispatcher is the **object construction / deserialization** path: given a
class hash from data it resolves the type and builds the instance. Its body sits inside a large walled
(non-exported / DECOMP-FAIL-adjacent) function, so the exact `hash → prototype → clone → Deserialize`
sequence is **inferred**, but the call structure (type getters feeding a builder in the 0x147c9 region)
is **proven**. This is the code-side counterpart to memory `[[rtpc-entity-assembly]]`: the `.epe`
RTPC v3 blob declares classes by hash (class `0xc1d8333a` = a render part, etc.), and this dispatcher
turns those hashes into live components via the registry. (proven call structure; deserialize body
inferred.)

## Data & config integration

- **RTPC `.epe` = the entity blueprint.** An entity `.epe` (RTPC v3) is a tree of typed nodes; each node
  names its class by the **same lookup3 32-bit hash** the code caches for that class name. Loading =
  hash-lookup in the reflection registry → prototype → typed construct. So a `CTransformComponent` node
  in the `.epe` and `hash("CTransformComponent")` in the code are the **same key**. (proven identity;
  round-trip against a specific `.epe` node is next-pass — inferred.)
- **ADF supplies the typed payload.** Component/manager configuration values ride in ADF
  (`docs/formats/adf.md`); the reflected class provides the schema, ADF provides the bytes. (inferred —
  same two-layer pattern seen across all system docs: mechanism in code, magnitudes in data.)
- **`CInspectableComponent` is the editor/reflection surface.** Its presence as a first-class reflected
  component (own vtable `PTR_LAB_141d9d118`, own hash getter) is the hook the tools/editor used to
  enumerate an object's properties — the runtime face of the same reflection the `.epe` serializes.
  (proven it's a reflected component; the property-enumeration API is behind its vtable — inferred.)
- **Managers own the world.** `CGameObjectManager` (create/destroy/iterate), `CGameWorld`,
  `CSpawnSystem`, `CConstraintFactory` are singletons in the manager table; gameplay code resolves them
  by name through the manager registry. Their create/destroy/pool bodies are behind the manager
  prototype vtables (`PTR_LAB_141d90648` for `CGameObjectManager`) and are **not** in the text dump.
  (proven the managers exist + are name-registered; lifetime bodies inferred/walled.)

## Notable constants / tunables

- Reflection registry: **`0x2000` (8192) buckets**, **`0x10`-byte slots**, sentinel `0xffff` (empty
  bucket) — `FUN_14cfefcc0`. (proven.)
- Prototype allocation sizes: **8** (managers, conditionals), **`0x10`** (components/objects) — the
  `thunk_FUN_1496a12b0(...)` immediates. (proven.)
- lookup3 seed `0x21524111`, 12-byte block stride — the hash used for every class name
  (`FUN_14aadeee0`, from `behavior_system.md` / `docs/formats/name_hash.md`). (proven.)
- Registry population: **92** managers (`FUN_148f960c0`), **~404** component/object classes
  (`FUN_14cfef490` insert-site count; ~340 helper fns under `FUN_14085fd00`), **184** conditionals/objectives,
  **198** graph conditions. (proven counts.)
- Class-name-string literals fed to `FUN_140f27f60` across the binary: **2,287 distinct**; total call
  sites **6,138** (from `behavior_system.md`). (proven.)

## Call-graph highlights

- `FUN_14085fd00` (component registrar, walled) → **340** per-class helpers (`FUN_14080d250`,
  `FUN_1402b5080`, …) → each: `thunk_FUN_1496a12b0(0x10)` (alloc) + class hash getter +
  `thunk_FUN_14cfef490` (insert). (proven.)
- `FUN_14cfef490` (insert) → `thunk_FUN_14768f3e0` (bucket add). Container built by `FUN_14cfefcc0`.
  (proven.)
- `FUN_148f960c0` (manager registrar) → `thunk_FUN_1496a12b0(8)` + `thunk_FUN_148f6e780`/inline push ×92.
  (proven.)
- Core object type-hash getters (`FUN_14027bd70` `CGameObject`, `FUN_1402abe20`
  `CGameObjectReferenceObject`, `FUN_140795810` `CEntityObject`, …) each have callers
  `= {registrar, 0x147c9… construct dispatcher}`. (proven.)
- `FUN_1407dc170` (`CTransformComponent` hash) is consumed by the physics transform-bridge path
  (`physics_constraints.md`). (proven cross-ref.)

## Open questions / lower-confidence

- **`CGameObjectManager` create/destroy/pool bodies** are behind the manager prototype vtable
  (`PTR_LAB_141d90648`) — not in the functions-only export. The handle model (generation counter? free
  list? slot index?) is therefore **inferred**: `CGameObjectReferenceObject` is clearly the handle
  wrapper and `CGameObjectList` the collection, but their byte layout needs a live x64dbg vtable read.
  (inferred.)
- **The 0x147c9… construct/deserialize dispatcher** body is walled; the `hash → prototype → clone →
  Deserialize(context)` sequence is inferred from the double-caller structure, not read. Confirming the
  `Deserialize` vtable slot needs a live read. (inferred.)
- **Component attach list on `CGameObject`** — whether components live in a fixed struct, an inline
  small-vector, or the reflection map per-instance — is not visible (vtable-walled). The dedicated hash
  getters for Transform/Inspectable/Health/Mounted suggest a small fixed core set, but this is
  inferred. (inferred.)
- **`.epe` round-trip:** identity (shared lookup3 hash) is proven, but no specific `.epe` node has been
  hashed and matched to a registry key yet — the concrete data→registry link is next-pass. (inferred.)

## Appendix A — decomp anchors (re-verifiable)

| Anchor | FUN_ / DAT_ |
|---|---|
| lookup3 name hash (thunk / core) | `FUN_140f27f60` → `FUN_14aadeee0` |
| Reflection registry insert / bucket-add / ctor | `FUN_14cfef490` / `thunk_FUN_14768f3e0` / `FUN_14cfefcc0` (vtable `PTR_LAB_14241aa48`) |
| Component/object registrar (walled) + clean sub-registrar | `FUN_14085fd00` (DECOMP FAIL) / `FUN_1402b5080` |
| Example component register helper | `FUN_14080d250` (`CInspectableComponent`, vtable `PTR_LAB_141d9d118`) |
| Manager registrar + list append | `FUN_148f960c0` / `FUN_148f6e780` |
| Conditional / graph-condition registrars | `FUN_148f99a90` / `FUN_1485b9730` (see `behavior_system.md`) |
| Prototype allocator | `thunk_FUN_1496a12b0` (8 = mgr/cond, 0x10 = component/object) |
| `CGameObject` hash | `FUN_14027bd70` → `_DAT_142cafa58` |
| `CEntityObject` hash | `FUN_140795810` → `DAT_142cb89ec` |
| `CComponent` hash | `FUN_1402aab20` → `DAT_142cb0850` |
| `CGameObjectReferenceObject` hash (handle) | `FUN_1402abe20` → `_DAT_142cb0b30` |
| `CGameObjectList` hash (collection/pool) | `FUN_1407daaf0` → `DAT_142cb8cf0` |
| `CTransformComponent` hash | `FUN_1407dc170` → `DAT_142cb8ee0` (len 0x13) |
| `CInspectableComponent` hash | `DAT_142cb98ec` (getter at ~`0x140f0f…`) |
| `CGameObjectManager` prototype vtable | `PTR_LAB_141d90648` (in `FUN_148f960c0`) |
| Construct/deserialize dispatcher (walled) | `0x147c9xxxx` (2nd caller of core object getters) |

## Appendix B — vocabulary (proven from the decomp)

### B.1 — Singleton managers (92, registered in `FUN_148f960c0`)

`CSoundSystem`, `CProfileManager`, `CVideoManager`, `CUIManager`, `CModelInstanceManager`,
`CLandscapeManager`, `CEffectSystem`, `CProjectEffectToTerrain`, `CSpawnSystem`, `CAiSystem`,
`CHavokDestructionInstancedGraphicsManager`, `CTargetSystem`, `CRiverManager`, `CWindTunnelManager`,
`CTopographicalWind`, `CCoverageManager`, `CRoadManager`, **`CGameObjectManager`**, `CGameWorld`,
`CConstraintFactory`, `CConditionalManager`, `CCutsceneManager`, `CActivityManager`, `CObjectiveManager`,
`CObjectiveContentManager`, `CTacticalNodeManager`, `CRewardSystem`, `CMediaRevolutionManager`,
`CRetoolerManager`, `CNewGamePlusManager`, `CFrontlineManager`, `CHeatManager`, `CMissionManager`,
`CQuestManager`, `CDialogueCoordinator`, `CLightningManager`, `CWorldTime`, `CCharacterManager`,
`CPlayerManager`, `CPlayerSpawnPointManager`, `CAmmunitionManager`,
`CWeaponPlacementRulePersistentManager`, `CDialogueManager`, `CRadioSystem`, `COperationManager`,
`COnlineSuiteManager`, `CDownloadableContentManager`, `CFriendManager`, `CStatisticManager`,
`CNotificationManager`, `CChallengeManager`, `CPlayerReportingManager`, `CVideoRecordingManager`,
`CAchievementsManager`, `CSquareEnixMembership`, `COnlineFeatureManager`, `CFpsCounter`, `CStashManager`,
`CSkinManager`, `CButtonHintManager`, `CVocalsManager`, `CCollectionManager`, `CSupplyManager`,
`CSupplyFactoryManager`, `CEncounterManager`, `CWeaponManager`, `CUIInputManager`,
`CGameplayEventManager`, `CTutorialManager`, `CTimestampManager`, `CBinaryStateObjectManager`,
`CObjectiveDebugTextManager`, `CSupplyDropManager`, `CSupplyRewardManager`, `CDiscoveryManager`,
`CBiomeManager`, `CLeaderboardManager`, `CPilotDataManager`, `CVehicleDataManager`, `CCreatureManager`,
`CTurnTaker`, **`CActionTokenManager`**, `COnRoadService`, `CEnvironmentGraphicsModifierManager`,
`ChromaManager`, `CPresenceManager`, `CDaredevilPointManager`, `CDemonAreaManager`, `CAgencyBaseManager`,
`CDamageCoordinator`, `CVegetationInteraction`, `CRicoVocals`. (proven — string list from
`FUN_148f960c0`.)

### B.2 — Entity / game-object core object family (reflected into the component map)

`CGameObject`, `CEntityObject`, `CComponent`, `CGameObjectReferenceObject` (handle),
`CGameObjectList` (collection/pool), `CGameObjectExtension`,
`CEntitySpline{,Point,Segment,Occupant}`, plus the object-source / object-tracker / objective object
graph (`CObjectSource*`, `CObjectTracker*` + ~28 `CObjectTrackerFilter*`, `CObjective*`,
`CObjectiveGoal`, `CObjectiveParam`, …). (proven — `FUN_140f27f60("C…")` string set.)

### B.3 — Core components (31 `*Component` classes)

Spatial/reflection core: `CTransformComponent`, `CInspectableComponent`, `CComponent` (base),
`CHealthbarComponent`, `CMountedComponent`. Weapon-component set (attached to weapon objects):
`CAimPOIWeaponComponent`, `CAirplaneFlybyWeaponComponent`, `CAmmoRegenerationWeaponComponent`,
`CBarrelRecoilWeaponComponent`, `CBarrelSpinWeaponComponent`, `CCameraFollowWeaponComponent`,
`CCameraSwapWeaponComponent`, `CContentSwapperWeaponComponent`, `CCylinderWeaponComponent`,
`CDoActWeaponComponent`, `CFOWControllerWeaponComponent`, `CHitReactionCastWeaponComponent`,
`CLaserBeamWeaponComponent`, `CLightWeaponComponent`, `CLightningBeamWeaponComponent`,
`CLockonWeaponComponent`, `CMagnetWeaponComponent`, `CModelAttachementWeaponComponent`,
`CPhysicalizationWeaponComponent`, `CScopeWeaponComponent`, `CSpotlightWeaponComponent`,
`CTriggerBarkWeaponComponent`, `CTurretWeaponComponent`, `CVehiclePartRemoverWeaponComponent`,
`CWallPenetrationWeaponComponent`. (proven — string set.)

*(Counts and string lists are proven from the decomp. Prototype byte layouts, the manager/game-object
lifetime bodies, the component attach list, and the `.epe`→registry construct sequence are behind
data-section vtables / walled functions and are graded inferred where stated.)*

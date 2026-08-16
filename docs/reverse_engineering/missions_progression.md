# Missions, Activities, Supply, Progression & DLC — the content/progression layer

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

All FUN_/DAT_ addresses are from `output/_ghidra_jc4/jc4_all_functions_decomp.txt` (Ghidra decomp of
`JustCause4.exe`, 162,115 functions). "proven" = read directly from that decomp text.

## Overview

Just Cause 4's mission/progression layer is a set of **named singleton managers** (quests, objectives,
activities, supply drops, collections, statistics, DLC, notifications) that are all registered into one
global service registry at game-world bootstrap. Content is **data-driven and instantiated by class name**:
every gameplay type — objective goals, conditionals, supply items — is looked up through a single global
**type-registry factory** (`FUN_140f27f60`, called from 6,138 sites) that maps a `(name, length)` pair to a
cached type descriptor. RTPC/ADF asset data references those names to build objectives, quests, and the
**condition trees** that gate content. Progression *state* (missions completed, DLC unlocked, nodes secured,
challenges done) is surfaced to the rest of the game almost entirely through the **`CConditional_Is*`**
condition family, which the behavior/condition VM (see `behavior_system.md`) evaluates. Runtime progression
counters (quests completed, supply drops collected, collectibles found…) are serialized into an **OSDK JSON
telemetry/stats blob** (`FUN_1409ff030`). (proven — manager registry, factory, condition inventory, and stats
serializer all read directly.)

## Key classes & functions

| Class / string | FUN_ (registration / implementation) | Role |
|---|---|---|
| `CQuestManager` | registered in `FUN_148f960c0` (vtable `PTR_LAB_141d90848`) | Top-level quest/storyline tracking |
| `CMissionManager` | `FUN_148f960c0` (`PTR_LAB_141d90828`) | Mission instances |
| `CObjectiveManager` | `FUN_148f960c0` (`PTR_LAB_141d90708`) | Live objective tracking |
| `CObjectiveContentManager` | `FUN_148f960c0` (`PTR_LAB_141d90728`); typename `FUN_149247310` | Objective UI/content payloads |
| `CActivityManager` | `FUN_148f960c0` (`PTR_LAB_141d906e8`); typename `FUN_148eb7e00` | Side-activities registry |
| `COperationManager` | `FUN_148f960c0` (`PTR_LAB_141d909a8`) | "Operation" (replayable) missions |
| `CRewardSystem` | `FUN_148f960c0` (`PTR_LAB_141d90768`) | Grants rewards on completion |
| `CSupplyManager` | `FUN_148f960c0` (`PTR_LAB_141d90be8`); typename `FUN_149b16e00` | Supply-drop / loadout backbone |
| `CSupplyFactoryManager` | `FUN_148f960c0` (`PTR_LAB_141d90c08`) | Instantiates supply items |
| `CSupplyDropManager` (tick) | `FUN_140bec1c0`, per-crate `FUN_149b478f0` | Supply-crate state machine |
| `CCollectionManager` | `FUN_148f960c0` (`PTR_LAB_141d90bc8`) | Collectibles tracking |
| `CStatisticManager` | `FUN_148f960c0` (`PTR_LAB_141d90a28` **and** `…c28`); telemetry `FUN_1409ff030` | Progression counters / analytics |
| `CNotificationManager` | `FUN_148f960c0` (`PTR_LAB_141d90a48`); typename `FUN_147fc30ec` | Completion pop-ups |
| `CChallengeManager` | `FUN_148f960c0` (`PTR_LAB_141d90a68`) | Daredevil/monthly challenges |
| `CDownloadableContentManager` | `FUN_148f960c0` (`PTR_LAB_141d909e8`) | DLC ownership / gating |
| `CNewGamePlusManager` | `FUN_148f960c0` (`PTR_LAB_141d907c8`) | NG+ progression |
| Global type-registry factory | `FUN_140f27f60(name, len)` (size=5 thunk, 6138 callers) | Name → cached type descriptor |
| `ShowDLCStoreDialogAsync` dispatch | `FUN_1409b8480` → `FUN_1409a58e0` | Opens DLC store UI |
| Named-event broadcast | `thunk_FUN_148fdf5b0(name)` (29 sites) | Fires string-named gameplay events |

## How it works (from the decomp)

### 1. Manager registry bootstrap — `FUN_148f960c0`  (proven)

One large function (`size=4805`) constructs and registers **every** engine/gameplay manager into a service
container (`param_1`). Two registration idioms appear:

* Early managers use an inline pattern: allocate an 8-byte holder (`thunk_FUN_1496a12b0(8)`), store a vtable
  pointer (`PTR_LAB_141d90…`), and push a `(hash_id, 1, holder)` record into the array at `param_1+0x48`
  (e.g. `CSoundSystem` id `0x41cc6ff0`, `CModelInstanceManager` id `0x41cf8298`).
* Later managers (all the progression ones) use `thunk_FUN_148f6e780(param_1, "<ClassName>", holder)`.

The progression-relevant registrations, in file order (each with its vtable `PTR_LAB_`):
`CCutsceneManager`, **`CActivityManager`** (`…6e8`), **`CObjectiveManager`** (`…708`),
**`CObjectiveContentManager`** (`…728`), `CTacticalNodeManager` (`…748`), **`CRewardSystem`** (`…768`),
`CMediaRevolutionManager`, `CRetoolerManager`, **`CNewGamePlusManager`** (`…7c8`), `CFrontlineManager`,
`CHeatManager`, **`CMissionManager`** (`…828`), **`CQuestManager`** (`…848`), `CDialogueCoordinator`, …,
**`COperationManager`** (`…9a8`), `COnlineSuiteManager`, **`CDownloadableContentManager`** (`…9e8`),
`CFriendManager`, **`CStatisticManager`** (`…a28`), **`CNotificationManager`** (`…a48`),
**`CChallengeManager`** (`…a68`), `CPlayerReportingManager`, `CVideoRecordingManager`,
`CAchievementsManager`, `CSquareEnixMembership`, `COnlineFeatureManager`, `CStashManager`, `CSkinManager`,
`CVocalsManager`, **`CCollectionManager`** (`…bc8`), **`CSupplyManager`** (`…be8`),
**`CSupplyFactoryManager`** (`…c08`), **`CStatisticManager`** again (`…c28`), `CEncounterManager`,
`CWeaponManager`, `CGameplayEventManager`, `CTutorialManager`, `CTimestampManager`, …
(`CStatisticManager` is registered twice — proven; likely two distinct stat scopes, e.g. session vs.
persistent — the second-scope split is *inferred*.) A near-identical sibling registrar exists around
line 3209415–3209625 using the same `thunk_FUN_148f6e780` list. (proven)

### 2. Everything is instantiated by name — the type-registry factory `FUN_140f27f60`  (proven)

`FUN_140f27f60(const char *name, size_t len, …)` is a 5-byte jump thunk called from **6,138** sites. Each
call site is a thread-safe lazy singleton that caches the returned type descriptor in a `DAT_` slot:

```c
// FUN_14090e9a0 — the descriptor for the "CObjective" type
if (tls_epoch < DAT_142cb9254) {
  _Init_thread_header(&DAT_142cb9254);
  if (DAT_142cb9254 == -1) {
    DAT_142cb9250 = FUN_140f27f60("CObjective", 10, 0, ...);   // name + strlen
    _Init_thread_footer(&DAT_142cb9254);
  }
}
return &DAT_142cb9250;
```

The length argument is always the literal `strlen`: `"CObjective"`=10 (`0xa`), `"CObjectiveDebugText"`=0x13,
`"CConditional_IsDLCUnlocked"`=0x1a, `"CConditional_IsMissionCompleted"`=0x1f. This factory is the bridge
that lets serialized RTPC/ADF asset data name a class and get back a live type descriptor to construct it.
(proven)

The **objective-goal** taxonomy is registered this way — each is a selectable "how does this objective
complete" rule: `CObjectiveGoal`, `CObjectiveGoal_Area`, `CObjectiveGoal_Counter`,
`CObjectiveGoal_CategoryCounter`, `CObjectiveGoal_Distance`, `CObjectiveGoal_TimeLimit`,
`CObjectiveGoal_StayNearObject`, `CObjectiveGoal_Conditional`. Objective **presentation** params:
`CObjectiveParam_ProgressBar`, `_HealthBar`, `_TimerPopup`, `_TornadoWidget`, `_DaredevilScoreAndTimer`,
`_BundleBlips`, `_AgencyRingCourse`, `_StayInAreaHack`. Objective **time** modifiers: `CObjectiveTime`,
`CObjectiveTimeCap`, `CObjectiveTimeSet`. (proven — all read as factory-registered class-name strings.)

### 3. Progression → condition VM — the `CConditional_Is*` family  (proven)

Progression state is exposed to gameplay logic (spawns, UI, mission flow) as **conditionals** the behavior VM
evaluates. Two are called out specifically:

* `CConditional_IsMissionCompleted` — descriptor cached in `DAT_142cb5c78` (getter `FUN_140651870`) and
  `DAT_142cb94cc` (getter at `FUN_140e53520`); created via `FUN_140f27f60("CConditional_IsMissionCompleted",0x1f)`.
* `CConditional_IsDLCUnlocked` — descriptor cached in `DAT_142cb5ae8` (getter `FUN_14063df00`) and
  `DAT_142cb94bc` (used inside game-mode init `FUN_140bdf5d0`); created via
  `FUN_140f27f60("CConditional_IsDLCUnlocked",0x1a)`. `FUN_140bdf5d0` builds a condition and calls the
  conditional-manager evaluate (`(**(code**)(*plVar9+0x38))(plVar9,0)`) to OR a DLC gate into a mode flag at
  `param_1+0x254`. (proven — construction/caching; the *evaluate reads DLC-manager ownership* step is inferred.)

The full progression-facing condition inventory (all proven as class-name strings):

* **Missions/quests/objectives:** `IsMissionActive`, `IsMissionCompleted`, `IsQuestActive`, `IsQuestAvailable`,
  `IsQuestCompleted`, `IsObjectiveActive`, `IsObjectiveCompleted`, `IsObjectiveFailed`, `IsObjectiveHidden`,
  `IsObjectiveTracked`, `IsObjectiveCurrentInSequence`, `IsObjectiveContentAvailable`,
  `IsAnyHardFailObjectiveActive`, `IsAnythingTracked`, `IsObjectTracked`.
* **Frontline / province-takeover ("nodes"):** `IsNodeCleared`/`Secured`/`Discovered`/`Tracked`/`Unsafe`
  (+ `Neighbor{Cleared,Secured,Unsafe}`), `IsInClearedNode`, `IsInSecuredTerritory`,
  `IsLocationComplete`/`Discovered`/`InSecuredTerritory`, `IsPlayerInsideNode`, `IsNearActiveFrontline`,
  `IsPOILocked`, `IsHoveringOverDiscoveryLocation`.
* **DLC / unlocks:** `IsDLCUnlocked`, `IsRetoolerFeatureUnlocked`, `IsCauldronUnlocked`, `IsCowGunUnlocked`,
  `AreSupplyDropsUnlocked`, `HasUnlockedSupplyItems`, `IsFastTravelAvailable`.
* **Challenges:** `IsMonthlyChallengeAvailable`/`Complete`/`RewardClaimed`, `IsDaredevilRaceTierReached`,
  `IsAgencyRingCourseActive`/`Complete`.
* **Supply/loadout:** `IsSupplyCategorySelectedInUI`, `IsSupplyEquippedByPilot`, `IsSupplyLoadoutMenuActive`,
  `IsPilotReady`, `IsPilotSelectedInUI`/`InFastTravelUI`.
* **Collectibles / tutorial / media:** `CConditional_CollectibleCompletion`, `CConditional_CollectibleStatus`,
  `IsTutorialPhaseActive`/`Completed`, `IsMediaRevolutionMilestoneApplied`.

### 4. Supply-drop lifecycle — a per-crate state machine  (proven)

A supply-drop object is a small state machine; its state enum lives at object offset `+8`, and every
transition **broadcasts a string-named gameplay event** via `thunk_FUN_148fdf5b0(name)`:

* `FUN_140bec1c0` — on crate creation: fires `"supply_drop_crate_spawned"` and sets `state(+8) = 2`.
* `FUN_149b478f0` (per-frame settle check) — tracks crate position (`+0x88..+0x90`); once it settles below a
  velocity²/timeout threshold (`DAT_141ca6cc4`, timeout `DAT_141cae1bc`) it fires `"requested_supply_landed"`
  (or `"requested_supply_landed_water"` if over water, detected via `thunk_FUN_149e23710(...,DAT_141cae1ac,2)`)
  and sets `state(+8) = 3`.

The observed event/state names form the drop lifecycle (proven strings):
`requested_supply` → `requested_supply_spawned` → `requested_supply_in_world` → `requested_supply_prebreak`
→ `requested_supply_break` → `requested_supply_landed` / `_landed_water`; plus `supply_drop_crate_spawned`,
`supply_drop_cancelled`, `supply_drop_blocking`. Supporting classes: `CSupplyDropManager`,
`CSupplyDropDescription`, `CSupplyDroppableItem`, `CSupplyFactory`, `CSupplyRewardManager`, `CResupplyPoint`,
`CSupplyTutorialLimiter`, `CLocationResolverSupply`/`…Category`. (proven)

### 5. Progression counters & telemetry — `FUN_1409ff030`  (proven)

`FUN_1409ff030` (`size=5300`, caller `0x14949d990`) builds an **OSDK JSON object** (via
`osdk_JSONObject_setMemberString`, `osdk_FloatToAscii`, `osdk_String_*`) that is the canonical progression/
stats snapshot. It writes typed keys (Hungarian prefixes = wire type): header `s__UUID`, `f__Progress`,
`s__Zone`, then progression counters. Proven keys include:

* `i32_collectible_count`, `i32_quests_completed_count`, `i32_operation_mission_completes_count`,
  `i32_operation_mission_attempts` / `_completes`, `i32_supply_drops_collected_count`,
  `i32_supply_drops_requested_count`, `i32_vehicles_unlocked_count`, `i32_weapons_unlocked_count`.
* `i64_operation_mission_duration_ms`, `i64_operation_mission_checkpoint_duration_ms`.
* `s_last_quest_completed`, `s_last_mission_checkpoint_completed`, `s_last_operation_mission_completed`,
  `s_operation_mission_checkpoint_id` / `_status` / `_fail_reason`.
* `s_supply_drop_object_id`, `s_supply_drop_pilot`, `s_supply_drop_status`.

`f__Progress` is read from a float at `*(param_1[1]+8)+0x70`; `s__Zone` from `…+0x74`. This is the concrete
"what counts as progression" ledger. (proven — keys and serializer calls read directly; whether this feeds
save-game vs. remote analytics is *inferred* from the OSDK/JSON framing.)

### 6. Notifications & DLC store  (proven)

Completion feedback routes through `CNotificationManager` with generic pop-up templates:
`popup_generic_quest_complete`, `popup_generic_collectible_complete`, `popup_generic_encounter_complete`.
The DLC purchase path is a UI-command dispatch: `FUN_1409b8480` calls
`FUN_1409a58e0(uiTarget, "ShowDLCStoreDialogAsync", args, 0)` (callers `FUN_140dcbe50`, `FUN_140dd2310`,
`FUN_14a633e60`) — i.e. DLC that isn't owned deep-links to the store dialog. (proven)

## Data & config integration

* **Class name → type descriptor.** Objectives, goals, params, conditionals and supply items are all
  instantiated by name through `FUN_140f27f60`. In the asset pipeline these names are the same class strings
  RTPC/ADF entity components reference (cross-ref `[[rtpc-entity-assembly]]`, `[[composite-assets]]`), so a
  mission/objective asset is a data graph naming `CObjectiveGoal_*` / `CObjectiveParam_*` / `CConditional_*`
  nodes. (inferred — the naming convention matches the entity-component class-hash scheme; not byte-verified here.)
* **Condition trees gate content.** Quest availability, objective visibility, spawn eligibility, and UI state
  are expressed as `CConditional_*` trees evaluated by the shared conditional VM (`CConditionalManager`,
  registered in `FUN_148f960c0` at `PTR_LAB_141d906a8`; see `behavior_system.md`). Progression managers are the
  data sources those conditionals query. (proven that the conditions exist and are factory-registered;
  the manager-query wiring per condition is *inferred*.)
* **Stats are a typed JSON schema.** The `i32_/i64_/s_/f_` prefixes in `FUN_1409ff030` are a fixed telemetry
  schema; consumers key off exact strings. (proven)

## Notable constants / tunables

| Constant | Where | Meaning |
|---|---|---|
| `"CConditional_IsMissionCompleted"` len `0x1f` | `FUN_140651870`, `FUN_140e53520` | Mission-complete gate name/len |
| `"CConditional_IsDLCUnlocked"` len `0x1a` | `FUN_14063df00`, `FUN_140bdf5d0` | DLC gate name/len |
| supply state `+8 = 2` | `FUN_140bec1c0` | Crate spawned / in world |
| supply state `+8 = 3` | `FUN_149b478f0` | Crate landed |
| `DAT_141ca6cc4` | `FUN_149b478f0` | Settle distance² threshold for "landed" |
| `DAT_141cae1bc` | `FUN_149b478f0` | Settle timeout (accumulated in `+0x9c`) → forces landed |
| `DAT_141cae1ac` (cmp mode `2`) | `FUN_149b478f0` | Water-surface test → `_landed_water` |
| manager vtable table | `PTR_LAB_141d906e8 … 141d90c28` | Progression manager vtables (see §1) |
| inline manager hash ids | `FUN_148f960c0` | e.g. `CModelInstanceManager` `0x41cf8298`, `CSoundSystem` `0x41cc6ff0` |

## Call-graph highlights

* **Bootstrap:** engine init → `FUN_148f960c0` registers all progression managers (+ sibling registrar at
  ~line 3209415). (proven)
* **Type creation:** any `CObjective*/CConditional*/CSupply*` construction → per-type getter (e.g.
  `FUN_14090e9a0`, `FUN_140651870`) → `FUN_140f27f60(name,len)` → cached `DAT_` descriptor. (proven)
* **Supply drop:** `0x140bebf50` → `FUN_140bec1c0` (spawn, event `supply_drop_crate_spawned`) → per-frame
  `FUN_149b478f0` (settle, event `requested_supply_landed[_water]`) → `thunk_FUN_148fdf5b0` broadcast. (proven)
* **Stats:** `0x14949d990` → `FUN_1409ff030` (JSON progression snapshot). (proven)
* **DLC store:** `FUN_140dcbe50` / `FUN_140dd2310` / `FUN_14a633e60` → `FUN_1409b8480` →
  `FUN_1409a58e0("ShowDLCStoreDialogAsync")`. (proven)
* **DLC gate in game mode:** `FUN_140bdf5d0` builds `IsDLCUnlocked` conditional, evaluates it, ORs result into
  mode flag `+0x254`. (proven construction; semantic of the evaluated bit is inferred.)

## Open questions / lower-confidence

* The concrete **Evaluate() vtable methods** for `CConditional_IsMissionCompleted` / `IsDLCUnlocked` (i.e. the
  exact manager query each performs) were not located — only their construction/caching. The conditions are
  reached indirectly through `CConditionalManager` dispatch. (open)
* **Two `CStatisticManager` registrations** in `FUN_148f960c0` — likely two stat scopes (session vs. persistent
  / local vs. online); not confirmed. (inferred)
* Whether `FUN_1409ff030`'s JSON is a **save-game** blob, a **remote telemetry** payload, or both is unresolved
  (OSDK naming leans telemetry). (inferred)
* The **`CObjectiveGoal_*` completion math** (counter thresholds, area/distance tests) lives in each goal's
  Update method — not yet traced from the factory registration to the implementation. (open)
* `CActivityManager` / `COperationManager` **registration-of-activity** flow (how an activity/operation is
  added, tracked, and marked complete) is named but its tick was not read. (open)

## Appendix — decomp anchors

**Manager registrar (proven):** `FUN_148f960c0` @0x148f960c0 (size=4805); sibling ~line 3209415.
Progression vtables `PTR_LAB_141d906e8`(Activity) `…708`(Objective) `…728`(ObjectiveContent) `…768`(Reward)
`…7c8`(NewGamePlus) `…828`(Mission) `…848`(Quest) `…9a8`(Operation) `…9e8`(DLC) `…a28`/`…c28`(Statistic×2)
`…a48`(Notification) `…a68`(Challenge) `…bc8`(Collection) `…be8`(Supply) `…c08`(SupplyFactory).

**Type-registry factory (proven):** `FUN_140f27f60` @0x140f27f60 (size=5, 6138 callers). Example getters:
`FUN_14090e9a0` ("CObjective",10 → `DAT_142cb9250`), `FUN_140651870` ("CConditional_IsMissionCompleted",0x1f →
`DAT_142cb5c78`), `FUN_14063df00` ("CConditional_IsDLCUnlocked",0x1a → `DAT_142cb5ae8`),
`FUN_14063ce20` ("CConditional_AreSupplyDropsUnlocked").

**Typename methods (proven):** `FUN_149247310`→"CObjectiveContentManager", `FUN_148eb7e00`→"CActivityManager",
`FUN_149b16e00`→"CSupplyManager", `FUN_147fc30ec`→"CNotificationManager". (`FUN_14f064721` returns
"CQuestManager" but is control-flow-flattened junk — ignore.)

**Supply state machine (proven):** `FUN_140bec1c0` @0x140bec1c0 (event `supply_drop_crate_spawned`, state=2);
`FUN_149b478f0` @0x149b478f0 (events `requested_supply_landed[_water]`, state=3, thresholds `DAT_141ca6cc4`,
`DAT_141cae1bc`, `DAT_141cae1ac`). Event broadcaster `thunk_FUN_148fdf5b0` (29 sites).

**Stats serializer (proven):** `FUN_1409ff030` @0x1409ff030 (size=5300, caller 0x14949d990); OSDK helpers
`osdk_JSONObject_setMemberString`, `osdk_FloatToAscii`, `osdk_String_constructWithSize/resize/constructCopy`.

**DLC store (proven):** `FUN_1409b8480` @0x1409b8480 → `FUN_1409a58e0(...,"ShowDLCStoreDialogAsync",...)`.

**Class-name string inventory (proven):** objective goals `CObjectiveGoal[_Area/_Counter/_CategoryCounter/
_Distance/_TimeLimit/_StayNearObject/_Conditional]`; params `CObjectiveParam_[ProgressBar/HealthBar/
TimerPopup/TornadoWidget/DaredevilScoreAndTimer/BundleBlips/AgencyRingCourse/StayInAreaHack]`; time
`CObjectiveTime[/Cap/Set]`; supply `CSupply[Manager/FactoryManager/Factory/DropManager/DropDescription/
DroppableItem/RewardManager/TutorialLimiter]`, `CResupplyPoint`, `CLocationResolverSupply[/Category]`;
collectibles `CCollectibleObject[/CandySkull/RingGroup]`, `CCollectibleRing`, `CCollectibleCategory`,
`CCollectibleSettings`, `CConditional_Collectible[Completion/Status]`; managers `CQuestManager`,
`CMissionManager`, `COperationManager`, `CActivityManager`, `CObjective[Content]Manager`, `CRewardSystem`,
`CCollectionManager`, `CStatisticManager`, `CNotificationManager`, `CChallengeManager`,
`CDownloadableContentManager`, `CNewGamePlusManager`, `CProgressionBookmark`. Full `CConditional_Is*` list in §3.

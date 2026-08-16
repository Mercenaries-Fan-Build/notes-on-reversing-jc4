# Progression Economy — rewards, unlocks, supply & inventory (the reward/unlock/inventory layer)

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

All FUN_/DAT_ addresses are from `output/_ghidra_jc4/jc4_all_functions_decomp.txt` (Ghidra decomp of
`JustCause4.exe`, 162,115 functions). "proven" = read directly from that decomp text.

> **Scope split.** Missions/objectives/DLC-gating and the supply-drop **crate state machine** live in
> [missions_progression.md](missions_progression.md) — this doc does **not** re-derive them. Here the focus is the
> **economy**: how rewards are defined and granted (`CRewardSystem` / the `C*Reward` taxonomy), the **supply
> factory** that produces droppable items (`CSupplyFactory` → `CSupplyDroppableItem`), the player-owned
> **stash** (`CStashManager`), how **unlock conditions** gate loadout/pilot/retooler content (ties to the
> `CConditional_*` / `C*Condition` families in [behavior_system.md](behavior_system.md)), and **New Game Plus**
> carry-over (`CNewGamePlusManager`). The **retooler** is JC4's loadout/customization subsystem and is documented
> here because it is the equipment economy.

> **Methodology caveat (proven, functions-only export).** What is recoverable from this export is the
> **registration skeleton** (`FUN_14085fd00` component registrar, `FUN_148f960c0` / the ~line-3209430 sibling
> manager registrar, `FUN_1485b9730` condition registrar), the **type/class-name inventory**, the **call graph**,
> and **string constants**. What is **not** recoverable are the per-manager **tick/grant bodies** and the
> **tunable magnitudes** (reward amounts, unlock thresholds) — those live in reflected data (RTPC/ADF) and in
> vtable methods reached only through indirect dispatch. Walled items are tagged **(open)** with a resolve route.

## Overview

JC4's economy is not a single "currency + shop" system; it is a set of **named singleton managers** plus a
**class-name-keyed type taxonomy**, all built on the same engine spine as every other system (see the
README's "Cross-cutting architecture"):

* **Rewards** are declared as reflected objects of a small **`C*Reward` taxonomy** (`CChaosReward`,
  `CBackerReward`, `CMediaRevolutionReward`) and dispensed through **`CRewardSystem`** (a per-entity component)
  and **`CSupplyRewardManager`** (the supply-specific reward path). (proven — taxonomy + registrations)
* **Supply items** are produced by **`CSupplyFactory`** — the factory type that instantiates
  **`CSupplyDroppableItem`** payloads that the supply-drop crate delivers. `CSupplyFactoryManager` owns the
  factory set; `CResupplyPoint` marks where resupply happens. (proven — types + registrations)
* **The stash** (`CStashManager`) is the player-owned pool that outlives a single loadout — registered as a
  world manager and referenced by entities through its reflected type-id. (proven — registration + type-id
  resolver; the pool contents/add-remove ops are **open**.)
* **Unlocks/prerequisites** are expressed as **conditions**, in two registries: the game-logic
  **`CConditional_*Unlocked` family** (DLC, pilots, supply items, retooler features, cauldron, cow-gun) and the
  behavior-tree **`C*Condition` family** (`CUpgradePurchasedCondition`, `CInventoryCondition`,
  `CInventoryHasWeaponAmmunitionCondition`). Both are evaluated by the shared conditional/behavior VM
  ([behavior_system.md](behavior_system.md)); the economy managers are the data sources they query.
  Dedicated **`C*Unlocker`** objects (`CMapUnlocker`, `CHoverboardUnlocker`, `CPremiumWingsuitUnlocker`,
  `CChallengeUnlockController`) flip specific feature gates, and **`CPrerequisiteObject`** encodes a
  content prerequisite. (proven — full string/registration inventory)
* **Loadout/equipment** is the **retooler** (`CRetoolerManager` + `CRetoolerLoadout{Object,Editable,Visibility}`)
  and the **pilot** system (`CPilotDataManager` + `CPilotData`) that selects who/what a supply drop equips.
  (proven — types + registrations)
* **New Game Plus** is a manager (`CNewGamePlusManager`) plus a `CProgressionBookmark` named `"new_game_plus"`;
  each save slot carries an `m_NewGamePlus` flag byte. (proven — registration, bookmark, save-slot flag)

## Key classes & functions

| Class / string | FUN_ (registration / resolver) | Role |
|---|---|---|
| `CRewardSystem` | manager: sibling registrar @~3209435 (`PTR_LAB_141d90768`); type-id resolver `FUN_140946b80` (via `FUN_140938e70`, id `DAT_142cba748`) | Grants rewards; per-entity reward component |
| `CSupplyRewardManager` | manager registrar @~3209675 (`PTR_LAB_141d90d68`) | Supply-specific reward dispensing |
| `CChaosReward` | type `FUN_140f27f60("CChaosReward",0xc)` → `DAT_142cba720`; component reg `thunk_FUN_147cafcd0` @~4140692 | Reward paid in "chaos" (JC currency of progression) |
| `CBackerReward` | type `FUN_140f27f60("CBackerReward",0xd)` → `_DAT_142cb97e4` (in `FUN_140844e50`) | Kickstarter/backer reward payload |
| `CMediaRevolutionReward` | type `FUN_140f27f60("CMediaRevolutionReward",0x16)` → `DAT_142cb9d7c` | Media-revolution milestone reward |
| `CSupplyFactory` | type reg `FUN_140813fd0` → `DAT_142cb92c0` (via `FUN_14085fd00`, vtable `PTR_LAB_141d9d1b8`) | Produces supply items |
| `CSupplyFactoryManager` | manager registrar @~3209620 (`PTR_LAB_141d90c08`) | Owns the factory set |
| `CSupplyDroppableItem` | type reg `FUN_140813f10` → `DAT_142cb92b8` (`"CSupplyDroppableItem",0x14`) | The item a supply crate delivers |
| `CSupplyDropDescription` | type `FUN_140f27f60(...,0x16)` → `_DAT_142cb9d44` | Describes a requested drop |
| `CResupplyPoint` | type reg → `DAT_142cb8e00` (`"CResupplyPoint",0xe`) | World resupply location |
| `CSupplyTutorialLimiter` | type reg `FUN_140814090` → `DAT_142cb8e74` (`,0x16`) | Gates supply during tutorial |
| `CStashManager` | manager registrar @~3209590 (`PTR_LAB_141d90b48`); type-id resolver `FUN_1408df410`/`FUN_1408d26e0` (id `DAT_142cb92c0`… see note) | Player-owned item pool |
| `CNewGamePlusManager` | manager registrar @~3209450 (`PTR_LAB_141d907c8`); type-id resolver `FUN_140b833c0`/`FUN_140b64880` | NG+ progression state |
| `CProgressionBookmark` "new_game_plus" | ctor `FUN_140b6b7a0`; type reg `FUN_140811c30` → `DAT_142cb92b0` (`,0x14`) | Bookmark marking the NG+ boundary |
| `CRetoolerManager` | manager registrar @~3209445 (`PTR_LAB_141d907a8`) | Loadout/customization ("retooler") |
| `CRetoolerLoadoutObject` | type reg `FUN_140948150` → `DAT_142cb93c0` (`,0x16`) | An editable loadout instance |
| `CRetoolerLoadout{Editable,Visibility}` | component reg `thunk_FUN_147cafcd0` @~4140702/4140697 | Loadout edit/visibility components |
| `CPilotDataManager` | manager registrar @~3209695 (`PTR_LAB_141d90de8`) | Pilot roster (who delivers/equips) |
| `CPilotData` | type reg `FUN_140811520` → `DAT_142cb8dd8` (`"CPilotData",10`) | One pilot's data |
| `CPrerequisiteObject` | type reg → `DAT_142cb9238` (`,0x13`) | Content prerequisite node |
| `CMapUnlocker` | type → `DAT_142cb8d80` (`,0xc`) | Unlocks map region |
| `CHoverboardUnlocker` | type → `_DAT_142cb9764` (`,0x13`) | Unlocks hoverboard |
| `CPremiumWingsuitUnlocker` | type → `DAT_142cb8df0` (`,0x18`) | Unlocks premium wingsuit (DLC) |
| `CChallengeUnlockController` | type → `_DAT_142cb9958` (`,0x1a`) | Drives challenge-based unlocks |
| Global type-registry factory | `FUN_140f27f60(name,len)` (6138 callers) | Name → cached type descriptor |

Managers registered in the ~line-3209430 sibling registrar (a near-clone of `FUN_148f960c0`; see
missions_progression.md §1). Each `thunk_FUN_1496a12b0(8)` holder gets a vtable `PTR_LAB_141d90…` then
`thunk_FUN_148f6e780(container,"<ClassName>",holder)`. (proven)

**Note on the `DAT_142cb92c0` collision:** the CSupplyFactory type descriptor (`FUN_140813fd0`) and the
CStashManager type-id path both surface `DAT_142cb92c0` in the export near the same region — the
CStashManager resolver (`FUN_1408df410` → `FUN_1408d26e0`) uses the long **`ArGetTypeId<class CStashManager>`**
key (len `0x40`) which caches into its own slot; treat the two as distinct despite the decomp's reused label.
(inferred — label reuse in the export; the two callsites use different keys/lengths.) (open: exact stash slot.)

## How it works (from the decomp)

### 1. Two registries, one namespace

Everything in the economy is instantiated **by class name** through the global factory `FUN_140f27f60(name,len)`
(the same 6,138-caller factory used everywhere; README + missions §2). Two distinct registration idioms appear,
and the economy uses both:

* **Reflected component/type registration — `FUN_14085fd00`.** Per-type thunks (e.g. `FUN_140813fd0` for
  `CSupplyFactory`, `FUN_140813f10` for `CSupplyDroppableItem`, `FUN_140811520` for `CPilotData`,
  `FUN_140811c30` for `CProgressionBookmark`) each allocate a `0x10`-byte holder, store a vtable pointer
  (`PTR_LAB_141d9…`), lazily resolve the type descriptor via `FUN_140f27f60("<Name>", strlen)`, and register it
  with `thunk_FUN_14cfef490`. These are the **data types** an asset can name. (proven — read directly in
  `FUN_140813fd0` etc.)
* **Manager registration — the sibling registrar @~3209430.** Allocates an 8-byte holder, stores a manager
  vtable (`PTR_LAB_141d90…`), and registers it by name with `thunk_FUN_148f6e780`. Every economy **manager**
  (`CRewardSystem`, `CSupplyRewardManager`, `CStashManager`, `CSupplyFactoryManager`, `CSupplyDropManager`,
  `CRetoolerManager`, `CNewGamePlusManager`, `CPilotDataManager`) is a row here. (proven — read directly.)

The **length argument** to `FUN_140f27f60` is always the literal `strlen`, a useful oracle: `"CChaosReward"`=0xc,
`"CBackerReward"`=0xd, `"CPilotData"`=10, `"CSupplyFactory"`=0xe, `"CSupplyDroppableItem"`=0x14,
`"CResupplyPoint"`=0xe, `"CRetoolerLoadoutObject"`=0x16, `"CPrerequisiteObject"`=0x13. (proven)

### 2. Rewards — the `C*Reward` taxonomy and `CRewardSystem`  (proven types; grant body open)

Rewards are reflected objects of a **three-member taxonomy**, each a distinct reward *kind*:

* **`CChaosReward`** (`DAT_142cba720`) — reward denominated in "chaos", JC4's core progression currency
  (chaos drives frontline/region takeover). Registered both as a factory type and as a reflected component
  (`thunk_FUN_147cafcd0(container,"CChaosReward",…)` @~4140692). (proven)
* **`CBackerReward`** (`_DAT_142cb97e4`, resolved in `FUN_140844e50`) — the Kickstarter-backer reward payload.
  (proven)
* **`CMediaRevolutionReward`** (`DAT_142cb9d7c`) — reward tied to a "media revolution" milestone (paired with the
  condition `CConditional_CanUnlockNextMediaRevolutionMilestone`). (proven)

`CRewardSystem` is registered as a **per-entity reward component** (README component family). Its type-id
resolver `FUN_140946b80` follows the standard reflected-reference pattern: if the property node
(`*(int*)(param_2+1) != 1`) is unset it flags `+0xc |= 2` and returns; otherwise it resolves the
`ArGetTypeId<class CRewardSystem>` id (`DAT_142cba748`, key len `0x40`), fetches the component via
`FUN_140938e70`, and sets a match bit `+0xc |= (id != found)`. This is the **"does this entity carry a
RewardSystem"** resolver, not the grant itself. (proven — resolver; the actual "add reward to player" method is
reached through the component vtable and is **open**.)

The **grant surface that IS visible** is the reward **UI** dispatch: functions push Scaleform commands
`"SetReward"` (`FUN` @~1628280), `"AddRewardToStage"` and `"ShowRewards"` (`FUN_140e2d490` @~1644788) into the
`RewardUI` movie (`REWARD_CONTAINER`, `REWARD_IMAGE`, `REWARD_TEXT`, `REWARDS_TITLE`) — i.e. once a reward is
granted, the results screen is populated by name. `"AddRewardToStage"` iterates a reward array
(`param_1[0x5f]`, stride `0x30`). (proven — Scaleform callback strings + iteration read directly.)

### 3. Supply factory — what produces the delivered items  (proven types; produce body open)

The supply economy is a factory pipeline distinct from the crate *state machine* (that's in
missions_progression §4):

* **`CSupplyFactory`** (`DAT_142cb92c0`, reg `FUN_140813fd0`, vtable `PTR_LAB_141d9d1b8`) — the factory type
  that **produces** supply items. `CSupplyFactoryManager` (`PTR_LAB_141d90c08`) owns the set of factories.
* **`CSupplyDroppableItem`** (`DAT_142cb92b8`, reg `FUN_140813f10`) — the concrete **payload** a factory makes
  and a crate delivers (a vehicle, weapon, or gadget instance).
* **`CSupplyDropDescription`** (`_DAT_142cb9d44`) — the request descriptor (what/where/which pilot).
* **`CResupplyPoint`** (`DAT_142cb8e00`) — a world location that grants resupply.
* **`CSupplyTutorialLimiter`** (`DAT_142cb8e74`, reg `FUN_140814090`) — throttles supply availability during the
  tutorial, gating the economy until the mechanic is taught.

So the delivery economy is: **`CSupplyFactoryManager` → `CSupplyFactory` produces a `CSupplyDroppableItem`**,
described by a `CSupplyDropDescription`, delivered by the `CSupplyDropManager` crate state machine (missions doc),
equipped per the pilot's loadout (§5). The **factory `Produce()` body** (how a description maps to a concrete
model/entity) is behind the factory vtable and is **open**. (proven — the type pipeline and registrations; the
production math is walled.)

The stats ledger (missions §5, `FUN_1409ff030`) confirms the economy's countable units:
`i32_supply_drops_requested_count`, `i32_supply_drops_collected_count`, `i32_vehicles_unlocked_count`,
`i32_weapons_unlocked_count`, plus per-drop `s_supply_drop_object_id` / `s_supply_drop_pilot` /
`s_supply_drop_status`. (proven — cross-ref)

### 4. The stash — player-owned pool  (proven registration; ops open)

`CStashManager` is registered as a world manager (`PTR_LAB_141d90b48`). Entities reference it through the
reflected type-id `ArGetTypeId<class CStashManager>` (key len `0x40`), resolved by `FUN_1408df410` (which caches
the id and calls the resolver `FUN_1408d26e0`, the same reference-resolver shape as `CRewardSystem` §2). The
stash is the **persistent player pool** items flow into/out of; its **add/remove/query ops** live behind the
manager vtable and were not reached from the registration skeleton. (proven — registration + type-id resolver;
pool operations **open**. Resolve route: breakpoint the `PTR_LAB_141d90b48` vtable in x64dbg while items are
stashed, or trace callers of `FUN_1408d26e0`.)

### 5. Loadout / equipment — the retooler and pilots  (proven types)

JC4's "equipment economy" is the **retooler** plus the **pilot** selection that binds a loadout to a supply drop:

* **`CRetoolerManager`** (`PTR_LAB_141d907a8`) owns loadout/customization. **`CRetoolerLoadoutObject`**
  (`DAT_142cb93c0`) is an editable loadout; **`CRetoolerLoadoutEditable`** and **`CRetoolerLoadoutVisibility`**
  are reflected components (registered `thunk_FUN_147cafcd0` @~4140702/4140697) controlling whether a loadout
  slot can be edited and whether it is shown. (proven)
* **`CPilotDataManager`** (`PTR_LAB_141d90de8`) holds the pilot roster; **`CPilotData`** (`DAT_142cb8dd8`) is one
  pilot. The pilot is who a supply drop equips — the condition family below reads pilot/loadout UI state.
  (proven)

The loadout/pilot UI/gate state is surfaced entirely through the **`CConditional_*` family** (evaluated by the
behavior VM, [behavior_system.md](behavior_system.md)):

* Retooler: `CConditional_IsRetoolerGroupSelected`, `_IsRetoolerOptionEquipped`, `_IsRetoolerLoadoutSelected`,
  `_IsRetoolerFeatureUnlocked` (len `0x26`, `DAT_142cb5ee0`).
* Supply/loadout: `_IsSupplyCategorySelectedInUI`, `_IsSupplyEquippedByPilot`, `_IsSupplyLoadoutMenuActive`,
  `_HasUnlockedSupplyItems` (len `0x23`, `DAT_142cb5a98`), `_AreSupplyDropsUnlocked`.
* Pilot: `_HasPilotUnlocked` (len `0x1d`, `DAT_142cb5a88`), `_IsPilotReady`, `_IsPilotSelectedInUI`,
  `_IsPilotSelectedInFastTravelUI`.

All are registered together in the component/conditional registrar block ~line 4140000 via
`thunk_FUN_147cafcd0`. (proven — string inventory; per-condition Evaluate() bodies **open**, same as missions
doc's conditional caveat.)

### 6. Unlock graph & prerequisites  (proven inventory; evaluate bodies open)

Unlocks are expressed two ways, both feeding the behavior/conditional VM:

* **Game-logic unlock conditions (`CConditional_*Unlocked`)** — the gate predicates:
  `IsDLCUnlocked` (`0x1a`), `HasPilotUnlocked` (`0x1d`), `HasUnlockedSupplyItems` (`0x23`),
  `AreSupplyDropsUnlocked`, `IsCauldronUnlocked` (`0x1f`, `DAT_142cb5ad8`), `IsCowGunUnlocked` (`0x1d`,
  `DAT_142cb5ae0`), `IsRetoolerFeatureUnlocked` (`0x26`), `CanUnlockNextMediaRevolutionMilestone`,
  `IsMonthlyChallengeRewardClaimed`. (Cauldron = the weapon-mod bench; cow-gun = a novelty unlock.) (proven)
* **Behavior-tree conditions (`C*Condition`)** — registered through the *other* condition registry
  `thunk_FUN_14aadec10` (i.e. `FUN_1485b9730`, see behavior_system): **`CUpgradePurchasedCondition`**
  (`DAT_142cb4494`, `FUN_14054d090`) — the "player bought this upgrade" gate; **`CInventoryCondition`**
  (`DAT_142cb4594`); **`CInventoryHasWeaponAmmunitionCondition`** (`DAT_142cb4434`). These gate AI/behavior
  transitions on inventory/purchase state. (proven)

Actual unlock **actuators** are dedicated objects that flip a specific feature gate when their prerequisite is
met: **`CMapUnlocker`**, **`CHoverboardUnlocker`**, **`CPremiumWingsuitUnlocker`**, **`CChallengeUnlockController`**.
A generic prerequisite node is **`CPrerequisiteObject`** (`DAT_142cb9238`). Together these form the unlock graph:
a `CPrerequisiteObject`/`CConditional_*` predicate gates a `C*Unlocker` that enables content, surfaced to UI by
the matching `CConditional_Is*Unlocked`. (inferred wiring — the object set and conditions are proven; the
per-unlocker "set flag" body is **open**. Resolve route: trace `PTR_LAB` vtables of the unlocker holders.)

### 7. New Game Plus carry-over  (proven markers; carried-set open)

* **`CNewGamePlusManager`** (`PTR_LAB_141d907c8`) is the NG+ manager; entities reach it via
  `ArGetTypeId<class CNewGamePlusManager>` (len `0x46`), resolver `FUN_140b833c0`/`FUN_140b64880`. (proven)
* A **`CProgressionBookmark` named `"new_game_plus"`** is constructed in `FUN_140b6b7a0` (via
  `FUN_140b6cef0(obj+2,"new_game_plus",1,…)`), marking the NG+ boundary in the progression bookmark stream.
  A related UI/info object uses the key `"new_game_plus_info"` (`FUN_140b91130`). (proven)
* Each **save slot carries a per-slot NG+ flag**: the save-slot enumerator `FUN_140e24680` writes, per slot,
  `m_Text`, `m_CurrentSaveSlot` (byte at slot+`0x1e0`) and **`m_NewGamePlus`** (byte at slot+`0x1e1`) into the
  slot's UI record. So NG+ is a **boolean property of a save slot**, not a separate save. (proven)

What **carries over** into an NG+ run (which unlocks/stash/loadout persist vs. reset) is decided in the NG+
manager tick, which is not in this export. (open. Resolve route: `PTR_LAB_141d907c8` vtable + save-serialize
callers of `FUN_140e24680`.)

## Data & config integration

* **Class name → type descriptor.** Every economy type (`C*Reward`, `CSupplyFactory`, `CSupplyDroppableItem`,
  `CPilotData`, `CRetoolerLoadoutObject`, `CPrerequisiteObject`, `C*Unlocker`) is instantiated by name through
  `FUN_140f27f60`. In the asset pipeline those names are the class strings RTPC/ADF entity components reference
  (cross-ref `[[rtpc-entity-assembly]]`, `[[composite-assets]]`), so a reward/loadout asset is a data graph
  naming these nodes. (inferred — naming convention matches the component class-hash scheme; not byte-verified
  here.)
* **Conditions gate content.** Unlock/loadout/pilot availability is a tree of `CConditional_*` /`C*Condition`
  nodes evaluated by the shared VM; the economy managers (`CDownloadableContentManager`, `CPilotDataManager`,
  `CRetoolerManager`, `CSupplyManager`) are the data sources those conditions query. (proven that the conditions
  exist and are registered; the per-condition manager query is **inferred**.)
* **Economy counters are telemetry keys.** The typed JSON schema in `FUN_1409ff030` (missions §5) is where the
  economy's outcomes are counted (`i32_vehicles_unlocked_count`, `i32_weapons_unlocked_count`,
  `i32_supply_drops_*`). (proven)

## Notable constants / tunables

| Constant / marker | Where (FUN_) | Meaning |
|---|---|---|
| `"CChaosReward"` len `0xc` → `DAT_142cba720` | `FUN` @1055691 | Chaos-currency reward type |
| `"CBackerReward"` len `0xd` → `_DAT_142cb97e4` | `FUN_140844e50` | Backer reward type |
| `"CMediaRevolutionReward"` `0x16` → `DAT_142cb9d7c` | @865563 | Milestone reward type |
| `ArGetTypeId<class CRewardSystem>` len `0x40` → `DAT_142cba748` | `FUN_140946b80` | RewardSystem component id |
| `"CSupplyFactory"` `0xe` → `DAT_142cb92c0` | `FUN_140813fd0` | Supply factory type |
| `"CSupplyDroppableItem"` `0x14` → `DAT_142cb92b8` | `FUN_140813f10` | Delivered item type |
| `"CResupplyPoint"` `0xe` → `DAT_142cb8e00` | @840518 | Resupply location type |
| `ArGetTypeId<class CStashManager>` len `0x40` | `FUN_1408d26e0`/`FUN_1408df410` | Stash manager id |
| `"CPilotData"` len `10` → `DAT_142cb8dd8` | `FUN_140811520` | Pilot data type |
| `"CRetoolerLoadoutObject"` `0x16` → `DAT_142cb93c0` | `FUN_140948150` | Editable loadout type |
| `"CPrerequisiteObject"` `0x13` → `DAT_142cb9238` | @867579 | Prerequisite node type |
| `"CUpgradePurchasedCondition"` → `DAT_142cb4494` | `FUN_14054d090` | Purchase gate (behavior registry) |
| `CConditional_IsRetoolerFeatureUnlocked` `0x26` → `DAT_142cb5ee0` | @627527 | Retooler feature gate |
| `CConditional_HasUnlockedSupplyItems` `0x23` → `DAT_142cb5a98` | @605689 | Supply-items gate |
| save-slot `+0x1e0` `m_CurrentSaveSlot`, `+0x1e1` `m_NewGamePlus` | `FUN_140e24680` | Per-slot NG+ flag |
| bookmark `"new_game_plus"` | `FUN_140b6b7a0` | NG+ boundary bookmark |
| manager vtables | sibling registrar @~3209430 | `CRewardSystem` `141d90768`, `CRetoolerManager` `141d907a8`, `CNewGamePlusManager` `141d907c8`, `CStashManager` `141d90b48`, `CSupplyManager` `141d90be8`, `CSupplyFactoryManager` `141d90c08`, `CSupplyDropManager` `141d90d48`, `CSupplyRewardManager` `141d90d68`, `CPilotDataManager` `141d90de8` |

## Call-graph highlights

* **Type/manager bootstrap:** engine init → `FUN_14085fd00` registers reflected economy *types*
  (`FUN_140813fd0` CSupplyFactory, `FUN_140813f10` CSupplyDroppableItem, `FUN_140811520` CPilotData, …);
  the sibling registrar @~3209430 registers economy *managers*. (proven)
* **Reward resolve:** entity property → `FUN_140946b80` → `FUN_140938e70` (get component by id
  `DAT_142cba748`) → match bit. (proven)
* **Reward UI:** grant → `FUN_140e2d490` pushes `"AddRewardToStage"` / `"ShowRewards"` into `RewardUI`;
  `"SetReward"` @1628280. (proven)
* **Supply pipeline:** `CSupplyFactoryManager` → `CSupplyFactory` (`DAT_142cb92c0`) → `CSupplyDroppableItem`
  (`DAT_142cb92b8`) → crate state machine (missions §4). (proven types; produce body open)
* **NG+ save:** `FUN_140e24680` enumerates save slots, writes `m_NewGamePlus` (slot+`0x1e1`);
  `FUN_140b6b7a0` builds the `"new_game_plus"` bookmark. (proven)

## Open questions / lower-confidence

* **Reward grant body.** `CRewardSystem` / `CSupplyRewardManager` "add reward to player" and the `C*Reward`
  Apply() methods are behind vtables, not in this export. Only the resolver and UI dispatch are visible. (open —
  x64dbg the `PTR_LAB_141d90768` / `…d68` vtables on a reward event.)
* **Chaos currency arithmetic.** `CChaosReward` names the currency but the balance store and the earn/spend math
  (frontline chaos accrual) were not located here. (open — cross-ref faction_frontline_chaos.md.)
* **Supply factory `Produce()`** — how a `CSupplyDropDescription` maps to a concrete `CSupplyDroppableItem`
  model/entity. (open — factory vtable `PTR_LAB_141d9d1b8`.)
* **Stash operations.** Only registration + type-id resolver recovered; add/remove/query pool ops unread. (open)
* **NG+ carried set.** Which unlocks/stash/loadout persist vs. reset across an NG+ boundary is decided in the
  `CNewGamePlusManager` tick, not in this export. (open)
* **`CPrerequisiteObject` → `C*Unlocker` wiring** is inferred from the object set + naming, not traced through a
  concrete evaluate→unlock edge. (inferred/open)
* **`CBackerReward`** is present but likely inert in retail (Kickstarter fulfilment); its trigger path was not
  found. (speculative)

## Appendix — decomp anchors

**Manager registrar (proven):** sibling registrar @~line 3209430 (clone of `FUN_148f960c0`). Economy vtables:
`CRewardSystem`=`PTR_LAB_141d90768`, `CRetoolerManager`=`141d907a8`, `CNewGamePlusManager`=`141d907c8`,
`CStashManager`=`141d90b48`, `CSupplyManager`=`141d90be8`, `CSupplyFactoryManager`=`141d90c08`,
`CSupplyDropManager`=`141d90d48`, `CSupplyRewardManager`=`141d90d68`, `CPilotDataManager`=`141d90de8`.

**Reflected type registrations (proven, via `FUN_14085fd00`):** `FUN_140813fd0`→"CSupplyFactory"(`DAT_142cb92c0`,
vtable `PTR_LAB_141d9d1b8`); `FUN_140813f10`→"CSupplyDroppableItem"(`DAT_142cb92b8`);
`FUN_140814090`→"CSupplyTutorialLimiter"(`DAT_142cb8e74`, `PTR_LAB_141d9eba8`);
`FUN_140811520`→"CPilotData"(`DAT_142cb8dd8`); `FUN_140811c30`→"CProgressionBookmark"(`DAT_142cb92b0`);
`FUN_140948150`→"CRetoolerLoadoutObject"(`DAT_142cb93c0`); "CResupplyPoint"(`DAT_142cb8e00`, @840518);
"CPrerequisiteObject"(`DAT_142cb9238`, @867579).

**Reward taxonomy (proven):** `CChaosReward`(`DAT_142cba720`, @1055691; component reg @~4140692),
`CBackerReward`(`_DAT_142cb97e4`, `FUN_140844e50`), `CMediaRevolutionReward`(`DAT_142cb9d7c`, @865563).
Reward UI: `FUN_140e2d490` ("AddRewardToStage"@1644788, "ShowRewards"@1644813), "SetReward"@1628280;
Scaleform ids `RewardUI`/`REWARD_CONTAINER`/`REWARD_IMAGE`/`REWARD_TEXT`/`REWARDS_TITLE`.

**Reference resolvers (proven):** `FUN_140946b80`(RewardSystem, id `DAT_142cba748`, via `FUN_140938e70`);
`FUN_1408df410`→`FUN_1408d26e0`(StashManager, `ArGetTypeId<class CStashManager>` len `0x40`);
`FUN_140b833c0`→`FUN_140b64880`(NewGamePlusManager, len `0x46`).

**Unlock conditions (proven strings):** game-logic `CConditional_*Unlocked` registered `thunk_FUN_147cafcd0`
@~line 4140000 — `IsDLCUnlocked`, `HasPilotUnlocked`(`DAT_142cb5a88`,@605664), `HasUnlockedSupplyItems`
(`DAT_142cb5a98`,@605689), `AreSupplyDropsUnlocked`, `IsCauldronUnlocked`(`DAT_142cb5ad8`,@605865),
`IsCowGunUnlocked`(`DAT_142cb5ae0`,@605890), `IsRetoolerFeatureUnlocked`(`DAT_142cb5ee0`,@627527),
`IsWeaponInInventory`(`DAT_142cb5f50`,@627878), `IsWeaponTypeInInventory`, retooler
`IsRetoolerGroupSelected`/`IsRetoolerOptionEquipped`/`IsRetoolerLoadoutSelected`, supply
`IsSupplyCategorySelectedInUI`/`IsSupplyEquippedByPilot`/`IsSupplyLoadoutMenuActive`, pilot
`IsPilotReady`/`IsPilotSelectedInUI`/`IsPilotSelectedInFastTravelUI`, `CanUnlockNextMediaRevolutionMilestone`,
`IsMonthlyChallengeRewardClaimed`. Behavior-registry `C*Condition` (via `thunk_FUN_14aadec10`/`FUN_1485b9730`):
`CUpgradePurchasedCondition`(`DAT_142cb4494`,`FUN_14054d090`), `CInventoryCondition`(`DAT_142cb4594`,@503688),
`CInventoryHasWeaponAmmunitionCondition`(`DAT_142cb4434`,@503707).

**Unlockers (proven):** `CMapUnlocker`(`DAT_142cb8d80`), `CHoverboardUnlocker`(`_DAT_142cb9764`),
`CPremiumWingsuitUnlocker`(`DAT_142cb8df0`), `CChallengeUnlockController`(`_DAT_142cb9958`).

**New Game Plus (proven):** `CNewGamePlusManager` resolver `FUN_140b833c0`; bookmark ctor `FUN_140b6b7a0`
("new_game_plus"); info `FUN_140b91130` ("new_game_plus_info"); save-slot enumerator `FUN_140e24680`
(`m_CurrentSaveSlot`+`0x1e0`, `m_NewGamePlus`+`0x1e1`).

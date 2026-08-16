# Vehicle data, pilots & the vehicle catalog — the management layer above vehicle physics

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

All function/global addresses are from `output/_ghidra_jc4/jc4_all_functions_decomp.txt`
(162,115 functions). "proven" = read directly from the decomp; "inferred" = a reasonable reading of
structure/flow; "speculative" = plausible but unconfirmed.

This doc is the **data / management layer** that sits *above* the vehicle physics + component family
documented in **`vehicles.md`** (driving, entry/exit/hijack, the ~120-entry driving-force registry,
`CVehicleData` / `CVehicleConfig` / `CVehiclePartProxy` / seat controllers, the `CAir/Land/SeaVehicle`
type hierarchy). It **builds on** that doc and does not repeat it — here we cover the world **managers**
(`CVehicleDataManager`, `CPilotDataManager`, `CRadioSystem`), the **pilot** system, the scripted-mission
**`CObjectiveVehicleController`**, and the **seat/passenger** and **radio** data. Model/mesh assembly lives
in memory `[[model-amf]]` / `[[rtpc-entity-assembly]]`; missions are `missions_progression.md` /
`objectives_operations.md`.

> **METHODOLOGY CAVEAT (functions-only export).** What is recoverable here is the **registration skeleton**
> (the manager-list builder, reflected-type registrars, condition/action registrars), the call graph, string
> constants, and the reachable CPU logic. The **per-manager bodies live behind data-section vtables**
> (`PTR_LAB_141d90de8` = `CPilotDataManager`, `PTR_LAB_141d90e08` = `CVehicleDataManager`,
> `PTR_LAB_141d909a8` = `CRadioSystem`) whose method tables are in the data section and are **not** in this
> export — so per-manager magnitudes (roster caps, spawn tables, cooldown seconds) are **walled open**, not
> invented. Every claim below cites the `FUN_`/`DAT_` it was read from.

---

## Overview

Above the physics body, a JC4 vehicle is administered by three **world singletons** registered into the global
manager registry: **`CVehicleDataManager`** (the vehicle-type/data registry — the "catalog"),
**`CPilotDataManager`**, and **`CRadioSystem`**. Two distinct things share the word "driver/pilot" and must
**not** be conflated:

1. **Pilots** (`CPilotData` / `CPilotDataManager` / `SPilotSaveData`) are the **supply-drop / rebel-drop
   pilots** — the roster of air-delivery pilots the player unlocks and selects in the Rebel-Drop UI. This is a
   *progression + UI + save* system, **not** an AI that flies a vehicle (proven — every string in the
   `CPilotData` cluster is supply-drop / fast-travel / pilot-select UI, below).
2. **AI drivers** — the NPCs that actually pilot/drive vehicles — are **behavior-tree + animation-graph**
   constructs (`airplane_driver`, `helicopter_driver`, …, `S_IDLE_DRIVER_VEHICLE`), *not* a `CPilotData`
   profile. No "pilot skill / aggression" numeric profile struct was found in the functions export (open
   question, below).

Scripted missions puppeteer a specific vehicle through **`CObjectiveVehicleController`** (a reflected entity
component) and its per-seat **`CObjectiveVehicleControllerSeat`**. Seats/passengers are described by a
`max_passengers` behavior property, a `Vehicle Seat Component`, seat-switch action/anim states, and
`CConditional_HasPassenger(s)`. The in-car radio is the `CVehicleRadio` component fed by the `CRadioSystem`
manager and `CRadio` / `CRadioStation` reflected types.

---

## Key classes & functions

### World managers (global service-locator registry)

Registered by the manager-list builder — the same contiguous list appears in **`FUN_148f960c0`** (size 4805,
lines ~4139177–4139206) and its twin **`FUN_146cd0bea`** (via `FUN_149b2f323`, lines ~3209695–3209705). Each
manager is an 8-byte object holding only its vtable pointer, allocated by `thunk_FUN_1496a12b0(8)` and inserted
by `thunk_FUN_148f6e780(registry, "CName", obj)`. (proven)

| Manager string | vtable (data-section) | role | evidence |
|---|---|---|---|
| `CPilotDataManager` | `PTR_LAB_141d90de8` | supply-drop pilot roster owner | line 4139177 / 3209695 |
| `CVehicleDataManager` | `PTR_LAB_141d90e08` | vehicle-type/data catalog (registry) | line 4139182 / 3209700 |
| `CRadioSystem` | `PTR_LAB_141d909a8` | radio-station playback manager | line 3209520 / 4139002 |

In the list, `CPilotDataManager` and `CVehicleDataManager` are **adjacent**, sitting right after
`CLeaderboardManager` and before `CCreatureManager` (proven) — i.e. they are peers in the world/game-service
block, alongside `CDiscoveryManager`, `CBiomeManager`, `CSupplyDropManager`, `CSupplyRewardManager`. The
manager **bodies** are walled open (caveat above).

A typed accessor for `CPilotDataManager` exists: **`FUN_140938b70`** (callers `FUN_14093a060` /
`FUN_14093a280`) lazily interns the reflected type-id string
`"class CHashString __cdecl ArGetTypeId<class CPilotDataManager>(void)"` (len 0x44) into `DAT_142cba738`, then
**binary-searches a sorted type-id → value map** (`param_1` = map) for that id — the standard "get manager /
is-type-present by `ArGetTypeId<T>` atom" pattern (proven — `FUN_140938b70` @0x140938b70).

### Vehicle catalog (`CVehicleDataManager`)

`CVehicleDataManager` is the runtime registry/catalog for vehicle **data**. The vehicle **data component**
(`CVehicleData`), the abstract type atoms (`CAirVehicle` / `CLandVehicle` / `CSeaVehicle`) and the concrete
leaves (`CBoat`, `CCar`, `CDirigible`, `CHelicopter`, `CMotorBike`, `CTrain`) are the reflected types the
catalog administers — all of those registrars (`FUN_140815870`, `FUN_140c28xxx`, …) and atoms are documented
in **`vehicles.md`** and are not repeated here. Mission/behavior logic keys off vehicle **type** through two
reflected helpers registered next to the vehicle set (proven):

| String | Registrar FUN_ | atom DAT_ | role (inferred) |
|---|---|---|---|
| `CConditional_IsPlayerInVehicleType` | `FUN_140f27f60(…,0x22,…)` @619416 | `DAT_142cb5d68` | branch on the *type* of vehicle the player occupies |
| `CObjectTrackerFilterVehicleType` | `FUN_140f27f60(…,0x1f,…)` @866939 | `DAT_142cb9978` | filter tracked/HUD objects by vehicle type |
| `CConditional_IsPlayerInVehicle` | `FUN_140f27f60(…,0x1e,…)` @619391 | `DAT_142cb5d40` | branch on "player is in *a* vehicle" |

These prove the catalog exposes a **vehicle-type taxonomy** consumable by the objective/tracker layer.
(proven strings / inferred roles)

### Pilots — the supply-drop / rebel-drop roster (`CPilotData` family)

| String | Registrar / site FUN_ | atom / DAT_ | role |
|---|---|---|---|
| `CPilotData` (reflected type, len 0xa) | `FUN_1407db2d0` @0x1407db2d0 | `DAT_142cb8dd8` | a single pilot's data record (proven) |
| `CPilotDataManager` | manager list (above) | `PTR_LAB_141d90de8` | owns the roster (proven) |
| `SPilotSaveData` (save struct) | `ArGetTypeId<struct SPilotSaveData>` len 0x42 @1038931,1045668,1047025,1047645 | — | per-pilot **persisted** save record (proven) |
| `std::vector<SPilotSaveData>` | `ArGetTypeId<…vector<…SPilotSaveData…>>` @1038894,1046116,1047002,1047629 | — | the **saved roster** (a vector of pilot save records) (proven) |
| `pilot_data` (property) | `FUN_14093c3d0` @0x14093c3d0 (`FUN_140b6cef0(…,"pilot_data",3,…)`) | `PTR_LAB_141dbce68/…cf60` | reflected property object; default value `0x3f800000` (=1.0f) at +0x1b (proven) |
| `pilot_busy_` (reservation) | `FUN_140959510` @0x140959510 | busy timer at obj+0x24 | see below (proven) |
| `supply_drop_selected_pilot` | `FUN_140f27f60(…,0x1a)` @1645104 | property atom | the currently selected pilot index (proven) |
| `CPilotSelectUI` / `ShowPilotSelect` | `FUN_140f27f60("ShowPilotSelect",0xf)` @1645159,1690131,1724932 | — | the Rebel-Drop pilot-select screen + its show event (proven) |
| `ach_all_pilots` | `FUN_140f27f60(…,0xe)` @1069386 | `DAT_142cba6cc` | "unlock all pilots" achievement hook (proven) |

**Pilot behavior-VM conditionals** (registered into the conditional table via `thunk_FUN_147cafcd0`, same table
as quest conditionals — proven at the cited lines):

| Condition string | Registrar FUN_ | atom DAT_ | meaning (inferred from name) |
|---|---|---|---|
| `CConditional_HasPilotUnlocked` | `FUN_140f27f60(…,0x1d)` @605664 | `DAT_142cb5a88` | pilot roster entry is unlocked |
| `CConditional_IsPilotReady` | `FUN_140f27f60(…,0x19)` @619215 | `DAT_142cb5d28` | pilot available (not on cooldown/busy) |
| `CConditional_IsPilotSelectedInUI` | `FUN_140f27f60(…,0x20)` @619266 | `DAT_142cb5d38` | pilot highlighted in the select UI |
| `CConditional_IsPilotSelectedInFastTravelUI` | (paired w/ above) | — | pilot chosen in fast-travel UI |
| `CConditional_IsSupplyEquippedByPilot` | `FUN_140f27f60(…,0x24)` @627653 | `DAT_142cb5f08` | the supply/loadout a pilot carries |

**Pilot location resolvers** (spawn/arrival point selection — proven):
`CLocationResolverPilot` (`FUN_140f27f60(…,0x16)` @865115, `DAT_142cb9bc8`) and
`CLocationResolverFastTravelPilot` (`FUN_140f27f60(…,0x20)` @865051, `DAT_142cb9bc0`) — these resolve *where* a
pilot delivers / where fast-travel-by-pilot drops the player.

### Scripted-mission vehicle control (`CObjectiveVehicleController`)

| String | Registrar FUN_ | atom / vtable | role |
|---|---|---|---|
| `CObjectiveVehicleController` (component) | `FUN_140811050` @0x140811050 (called from `FUN_14085fd00`) | vtable `PTR_LAB_141d9c060`, atom `DAT_142cb97dc` | the mission-owned vehicle-control component (proven registration) |
| `CObjectiveVehicleController` (reflected type) | `FUN_140947d30` @0x140947d30 | `DAT_142cb97dc` | same atom, standalone type registrar (proven) |
| `CObjectiveVehicleControllerSeat` | `FUN_140947dc0` @0x140947dc0 | `DAT_142cba6b0` (len 0x1f) | per-seat sub-object the controller drives (proven) |

`FUN_140811050` is the component registrar: it allocates a 0x10-byte vtable holder, points it at
`PTR_LAB_141d9c060`, lazily interns `"CObjectiveVehicleController"` (len 0x1b) into `DAT_142cb97dc`, and inserts
`{atom, holder}` into the component registry via `thunk_FUN_14cfef490` — the identical registration pattern as
the `CVehicle*` components in `vehicles.md` (proven). This makes `CObjectiveVehicleController` an
**entity component** a mission can attach to a vehicle to puppeteer it; the `Seat` variant addresses individual
seats. Its runtime steering/goal logic is behind the vtable (walled open). Coordinate with
`objectives_operations.md` for the mission side that drives it.

### Seats & passengers

| String | Site FUN_ | atom | role |
|---|---|---|---|
| `max_passengers` | `FUN_142ebdca0` @0x142ebdca0 (`thunk_FUN_14aadec10`) | `_DAT_142cb0dd4` | seat-count behavior property (proven) |
| `ACT_SWITCH_SEAT` | `FUN_140f27f60("ACT_SWITCH_SEAT",0xf)` @3087974 | `_DAT_142cb32d0` | in-vehicle seat-switch action atom (proven) |
| `Vehicle Seat Component` | vehicle-assembly `FUN_140c53…` @1466903,1467139 | stored at `vehicle+0xf8` | the seat component resolved during assembly (proven) |
| `CConditional_HasPassenger` | `FUN_140f27f60(…,0x19)` @605614 | `DAT_142cb5a78` | vehicle has ≥1 passenger (proven) |
| `CConditional_HasPassengers` | `FUN_140f27f60(…,0x1a)` @605639 | `DAT_142cb5a80` | plural variant (proven) |
| `ply.driverseat_entered` | `thunk_FUN_147625370(…)` @1458520 | event | fired when the driver seat is taken (proven) |
| `vehicle_select_left_seat_context` / `_right_seat_context` | string table | — | entry-side seat selection contexts (proven strings) |

The seat is wired during the same assembly routine that resolves the winch/telescopic sockets (`vehicles.md`):
`Vehicle Seat Component` → `vehicle+0xf8`, and a `"vehicle_common"` (len 0xe) asset is loaded via
`FUN_140fdc0c0(DAT_142cba830, …)` into `vehicle+0xb0` (proven — @1466903–1466912). Anim-graph seat/role states
form a matrix: `S_IDLE_DRIVER_VEHICLE`, `S_IDLE_PASSENGER_VEHICLE`, `S_SWITCHING_SEAT_TO_DRIVER`,
`S_DRIVER_TO_PASSENGER`, `S_PASSENGER_TO_DRIVER`, `S_NEUTRAL_TO_DRIVER`, `S_PASS1_TO_DRIVER_VEHICLE`,
`S_REVERSE_DRIVER_VEHICLE`, `S_STUNT_TO_DRIVER_VEHICLE` (proven strings) — i.e. seat occupancy and role
transitions are animation-graph states gated by `ACT_SWITCH_SEAT`. (inferred flow)

### Vehicle radio

| String | Registrar FUN_ | atom / vtable | role |
|---|---|---|---|
| `CVehicleRadio` (component) | `FUN_1408159f0` @0x1408159f0 (from `FUN_14085fd00`) | vtable `PTR_LAB_141d9e248`, atom `DAT_142cb9b48` | per-vehicle radio component (proven; see `vehicles.md`) |
| `CRadio` (reflected type, len 6) | `FUN_140f27f60("CRadio",6,…)` @867675 | `DAT_142cb9b40` | radio object (proven) |
| `CRadioStation` (len 0xd) | `FUN_140f27f60("CRadioStation",0xd,…)` @198747,867707 | `DAT_142cb0bc8` | a station definition (proven) |
| `CRadioSystem` | manager list @3209520,4139002 | `PTR_LAB_141d909a8` | world radio manager (proven) |
| `CConditional_IsRadioStationActive` | `FUN_140f27f60(…,0x21)` @627477 | `DAT_142cb5ec4` | behavior branch on active station (proven) |

`SoundCarRadioVolume` is **not** the radio component — it is a **user audio setting** serialized in the
options blob at `options+0x148` alongside `SoundGameplayVolume`/`SoundMusicVolume`/`SoundVoiceVolume`
(proven — `FUN_140b4e210(…,"SoundCarRadioVolume", *(param_1+0x148))` @1392108; also read at 1377099/1377698).
Note it as the volume control that feeds the `CVehicleRadio`/`CRadioSystem` playback, not part of the vehicle
data. (inferred link)

**Horn:** no vehicle-horn action/string was found — a case-insensitive `horn` search returns only credits
names (`Michael Horning`, `Maximilian Horn`). If a horn exists it is not exposed as a named `ACT_`/class in the
functions export. (proven-absent)

### Livery / paint (workshop tie)

There is no `CVehiclePaint`/`CLivery` *data* class in the export; vehicle paint is a **render-material** family,
the `CarPaint` render blocks: `CarPaint`, `CarPaint_Skinned`, `CarPaint_Deform`, `CarPaintOutline`,
`CarPaintShadow`, `CarPaintPreZ*`, `CarPaintCloaked`/`…CloakTransition`, and the constant block
`CRenderBlockCarPaint_StaticMaterialConstants` (proven strings). Livery selection is therefore expressed as
material/constant swaps on the model, which is the workshop **CarPaint** path (memory: workshop model/material
work). This doc records the string family; the render-block decode belongs with rendering/workshop notes.
(proven strings / inferred meaning)

---

## How it works (from the decomp)

### Manager startup
The manager-list builder (`FUN_148f960c0` / mirror `FUN_146cd0bea`) constructs each world singleton as an
8-byte vtable holder and registers it by class-name string into the service registry
(`thunk_FUN_148f6e780`). `CVehicleDataManager`, `CPilotDataManager` and `CRadioSystem` are created here; their
method bodies dispatch through the data-section vtables (`PTR_LAB_141d90e08 / …de8 / …909a8`). Lookups use the
`ArGetTypeId<T>` binary-search pattern (`FUN_140938b70` for `CPilotDataManager`). (proven skeleton; bodies
walled open)

### Pilot roster: save, select, reserve
- **Persistence.** The roster is a `std::vector<SPilotSaveData>` (proven type-id strings) — a growable list of
  per-pilot save records. Unlock state is queried by `CConditional_HasPilotUnlocked`. (proven)
- **Selection.** The player picks a pilot in `CPilotSelectUI`; the UI code (`FUN_…` @1645104–1645159) reads the
  `supply_drop_selected_pilot` property from the game-state object (`*(DAT_142cb7dc8 + 0x228)` is the selected
  index it watches for change), and when the select-UI state is absent it fires the `ShowPilotSelect` event
  (`FUN_140f27f60("ShowPilotSelect",0xf)`). `CConditional_IsPilotSelectedInUI` /
  `…IsPilotSelectedInFastTravelUI` gate on the highlighted entry. (proven)
- **Busy / cooldown reservation.** `FUN_140959510(obj, float busyTime)` implements a pilot-busy lock: the busy
  timer lives at `obj+0x24`; when a new busy period starts (current `+0x24 == 0` and `busyTime > 0`) it builds
  a reservation key string **`"pilot_busy_"` + a random suffix** (LCG `DAT_142c73a98 = DAT_142c73a98*0x343fd +
  0x269ec3`, formatted via `thunk_FUN_147a04180`) and passes it to a virtual (`vtable+0x20`) — i.e. each
  in-flight supply-drop pilot is tagged with a unique `pilot_busy_<rand>` reservation until its timer elapses.
  `CConditional_IsPilotReady` is the readiness gate that this busy state feeds. (proven mechanism; "supply-drop
  delivery" role inferred from the surrounding supply-drop strings)
- **Delivery location.** `CLocationResolverPilot` / `CLocationResolverFastTravelPilot` resolve the arrival/drop
  point (proven registrars). Ties to `CSupplyDropManager` / `CSupplyRewardManager` in the same manager block.
  (inferred)

### Objective vehicle control
A mission attaches `CObjectiveVehicleController` (component, vtable `PTR_LAB_141d9c060`) to a vehicle entity and
addresses seats through `CObjectiveVehicleControllerSeat`. Registration is the standard component pattern
(`FUN_140811050` → `thunk_FUN_14cfef490` insert). The controller's steering/waypoint logic dispatches through
its vtable (walled open). This is how scripted missions "drive" a vehicle without a player/AI occupant; the
driving *forces* it commands are still the physics-force registry in `vehicles.md`. (proven registration;
inferred behavior)

### Seat occupancy
Seat count is the `max_passengers` behavior property (`_DAT_142cb0dd4`); the seat component sits at
`vehicle+0xf8`. Entering the driver seat fires `ply.driverseat_entered`; switching seats is `ACT_SWITCH_SEAT`
driving the `S_*_DRIVER/PASSENGER` anim-graph state matrix. Passenger presence is queried by
`CConditional_HasPassenger(s)`. (proven)

---

## Data & config integration

- `CVehicleData` / `CObjectiveVehicleController` / `CVehicleRadio` / `CPilotData` are **reflected entity
  components/types** — instantiated from RTPC entity components + ADF config at assembly time (memory
  `[[rtpc-entity-assembly]]`, `[[composite-assets]]`). The catalog/manager holds them at runtime. (proven types
  / inferred data path)
- Pilot roster state is **saved** (`SPilotSaveData` / `std::vector<SPilotSaveData>`) — it is progression, not
  static config. Cross-reference `progression_economy.md` / `missions_progression.md` (`CSupplyManager`,
  `CSupplyDropManager`, `CSupplyRewardManager`). (proven)
- `max_passengers` / `direction_count` are behavior-VM properties (`thunk_FUN_14aadec10`) set from the
  behavior/RTPC layer, i.e. per-vehicle-type data. (proven)
- Radio stations (`CRadioStation`) are reflected types configured from data and played by `CRadioSystem`;
  `SoundCarRadioVolume` is a persisted user setting. (proven)

---

## Notable constants / IDs (read straight from the decomp)

| Constant / ID | Value | Where | Meaning |
|---|---|---|---|
| `pilot_data` default | `0x3f800000` (=1.0f) at `+0x1b` | `FUN_14093c3d0` | default value of the `pilot_data` property |
| pilot-busy RNG | `x = x*0x343fd + 0x269ec3` | `FUN_140959510` @1067283 | LCG generating the `pilot_busy_<rand>` reservation suffix |
| pilot busy-timer slot | `obj+0x24` (float) | `FUN_140959510` | 0 = ready; >0 = counting down |
| `ArGetTypeId<CPilotDataManager>` | interned, len 0x44 → `DAT_142cba738` | `FUN_140938b70` | manager lookup key |
| `ArGetTypeId<SPilotSaveData>` | len 0x42 | @1038931 etc. | save-record type key |
| `CObjectiveVehicleController` atom | `DAT_142cb97dc` (name len 0x1b) | `FUN_140811050` | component type id |
| `CObjectiveVehicleControllerSeat` atom | `DAT_142cba6b0` (len 0x1f) | `FUN_140947dc0` | seat sub-object type id |
| `CVehicleRadio` atom | `DAT_142cb9b48` | `FUN_1408159f0` | radio component id |
| `CRadio` / `CRadioStation` atoms | `DAT_142cb9b40` / `DAT_142cb0bc8` | @867675 / @198747 | radio reflected types |
| seat component slot | `vehicle+0xf8` | @1466903 | `Vehicle Seat Component` |
| `vehicle_common` asset | len 0xe → `vehicle+0xb0` | @1466908 | common vehicle asset loaded at assembly |
| user radio-volume slot | `options+0x148` | @1392108 | `SoundCarRadioVolume` setting |

No numeric pilot **skill/aggression** or roster-cap constants were present in the functions export — they, and
the spawn tables, are behind the manager vtables (walled open).

---

## Call-graph highlights

- `FUN_148f960c0` / `FUN_146cd0bea` → `thunk_FUN_148f6e780(reg, "CVehicleDataManager"|"CPilotDataManager"|"CRadioSystem", obj)` — manager registration.
- `FUN_14093a060` / `FUN_14093a280` → `FUN_140938b70` — `CPilotDataManager` type-id lookup.
- `FUN_14085fd00` → `FUN_140811050` — registers the `CObjectiveVehicleController` component (same dispatcher that registers the `CVehicle*` components).
- `FUN_14085fd00` → `FUN_1408159f0` — registers the `CVehicleRadio` component.
- pilot-select UI (`FUN_…@1645104`) → reads `supply_drop_selected_pilot`, fires `ShowPilotSelect`.
- `FUN_140959510` (busy lock) callers: `FUN_1493381d0`, `FUN_1493224d0`, `FUN_14095c920`, `FUN_1493209f0`, `FUN_1492ddb20` — the sites that reserve a pilot.

---

## Open questions / lower-confidence

- **AI driver profiles.** The NPCs that fly/drive vehicles use behavior-tree templates (`airplane_driver`,
  `helicopter_driver`, `motorbike_driver`, `dirigible_driver`, `submarine_driver`, `train_driver`,
  `sea_vehicle_driver`, `vehicle_land_driver`, `safe_driver`) and anim-graph states — **not** a `CPilotData`
  record. No numeric "skill/aggression" struct surfaced; where those templates' tunables live (behavior graph
  data vs. ADF) is unresolved. Cross-reference `ai_combat_encounters.md` / `behavior_system.md`. (open)
- **Manager bodies walled open.** `CVehicleDataManager` spawn/lookup logic, `CPilotDataManager` roster caps and
  cooldown seconds, and `CRadioSystem` station scheduling are all behind data-section vtables not in the
  functions export. (open — needs a data-section export or live debugger read)
- **`CObjectiveVehicleController` steering.** Registration is proven; the actual waypoint/goal execution is
  vtable-dispatched and not recovered. Pair with `objectives_operations.md`. (open)
- **Livery pipeline.** `CarPaint` is confirmed as the render-material family, but the *data* that selects a
  livery per vehicle (workshop CarPaint) was not traced here. (open — workshop/rendering note)
- **`pilot_data` property default 1.0f** — semantics (readiness fraction? a scalar multiplier?) unconfirmed.
  (speculative)

# Grappling hook / tether / reel / winch — JC4's signature traversal & combat device

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)
Source: `output/_ghidra_jc4/jc4_all_functions_decomp.txt` (headless Ghidra decomp of `JustCause4.exe`).

## Overview

The grappling hook is implemented as a **player equipment item** — the decomp preserves its source path
`d:\jenkins\jc4-rtm_pc_patch-source\source\project\game\equipment\item\grapplingdevice\grapplinghook.cpp`
(proven, FUN_1430518a0 @0x1430518a0). It is not one class but a small cluster: the device/controller
(`CGrapplingHook`), a projectile prop (`CGrapplingHookPropData`), the visible cable (`CGrapplingHookWire`),
and the attach target descriptor (`CGrapplePoint`). It runs in **four distinct modes**, each registered as
its own resource/allocation category: **default**, **dualtether**, **reelingin**, and **vehicle**
(proven, FUN_1430518a0). On top of the device sits a large **animation-graph action/blend-state layer**
(the `ACT_GRAPPLE_*`, `S_GRAPPLE_*`, `ACT_SIMPLE_GRAPPLE_*`, `S_REELED_*` families) and a **behavior-condition
layer** (`CReelDistanceCondition`, `CTetherTensionCondition`, …) that gates the animation transitions.
Tether/winch *state* is also exposed to mission/entity scripting through a set of RTPC conditionals
(`CConditional_TetherState`, `CConditional_DeployedTetherCount`, `CConditional_IsObjectConnectedToNumWinches`,
…). Physical pull is applied through named **physics force channels** — `GrapplingHook`, `ReelInKick`,
`WinchSuckerDamping`, `AirBuoyancyExtraWhenDualTethered` (proven, FUN_147759a20). Vehicle-mounted tethering
(seat/turret winch, "suck" beam) is a separate subsystem: the per-vehicle `CVehicleWinchController` component
and the global `CWinchControllerSystem` engine system.

## Key classes & functions

| Name (string) | FUN_ / DAT_ | Role |
|---|---|---|
| `CGrapplingHook` | type ctor FUN_1406b7570 @0x1406b7570 → `DAT_142cb8108` | Grapple device class type descriptor (name len 0xe). (proven) |
| `CGrapplingHookPropData` | FUN_1406b7600 @0x1406b7600 → `DAT_142cb80e0` | Fired hook projectile/prop data. (proven) |
| `CGrapplingHookWire` | FUN_1406b7690 @0x1406b7690 → `DAT_142cb8100` | The visible tether cable/rope object. (proven) |
| `CGrapplePoint` | FUN_14041a700 @0x14041a700 → `DAT_142cb22a4` | Attach-target descriptor (name len 0xd). (proven) |
| grapple mode tags | FUN_1430518a0 @0x1430518a0 | Registers `grapplinghook_default`/`_dualtether`/`_reelingin`/`_vehicle` → `DAT_142cb7ed0/e98/f40/f78`, all tagged `"CGrapplingHook"`. (proven) |
| Grapple/tether controller ctor | FUN_140aeb2d0 @0x140aeb2d0 (caller FUN_140ae98d0) | Player-side grapple state object: 5 tether-slot arrays (cap 10), `LastValidGrapple` ray, max-dist 100.0. (proven mechanism; inferred it's the CPlayer grapple controller) |
| Player command host | FUN_140ae98d0 @0x140ae98d0 | Constructs the grapple controller; registers `ply.grapple.enable`/`ply.grapple.disable`. (proven) |
| `CVehicleWinchController` | type ctor FUN_140815c30 @0x140815c30 → `DAT_142cb9460` (vtable `PTR_LAB_141d9c560`) | Per-vehicle winch component (name len 0x17). (proven) |
| Vehicle winch controller ctor | FUN_140c19fa0 @0x140c19fa0 (caller FUN_14083d870) | Builds `WINCH_SUCK_RAY` + `WINCH_AIM_RAY`, sets winch tunables. (proven) |
| `vehicle_winch_component` | instantiated in FUN_140c544e0 @0x140c544e0 | Resource-graph component attached to a vehicle seat (turret winch). (proven) |
| `CWinchControllerSystem` | engine-system entry in FUN_148f958e0 @0x148f958e0 (factory `PTR_LAB_141d903a8`, order 0x41d90058) | Global system that updates all vehicle winches. (proven registration; inferred role) |
| Force-channel registry | FUN_147759a20 @0x147759a20 | Registers force-type names incl. `GrapplingHook`, `ReelInKick`, `WinchSuckerDamping`, `AirBuoyancyExtraWhenDualTethered`. (proven) |
| Behavior-condition registry | FUN_1485e07e0, FUN_1485b9730 (callers of every `C*Condition` id accessor) | Master tables that register all reel/tether animation-gate conditions. (proven) |
| RTPC conditional registry | FUN_148f99a90 @0x148f99a90 | Registers scripting `CConditional_*` tether/winch/reel nodes. (proven) |

### Behavior-condition id accessors (animation-graph gates)

Each is a thread-safe lazy `int` type-id accessor `id = thunk_FUN_14aadec10("<Name>")` — i.e. the conditions
are **data-driven behavior-graph nodes**, not hand-coded branches; the id is looked up when the graph asset is
parsed. (proven — pattern identical across all of them)

| Condition | accessor FUN_ | DAT_ id | Gates |
|---|---|---|---|
| `CIsGrappleActiveCondition` | FUN @503831 (FUN_140549d10) | `DAT_142cb4194` | grapple currently active |
| `CIsTetheredCondition` | @504144 | `DAT_142cb417c` | player is tethered |
| `CTetherTensionCondition` | @505683 | `DAT_142cb4184` | tether under tension (taut) |
| `CCombatantNotTetherableCondition` | @501978 | `DAT_142cb418c` | target may not be tethered |
| `CDualTetheringInputCondition` | @502453 | `DAT_142cb452c` | second-tether input held |
| `CReelDirectionCondition` | FUN_14054be10 @0x14054be10 | `DAT_142cb43e4` | reel direction (in/out) |
| `CReelDistanceCondition` | FUN_14054be90 @0x14054be90 | `DAT_142cb43ec` | reel distance threshold |
| `CReelLandCondition` | FUN_14054bf90 @0x14054bf90 | `DAT_142cb415c` | reel-in landing |
| `CReelInBlendState` | FUN_14054bf10 @0x14054bf10 | `DAT_142cb3ff4` | "reeling in" anim blend state |
| `CReeledInBlendState` | FUN_14054c010 @0x14054c010 | `DAT_142cb3f8c` | "reeled in / attached" anim blend state |
| `COriginalReelVerticalDirCondition` | @504657 | `DAT_142cb41dc` | vertical dir at reel start |
| `CAngleBetweenGrappleAnchorAndLookCombatTarget` | @501674 | `DAT_142cb455c` | anchor-vs-look-target angle (combat aim assist) |

## How it works (from the decomp)

### Attach / fire
The device is an equipment item (grapplinghook.cpp). Firing goes through the animation-action layer:
`ACT_GRAPPLE_FIRE` / `ACT_SIMPLE_GRAPPLE_FIRE` / `ACT_GRAPPLE_TETHER_FIRE` (registered as name-ids via
FUN_140f27f60, e.g. `ACT_GRAPPLE_TETHER_FIRE` @0x142f21e40/line 3082299, `ACT_SIMPLE_GRAPPLE_REEL` @3103364).
A hook prop (`CGrapplingHookPropData`) travels out and, on contact, resolves a `CGrapplePoint`; a
`CGrapplingHookWire` is spawned as the visible cable. Attach animation variants exist per approach angle
(`S_GRAPPLE_ATTACH_0/120L/120R`, `_UPSIDE_DOWN`, grenade-throw variants). (proven strings; attach flow inferred
from the action/state naming — the exact projectile trace fn was not isolated)

### The grapple/tether controller state (FUN_140aeb2d0 @0x140aeb2d0) — proven
The controller object is built with **five parallel dynamic arrays**, each reserved to capacity 10
(`FUN_1400965e0(slot,10)` on `param_1+0x4/+0x7/+0xa/+0xd/+0x10`). These are the tether/anchor slot lists —
consistent with multi-tether ("dual tether" and beyond; see `CConditional_DeployedTetherCount`). Anchor
positions are initialised to the `0xdeadbeef` sentinel (`param_1+0x2fc..0x314`), i.e. "no anchor yet". Key
initial fields:
- `+0x3c0` (`param_1[0x78]`) = **100.0** (`0x42c80000`) — inferred **max grapple/tether distance**.
- `+0x3e0` = **-1.0** (`0xbf800000`), `+0x3e4` = **1.0** (`0x3f800000`) — a normalized direction / range pair
  (inferred reel vertical-dir bounds).
- `+0x3ec` = **2** — count/mode (inferred).
- Five id handles from `thunk_FUN_14aae5cc0()` stored at `param_1[0x13..0x17]` — inferred per-slot tether ids.
- A `LastValidGrapple` ray object at `param_1+0x86` (`FUN_1400d07c0(...,"LastValidGrapple")`) — the last valid
  aim/attach ray, reused when the current aim is invalid. (proven)

### Reeling in — proven (conditions) / inferred (math)
Reel-in is a distinct grapple mode (`grapplinghook_reelingin`, `DAT_142cb7f40`). It is driven by the
animation graph: transitions in/out of `CReelInBlendState` → `CReeledInBlendState` are gated by
`CReelDistanceCondition` (are we close enough to the anchor yet), `CReelDirectionCondition` (reel in vs. out),
and `CReelLandCondition` (did we arrive at a surface → pick a land anim). The state families
`S_ON_GROUND_PRE_REEL_FAST/SLOW`, `S_IN_AIR_PRE_REEL`, `ACT_PRE_REEL`, `S_GRAPPLE_REEL_IN`,
`ACT_SIMPLE_GRAPPLE_REEL`, and the landing set `ACT_SIMPLE_GRAPPLE_REEL_LAND_{FLOOR,WALL,CEILING,LEDGE,…}`
enumerate the reel-in animation outcomes (proven strings). The physical pull is applied via the **`ReelInKick`**
force channel (proven, FUN_147759a20); `reel_distance` and `reeled_in_kill` appear as named data/blackboard
members (property tag 0xd/0xe, FUN @4276204/4276282 area). Player input feeds `player_reeling_in_input`
(FUN_140b149f0 @0x140b149f0). NPCs have their own reel-in tasks (`NInputReelingInTask`,
`NStateTask_InputReeledInTask`, `ReelInKickTask`, blackboard `BB_NPC_GRAPPLE_TASK_LAST_REELING_TIME`).
(The precise force magnitude / spring model was not located — force is applied through the generic force-channel
system, so the constant lives in per-item config data, not in code.) (inferred)

### Dual / multi tether — proven (existence), inferred (mechanism)
A second tether is armed when `CDualTetheringInputCondition` passes (second-tether input held). The
`grapplinghook_dualtether` mode (`DAT_142cb7e98`) applies. The controller's 5 slot arrays (above) hold the
multiple tether endpoints. Two objects joined by a tether are pulled together; scripting can query
`CConditional_DeployedTetherCount` (how many tethers are out), `CConditional_IsTetheredToObject`,
`CConditional_TetherConnection`, and `CConditional_TetherState`/`_Simple`. A physics tunable
**`AirBuoyancyExtraWhenDualTethered`** (proven, FUN_147759a20) adds extra air buoyancy while dual-tethered —
i.e. dual-tethered objects float/lift more (matches the "balloon two things together and yank" gameplay).
`tether.force.detach` (FUN_148b2d140 @0x148b2d140, event `param_1+0x668`) is the detach event; `RetractTether`
and `SetRetoolerTetherCount` (FUN @1588851 / @1729417) manage retraction / configurable tether counts
(the "Retooler" grapple upgrade). (proven strings; join-constraint mechanism inferred)

### Tether tension → combat / hitreacts — inferred
`CTetherTensionCondition` fires when a tether is taut. Taut tethers on a combatant drive the reel-kick hit
reactions: `ACT_HITREACT_REEL_KICK_{0,90L,90R,180}`, `ACT_REEL_KICKED`, and gameplay events
`on.character.grappled` / `on.character.reelkicked` / `reeled_in_kill`. `CCombatantNotTetherableCondition`
blacklists certain targets. (proven strings; the tension threshold value was not isolated in code — data-driven)

### Vehicle winch (seat/turret "sucker" beam) — proven
Distinct from the hand grapple. Per-vehicle it is the `CVehicleWinchController` component (type ctor
FUN_140815c30; constructed by FUN_140c19fa0 via FUN_14083d870), instantiated onto a seat from the resource
`vehicle_winch_component` (FUN_140c544e0 @0x140c544e0; see also `Vehicle Seat Winch`, `S_TURRET_TETHERED`,
`ACT_TURRET_TETHERED`). The controller builds two rays:
- **`WINCH_SUCK_RAY`** (`param_1+0x85`, FUN_1400d07c0) — the pull/"suck-in" beam that draws a hooked object toward
  the vehicle.
- **`WINCH_AIM_RAY`** (`param_1+300`, FUN_1400cb810) — the aim/targeting ray.
Winch tunables set in the ctor (proven raw values; labels inferred):
`+0x2e4 = 30.0`, `+0x34c/+0x350 = 5.0`, `+0x354/+0x358 = 2.0`, `+0x374 = 10000.0` (large — inferred max ray
length or force cap), `+0xa78 (param_1[0x14f]) = -0.1`, sentinel `+0x37c = 0xdeadbeef`. `WinchSuckerDamping`
is the damping force channel applied while sucking (FUN_147759a20). The global `CWinchControllerSystem` engine
system (FUN_148f958e0) ticks winch controllers each frame; scripting queries
`CConditional_IsObjectConnectedToNumWinches` and `CConditional_IsReeledOnObject`. (proven)

## Data & config integration

- **Grapple is an equipment item.** Source path `…\equipment\item\grapplingdevice\grapplinghook.cpp` (proven).
  The four modes are separate config/allocation categories: `grapplinghook_default` / `_dualtether` /
  `_reelingin` / `_vehicle` (`DAT_142cb7ed0/e98/f40/f78`, FUN_1430518a0). A `grapplinghook_default_retract`
  variant and `grappling_hook_wire` / `GrapplingHookWire` / `GrapplingHookWireEnd` render assets also appear.
- **Named data members / blackboard.** `reel_distance` (tag 0xd), `reeled_in`/`reeled_in_kill` (tag 0xe),
  `LastValidGrapple`, `player_reeling_in_input`, `BB_NPC_GRAPPLE_TASK_LAST_REELING_TIME` — ADF/RTPC/blackboard
  fields read by the runtime (FUN_140b149f0, FUN @4276204+). Tunables like tether tension, reel force, and max
  distance are **data-driven** — applied through named force channels, so the numeric values live in item/RTPC
  config rather than in the executable. (proven the channels exist; inferred that values are data-side)
- **Physics force channels** (FUN_147759a20): `GrapplingHook`, `ReelInKick`, `WinchSuckerDamping`,
  `AirBuoyancyExtraWhenDualTethered` are entries in the same global force-type registry as vehicle/aero forces —
  the grapple applies its pull as just another named force on a rigid body. (proven)
- **Behavior graph.** Reel/tether conditions and blend states (`CReel*`, `CTether*`, `CIs*`) are behavior-graph
  node types registered by name (FUN_1485e07e0 / FUN_1485b9730); the animation-graph asset references them by id.
- **Mission/entity scripting** sees tether/winch state via RTPC `CConditional_*` nodes (FUN_148f99a90):
  `TetherState`, `TetherState_Simple`, `TetherConnection`, `IsTetheredToObject`, `IsReeledOnObject`,
  `DeployedTetherCount`, `IsObjectConnectedToNumWinches`. (proven)
- **Player toggles.** `ply.grapple.enable` / `ply.grapple.disable` script commands (FUN_140ae98d0). (proven)
- **Modifiers/upgrades.** `GRAPPLE_MODIFIER` (`_DAT_142cb391c`, FUN_143006280), `MOD_GRAPPLE_TETHER`,
  `MOD_GRAPPLE_TETHER_AND_NO_WEAPON_AIM`, `MOD_REELED_IN_ATTACHED(_HANG)`, `MOD_HOVERBOARD_REEL_SKI` — the
  upgrade/loadout modifier tags that switch grapple behavior. `SetRetoolerTetherCount` configures the Retooler
  upgrade's tether count. (proven strings)

## Notable constants / tunables

| Value (hex → float) | Location | Inferred meaning |
|---|---|---|
| `0x42c80000` = **100.0** | grapple ctrl FUN_140aeb2d0, `+0x3c0` | max grapple/tether distance |
| `0xbf800000` = **-1.0**, `0x3f800000` = **1.0** | grapple ctrl `+0x3e0/+0x3e4` | reel vertical-dir / normalized range |
| `0xdeadbeef` sentinels | grapple ctrl `+0x2fc…+0x314`; winch `+0x37c` | "no anchor / uninitialised" markers (proven) |
| `0x41f00000` = **30.0** | winch ctor FUN_140c19fa0, `+0x2e4` | winch parameter (range/speed) |
| `0x40a00000` = **5.0** | winch `+0x34c/+0x350` | winch parameter |
| `0x40000000` = **2.0** | winch `+0x354/+0x358` | winch parameter |
| `0x461c4000` = **10000.0** | winch `+0x374` | winch max ray length / force cap |
| `0xbdcccccd` = **-0.1** | winch `param_1[0x14f]` (`+0xa78`) | winch bias/damping term |
| `0x3f000000` = **0.5** | vehicle_winch_component setup FUN_140c544e0, `+0x2e4` | component default |
| `arrays reserved to 10` | grapple ctrl (5×) | tether slot capacity per list (proven) |

(All raw values proven from the cited FUN_; the *labels* are inferred — field semantics aren't named in the
stripped decomp.)

## Call-graph highlights

- `FUN_140b1ba70` → `FUN_140ae98d0` (player command host) → `FUN_140aeb2d0` (grapple/tether controller ctor).
  (proven from `callers=[]`)
- `FUN_14085fd00` → `FUN_140815c30` (registers `CVehicleWinchController` type).
- `FUN_14083d870` → `FUN_140c19fa0` (winch controller ctor, builds `WINCH_SUCK_RAY`/`WINCH_AIM_RAY`).
- `FUN_140c544e0` (vehicle assembly; 6 callers incl. FUN_140c3b8c0/…/FUN_140c6e300) instantiates
  `vehicle_winch_component` and `vehicle_telescopic_component`.
- Every `C*Condition`/`CReel*BlendState` id accessor is called from **FUN_1485e07e0** and **FUN_1485b9730**
  (the behavior-condition registration tables) plus a third `0x14848…` builder.
- Every `CConditional_*` tether/winch node is registered from **FUN_148f99a90** (RTPC conditional registry).
- `FUN_148f958e0` registers `CWinchControllerSystem` as a global engine system (factory `PTR_LAB_141d903a8`).

## Open questions / lower-confidence

- **Exact reel force / spring model.** Not in code — applied via the `GrapplingHook`/`ReelInKick` force
  channels, so magnitudes live in item/RTPC config data. Need to decode the `grapplinghook_*` config resources
  (ADF/RTPC) to get real numbers. (inferred)
- **Tether-join constraint.** How two tethered bodies are physically linked (distance constraint vs. force pair)
  wasn't isolated; `AirBuoyancyExtraWhenDualTethered` proves a buoyancy modifier but not the constraint itself.
- **Field semantics** in FUN_140aeb2d0 / FUN_140c19fa0 are inferred from value magnitude only (stripped struct).
- **The device's per-frame update fn** (the tick that walks the tether slots and applies force) was not pinned to
  a single FUN_ in this pass — the controller ctor and the force channels are proven; the update loop is the next
  dig (start from callers of the CGrapplingHook vtable slots / the `CWinchControllerSystem` tick).
- **`CAngleBetweenGrappleAnchorAndLookCombatTarget`** strongly implies aim-assist that biases the reel-kick toward
  the look-target; the angle threshold is data-driven and unconfirmed.

## Appendix — decomp anchors

Class type ctors: `CGrapplingHook` FUN_1406b7570→DAT_142cb8108; `CGrapplingHookPropData` FUN_1406b7600→DAT_142cb80e0;
`CGrapplingHookWire` FUN_1406b7690→DAT_142cb8100; `CGrapplePoint` FUN_14041a700→DAT_142cb22a4;
`CVehicleWinchController` FUN_140815c30→DAT_142cb9460 (vtable PTR_LAB_141d9c560).

Mode/allocation tags (FUN_1430518a0): `grapplinghook_default`=DAT_142cb7ed0, `_dualtether`=DAT_142cb7e98,
`_reelingin`=DAT_142cb7f40, `_vehicle`=DAT_142cb7f78; source path `…\equipment\item\grapplingdevice\grapplinghook.cpp`.

Controllers: grapple/tether ctrl FUN_140aeb2d0 (caller FUN_140ae98d0); winch ctrl ctor FUN_140c19fa0 (caller
FUN_14083d870); vehicle_winch_component assembly FUN_140c544e0; global CWinchControllerSystem FUN_148f958e0
(factory PTR_LAB_141d903a8, order 0x41d90058).

Rays/objects: `WINCH_SUCK_RAY` FUN_1400d07c0 @winch+0x85; `WINCH_AIM_RAY` FUN_1400cb810 @winch+300;
`LastValidGrapple` FUN_1400d07c0 @grapplectrl+0x86.

Force channels (FUN_147759a20): `GrapplingHook`, `ReelInKick`, `WinchSuckerDamping`,
`AirBuoyancyExtraWhenDualTethered`.

Behavior conditions (accessor→id, registry FUN_1485e07e0 / FUN_1485b9730):
`CIsGrappleActiveCondition`=DAT_142cb4194, `CIsTetheredCondition`=DAT_142cb417c,
`CTetherTensionCondition`=DAT_142cb4184, `CCombatantNotTetherableCondition`=DAT_142cb418c,
`CDualTetheringInputCondition`=DAT_142cb452c, `CReelDirectionCondition`=DAT_142cb43e4 (FUN_14054be10),
`CReelDistanceCondition`=DAT_142cb43ec (FUN_14054be90), `CReelLandCondition`=DAT_142cb415c (FUN_14054bf90),
`CReelInBlendState`=DAT_142cb3ff4 (FUN_14054bf10), `CReeledInBlendState`=DAT_142cb3f8c (FUN_14054c010),
`COriginalReelVerticalDirCondition`=DAT_142cb41dc,
`CAngleBetweenGrappleAnchorAndLookCombatTarget`=DAT_142cb455c.

RTPC conditionals (registry FUN_148f99a90): `CConditional_DeployedTetherCount`=DAT_142cb5a20,
`CConditional_IsObjectConnectedToNumWinches` (@618963), `CConditional_IsReeledOnObject`=DAT_142cb5ed8,
`CConditional_IsTetheredToObject`=DAT_142cb5f28, `CConditional_TetherConnection`=DAT_142cb5fb0,
`CConditional_TetherState`=DAT_142cb5fc0, `CConditional_TetherState_Simple`=DAT_142cb5fc8.

Actions (name-id via FUN_140f27f60): `ACT_GRAPPLE_TETHER_FIRE`=_DAT_142cb37a4 (@3082299),
`ACT_PULL_GRAPPLE`=_DAT_142cb3048, `ACT_SIMPLE_GRAPPLE_REEL`=_DAT_142cb5140 (@3103364); families
`ACT_GRAPPLE_*`, `ACT_SIMPLE_GRAPPLE_*`, `ACT_HITREACT_REEL_KICK_*`, `S_GRAPPLE_*`, `S_REELED_*`.

Events/blackboard: `tether.force.detach` (FUN_148b2d140 @grapple+0x668), `RetractTether` (@1588851),
`SetRetoolerTetherCount` (@1729417), `player_reeling_in_input` (FUN_140b149f0), `reel_distance`/`reeled_in_kill`
(FUN @4276204), `ply.grapple.enable`/`disable` (FUN_140ae98d0), `GRAPPLE_MODIFIER`=_DAT_142cb391c (FUN_143006280).
Modifier tags: `MOD_GRAPPLE_TETHER`, `MOD_GRAPPLE_TETHER_AND_NO_WEAPON_AIM`, `MOD_REELED_IN_ATTACHED(_HANG)`,
`MOD_HOVERBOARD_REEL_SKI`.

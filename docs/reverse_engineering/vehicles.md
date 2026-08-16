# Vehicles — driving, entry/exit, hijack, and the vehicle-side winch

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

All function/global addresses are from `output/_ghidra_jc4/jc4_all_functions_decomp.txt`
(162,115 functions). "proven" = read directly from the decomp; "inferred" = a reasonable reading of
structure/flow; "speculative" = plausible but unconfirmed. Model/mesh assembly of vehicles is documented
elsewhere (memory `[[model-amf]]`, `[[rtpc-entity-assembly]]`); the grapple/reel/tether internals of the
winch are covered in `grappling_hook.md` — this doc covers the **vehicle-side** controller and references
rather than duplicates them.

## Overview

A JC4 vehicle is an entity built from a fixed family of reflected C++ **component classes** (`CVehicleData`,
`CVehicleConfig`, `CVehiclePartProxy`, `CVehicleRadio`, `CVehicleRule`, `CVehicleCargoBay`,
`CVehicleActionVolume`, plus seat-mounted controllers `CVehicleTelescopicController` /
`CVehicleWinchController`), and is typed through a hierarchy `CAirVehicle` / `CLandVehicle` / `CSeaVehicle`
with concrete leaves (`CHelicopter`, `CBoat`, `CDirigible`, `CMotorBike`, `CTrain`, car). All of these are
registered by name into the engine's reflection/type registry at startup (proven — `FUN_14085fd00` dispatch,
below). Driving behaviour is data-driven physics: a large registry of ~120 named **force/impulse effects**
(wheel friction, suspension, helicopter rotor forces, motorbike lean torques, water drag, `WinchSuckerDamping`,
`VehiclePushAwayForce`, …) is applied to the vehicle rigid body (proven — `thunk_FUN_147728150` table).
Entry/exit/hijack are driven by the **behavior/animation-graph VM**: discrete `ACT_*` action atoms
(`ACT_ENTER_VEHICLE`, `ACT_EXIT_VEHICLE_HIJACK`, `ACT_CLOSE_LEFT_DOOR_FROM_INSIDE`, `ACT_ARMORED_HIJACK_EJECT`, …)
gate graph transitions, and a family of `C*VehicleCondition` / `CEntryVehicle*Condition` predicates gate
whether an action is allowed (proven — action registrars + condition registrars).

## Key classes & functions

### Component-class registration (one thunk per class, all dispatched from `FUN_14085fd00`)

Each registrar allocates the component's vtable object and interns the class name via
`FUN_140f27f60(name, len, 0, …)` (the reflected-type registrar; returns a 32-bit class atom stored in a
`DAT_` global). (proven)

| Class string | Registrar FUN_ | vtable | class-atom DAT_ | role (inferred from name) |
|---|---|---|---|---|
| `CVehicleActionVolume` | `FUN_140815630` | `PTR_LAB_141d9b6b0` | `DAT_142cb95b4` | trigger volume for vehicle interactions |
| `CVehicleCargoBay` | `FUN_1408156f0` | `PTR_LAB_141d9b688` | `DAT_142cb95ac` | cargo-bay seat/hold |
| `CVehicleConfig` | `FUN_1408157b0` | `PTR_LAB_141d9ce48` | `DAT_142cb9864` | tunable config block |
| `CVehicleData` | `FUN_140815870` | `PTR_LAB_141d9d230` | `DAT_142cb8e7c` | core vehicle data component |
| `CVehiclePartProxy` | `FUN_140815930` | `PTR_LAB_141d9c510` | `DAT_142cb96b4` | detachable/animated part proxy |
| `CVehicleRadio` | `FUN_1408159f0` | `PTR_LAB_141d9e248` | `DAT_142cb9b48` | in-vehicle radio |
| `CVehicleRule` | `FUN_140815ab0` | `PTR_LAB_141d9bc00` | `DAT_142cb93f8` | gameplay rule component |
| `CVehicleTelescopicController` | `FUN_140815b70` | `PTR_LAB_141d9c538` | `DAT_142cb96ac` | telescopic-arm seat controller |
| `CVehicleWinchController` | `FUN_140815c30` | `PTR_LAB_141d9c560` | `DAT_142cb9460` | **vehicle-mounted winch seat controller** |

`CVehicleData` is also registered as a plain reflected type by the standalone factory `FUN_1407dc2a0`
(class atom `DAT_142cb8e7c`, name len `0xc`) alongside `CTransformComponent`, `CUIInputPrompt`, `CVocalsLayer`
— i.e. it sits in the general entity-component registry (proven — `FUN_1407dc2a0` @0x1407dc2a0). (proven)

### Vehicle type hierarchy (reflected types, `FUN_140c28xxx`)

| Type string | Registrar FUN_ | type-atom DAT_ |
|---|---|---|
| `CAirVehicle` (len 0xb) | `FUN_140c28430` | `DAT_142cbda9c` |
| `CLandVehicle` (len 0xc) | `FUN_140c28700` | `DAT_142cbdab0` |
| `CSeaVehicle` (len 0xb) | `FUN_140c28820` | `DAT_142cbdab8` |
| `CBoat` (5) | `FUN_140c284c0` | `DAT_142cb96a4` |
| car — `&DAT_141d94258` (len 4, i.e. `"CCar"`) | `FUN_140c28550` | `DAT_142cb9674` |
| `CDirigible` (10) | `FUN_140c285e0` | `DAT_142cb969c` |
| `CHelicopter` (0xb) | `FUN_140c28670` | `DAT_142cb968c` |
| `CMotorBike` (10) | `FUN_140c28790` | `DAT_142cb967c` |
| `CTrain` (6) | `FUN_140c288b0` | `DAT_142cb9684` |

(proven — the string+length is read directly; `"CCar"` for the len-4 `&DAT_141d94258` is inferred from
its siblings and the 4-char length.) The three abstract base-type atoms `CAirVehicle` / `CLandVehicle` /
`CSeaVehicle` confirm a three-domain split with concrete leaves. (proven / inferred)

### Behavior-VM conditions (registered via `thunk_FUN_14aadec10(name)`)

Each is its own tiny registrar `FUN_14054dxxx` returning a condition-type-id (`DAT_142cb4xxx`). All are called
from the condition-table builders `FUN_1485e07e0` / `FUN_1485b9730` (see `callers=[]`). (proven)

| Condition string | Registrar FUN_ | type-id DAT_ |
|---|---|---|
| `CVehicleBelowCondition` | `FUN_14054d290` | `DAT_142cb44a4` |
| `CVehicleBoatCondition` | `FUN_14054d310` | `DAT_142cb4374` |
| `CVehicleBoolCondition` | `FUN_14054d390` | `DAT_142cb44ec` |
| `CVehicleCabbedCondition` | `FUN_14054d410` | `DAT_142cb43b4` |
| `CVehicleCrashImpulseCondition` | `FUN_14054d490` | `DAT_142cb42dc` |
| `CVehicleDamageState` | `FUN_14054d510` | `DAT_142cb409c` |
| `CVehicleDoorExistCondition` | `FUN_14054d590` | `DAT_142cb46b4` |
| `CVehicleDoorIsClosedCondition` | `FUN_14054d610` | `DAT_142cb46a4` |
| `CVehicleFloatCondition` | `FUN_14054d690` | `DAT_142cb44f4` |
| `CVehicleHelicopterCondition` | `FUN_14054d710` | `DAT_142cb436c` |
| `CVehicleIsCrashingOutOfControl` | `FUN_14054d790` | `DAT_142cb44b4` |
| `CVehicleIsUpsideDownCondition` | `FUN_14054d810` | `DAT_142cb453c` |
| `CVehicleJerkAccelerationCondition` | `FUN_14054d890` | `DAT_142cb42e4` |
| `CVehicleLockedForHijackCondition` | `FUN_14054d910` | `DAT_142cb46d4` |
| `CVehicleLockedForPlayerCondition` | `FUN_14054d990` | `DAT_142cb46c4` |
| `CVehicleMotorbikeCondition` | `FUN_14054da10` | `DAT_142cb4364` |
| `CVehicleVerticalAimState` | `FUN_14054da90` | `DAT_142cb3d38` |

**Entry-specific** conditions (registered together at lines 502491–502548): `CEntryVehicleDoorExistCondition`,
`CEntryVehicleDoorIsClosedCondition`, `CEntryVehicleLockedForHijackCondition`,
`CEntryVehicleLockedForPlayerCondition` (registrar `thunk_FUN_14aadec10`, e.g.
`DAT_142cb46cc = thunk_FUN_14aadec10("CEntryVehicleLockedForPlayerCondition")`). Their `GetTypeName` virtuals
are `FUN_1483acc20` → `"CEntryVehicleDoorExistCondition"` and `FUN_1483ad820` →
`"CEntryVehicleLockedForPlayerCondition"`. (proven) These are the entry-flow twins of the general vehicle
conditions: the general set answers "is the vehicle in state X" for the *occupant*; the `CEntryVehicle*` set
answers "may this character *enter/hijack* this vehicle" during the approach/entry graph. (inferred)

**Animation-graph** vehicle conditions: `CGraphEntryParentVehicleSpeedCondition`,
`CGraphParentVehicleSpeedCondition`, `CGraphParentVehicleDirToFaceAngleCondition` (registered near line
502985) — used by the anim graph to blend based on the vehicle the character is parented to (its speed and
relative facing). (proven strings / inferred role)

### Action atoms (behavior-graph actions; interned via `FUN_140f27f60`, atom cached in a `_DAT_`)

Each is registered by a one-line thunk called from the startup action table (`callers=[0x140008xxx]`). (proven)

| Action string | Registrar FUN_ | atom `_DAT_` |
|---|---|---|
| `ACT_ENTER_VEHICLE` | `FUN_142f11750` | `_DAT_142cb3448` |
| `ACT_ENTER_VEHICLE_CARGO_BAY` | `FUN_142f11850` | `_DAT_142cb3a0c` |
| `ACT_ENTER_VEHICLE_MOUNTED_WEAPON` | `FUN_142f11c30` | `_DAT_142cb3190` |
| `ACT_ENTER_VEHICLE_RIGHT` | `FUN_142f12080` | `_DAT_142cb36e0` |
| `ACT_EXIT_VEHICLE` | (line 3080904) | — |
| `ACT_EXIT_VEHICLE_HIJACK` | `FUN_142f144c0` | `_DAT_142cb3918` |
| `ACT_EXIT_VEHICLE_RIGHT` | (line 3081040) | — |
| `ACT_CLOSE_LEFT_DOOR_FROM_INSIDE` | `FUN_142f06c30` | `_DAT_142cb3678` |
| `ACT_CLOSE_RIGHT_DOOR_FROM_INSIDE` | `FUN_142f07210` | `_DAT_142cb2f84` |
| `ACT_GET_HIJACK` | `FUN_142f1c490` | `_DAT_142cb314c` |
| `ACT_ARMORED_HIJACK_EJECT` | `FUN_148355810` | pushed to controller+0x1c68 (not a static DAT) |

**Notable:** there is **no `ACT_OPEN_LEFT_DOOR` / `ACT_OPEN_RIGHT_DOOR`** in the binary — only the two
`CLOSE_*_DOOR_FROM_INSIDE` actions plus the door *conditions* (`CVehicleDoorExistCondition`,
`CVehicleDoorIsClosedCondition`). Door-opening is therefore folded into the `ACT_ENTER_VEHICLE*` animations;
only closing a door once seated is a discrete player-driven action. (proven — exhaustive `_DOOR_` search
returns exactly those two actions; inferred conclusion.)

## How it works (from the decomp)

### Startup registration
`FUN_14085fd00` is the vehicle-component class-registration dispatcher: it calls each `FUN_140815xxx`
registrar in turn (their `callers=[0x14086xxxx(FUN_14085fd00)]`), each of which allocates a 0x10-byte vtable
holder, points it at the class vtable (`PTR_LAB_141d9xxxx`), lazily interns the class name via
`FUN_140f27f60`, and inserts `{atom, holder}` into the registry map (`thunk_FUN_14cfef490`). The identical
pattern registers every subsystem; the vehicle set is the contiguous `CVehicleActionVolume … CVehicleWinchController`
run at 869961–870243. (proven)

### Seat-mounted controllers (winch / telescopic / mounted weapon)
A vehicle assembles its per-seat controllers by resolving **named component sockets** (proven —
`FUN_140c53a40` region, lines ~1466769–1466866):

- `"vehicle_mounted_component"` (len 0x19) → stored at `vehicle+0x260`
- `"vehicle_telescopic_component"` (len 0x1c) → stored at `vehicle+0x2a8`
- `"vehicle_winch_component"` (len 0x17) → stored at `vehicle+0x2f0`

Each is resolved through the asset system (`FUN_140fdc0c0(DAT_142cba830, …)`) and, when present, the
controller object is enabled: for the winch object `*(winch+0x2d0) = 1` and a bitfield update
`thunk_FUN_147b08fd0(winch+0x1d0, *(winch+0xaa8), 0, 0x1f)`; for telescopic, `*(tel+0x306)=1`,
`*(tel+0x2e0)=0`, `*(tel+0x2e4)=0x3f000000` (=0.5f) (proven — the float bit pattern is read directly).
Separately, a per-seat binder (`FUN_140c53… @ ~1449930–1450009`) attaches the seat's controller by looking
up seat objects named **`"Vehicle Seat Winch"`** and **`"Vehicle Seat Telescopic"`** via `FUN_1402995d0`
(a keyed lookup), ref-counting the resulting controller into the seat slot (`param_1+0x20` telescopic,
`param_1+0x26` winch). (proven strings / inferred slot semantics)

### The vehicle winch (vehicle side)
- The controller class is `CVehicleWinchController` (vtable `PTR_LAB_141d9c560`, atom `DAT_142cb9460`).
  It is the seat controller a player occupies to operate a vehicle-mounted winch. (proven registration;
  inferred role)
- The world-level system is **`CWinchControllerSystem`**, registered into the global systems list with class
  hash `0x41d90058` (proven — `FUN_148f960c0` region: `local_18 = "CWinchControllerSystem"; … *puVar1 =
  0x41d90058`). It sits next to `CPhysicalRestraintsSystem` (`0x41d8ffe8`) and `CPhysicsSystem`
  (`0x41cb1618`) in the physics-system block, i.e. the winch is a physics/constraint system. (proven)
- Behavior-VM predicate `CConditional_IsObjectConnectedToNumWinches` (hash `0xef9347ee`, registered at
  `FUN_143101974(0xef9347ee, "CConditional_IsObjectConnectedToNumWinches")` line 7097667; also interned by
  `FUN_140f27f60(…,0x2a,…)` at 618963) lets mission/behavior logic branch on how many winch tethers are
  attached to an object. (proven)
- Vehicle-relevant physics forces in the force registry include **`WinchSuckerDamping`** and **`ReelInKick`**
  (proven — force-name table, below). The tether tension/reel-in math and the attach/detach handshake are the
  shared grapple mechanic — see `grappling_hook.md` (`CVehicleWinchController` is listed there as a shared
  anchor). (cross-reference)

### Entry / exit / door flow
Entry is a behavior-graph state machine keyed on the `ACT_ENTER_VEHICLE*` action atoms and gated by the
`CEntryVehicle*Condition` predicates (door exists, door is closed, locked-for-player, locked-for-hijack).
The four enter variants (`_CARGO_BAY`, `_MOUNTED_WEAPON`, `_RIGHT`, plain) select the seat/socket; the exit
variants mirror them (`ACT_EXIT_VEHICLE`, `_RIGHT`, `_HIJACK`). Once seated, the occupant can fire
`ACT_CLOSE_LEFT_DOOR_FROM_INSIDE` / `ACT_CLOSE_RIGHT_DOOR_FROM_INSIDE`. Door-open has no discrete action — it
is part of the enter animation. (proven action set; inferred flow)

### Hijack
- **Standard hijack**: `ACT_GET_HIJACK` (approach/pull-out), `ACT_EXIT_VEHICLE_HIJACK` (the ejected
  occupant's forced exit), gated by `CEntryVehicleLockedForHijackCondition` /
  `CVehicleLockedForHijackCondition`. (proven strings; inferred pairing)
- **Armored hijack** (breaking into an armored/locked vehicle): a dedicated controller cluster at
  `FUN_148355xxx`. `FUN_148355810` interns `ACT_ARMORED_HIJACK_EJECT` and **pushes the action atom into the
  controller's request slot** `controller+0x1c68` via `FUN_1402870a0(controller+0x1c68, &atom)` — i.e.
  `+0x1c68` is the "requested action" queue that the behavior graph consumes. (proven)
- `FUN_148355320` is the eject gate: it checks a candidate seat (`thunk_FUN_1489da860(ctrl+0x970)` — the
  anim-graph sub-object at `+0x970`), verifies the vehicle isn't flagged busy via two state bits on the
  vehicle at `*(vehicle+0xd0)` (`>>0x28 & 1` and `>>0x34 & 1` must both be 0), and on success emits an event
  pair (`thunk_FUN_1485ee040(…,0x68af22c1,0x28,0x4fd06fba)` and `…,0xf7527b69,…`), pushes action atom
  `DAT_142cb3470` to `+0x1c68`, and advances the graph. (proven flow; bit meanings inferred as
  locked/occupied flags)
- `FUN_148355600` resets four seat/door slots: `for i in 0..3: FUN_1406b0740(ctrl+0x970, i, 1)` — the anim
  graph tracks up to 4 seats/doors. (proven; "4 seats" inferred from the loop bound)

### Driving physics — the force/impulse registry
`FUN_14775a…` (the function whose body runs 3378784–3378932, ending just before `FUN_14775a620`) registers
~120 named force/impulse effect types into a physics-effect registry via `thunk_FUN_147728150("<Name>")`.
These are the modular forces the physics step applies to a vehicle body. The vehicle-relevant families
(all proven — read directly):

- **Wheels/ground:** `WheelCollision`, `WheelDrag`, `WheelFriction`, `WheelFrictionForceOnGround`,
  `WheelSteeringDrag`, `Suspension`, `SuspensionForceOnGround`, `SurfaceDamping`, `BurnoutTorque`.
- **Handbrake/drift:** `HandbrakeReverseLinearDamp`, `HandbrakeTurnDamping`, `DriftExitDamping`,
  `DriftYawControl`, `DriftYawDamping`.
- **Motorbike:** `MotorbikeBodySteeringTorque`, `MotorbikeLeaningTorque`, `MotorbikeLeanAngularDamp`,
  `MotorbikeWheelieTorque`, `MotorbikeNosieTorque`, `MotorbikeFrontBunnyHopForce`,
  `MotorbikeRearBunnyHopForce`, `MotorbikeDriverAnimation`.
- **Helicopter:** `HelicopterPitchControl`, `HelicopterRollControl`, `HelicopterYawControl`,
  `HelicopterRotorUpForce`, `HelicopterRotorForwardForce`, `HelicopterRotorLateralForce`,
  `HelicopterAeroDragForce`, `HelicopterTopSpeedAeroDragForce`, `MainRotorWobbleImpulse`,
  `TailRotorWobbleImpulse`, `RotorCollisionOnSelf`, `RotorCollisionOnOther`.
- **Airplane/air:** `AirPlaneAeroDownforce`, `AirPlaneAeroDrag`, `AirPlaneAeroLift`, `AirPlaneRudder{Downforce,Drag,Lift}`,
  `AirPropulsion`, `AirPropulsionSteeringYaw`, `AirSteering{Pitch,Roll,Yaw}`, `AirCorrection{Pitch,Roll}`,
  `Aerodynamics`, `FinForce`, `PropellerForce`, `DirigibleControl`.
- **Boat/water:** `PlaningForce`, `PlaningTorque`, `HydroStaticForce`, `AirBuoyancy(Force)`,
  `OverallWaterDrag{Force,Torque}`, `WaterDrag{Force,Torque}`, `WaterFriction{Force,Torque}`, `Anchoring`.
- **Turbo/stunt:** `TurboJumpForwardPush`, `TurboJumpFrontSuspensionKick`, `TurboJumpRearSuspensionKick`,
  `SideBooster`.
- **Vehicle-generic & interaction:** `CharacterVehicleInteractionImpulse`, `CharacterImpulseToSOParent`,
  `VehiclePushAwayForce`, `VehicleAdditionalExplosionImpulse`, `VehicleAdditionalExplosionAngularImpulse`,
  `VehicleAdditionalExplosionImpulseSkidAndRoll`, `VehicleDebugCheat`, `WinchSuckerDamping`, `ReelInKick`,
  `MagnetWeapon`, `Keyframing`, `MotionController`.

## Data & config integration

- Vehicles are entities whose render parts are declared by RTPC components (memory `[[rtpc-entity-assembly]]`,
  render-part class `0xc1d8333a`); the gameplay layer here binds by **class atom** (the `DAT_142cbxxxx` values
  above) and by **named sockets** (`"vehicle_winch_component"`, `"Vehicle Seat Winch"`, …). (proven strings;
  inferred wiring)
- `CVehicleConfig` (atom `DAT_142cb9864`) and `CVehicleData` (atom `DAT_142cb8e7c`) are the ADF-backed config
  and runtime-data components; `CVehicleDataManager` (`thunk_FUN_148f6e780(param_1,"CVehicleDataManager",…)`
  at 3209700 / 4139182) is the manager that owns the per-model `CVehicleData` templates. (proven strings;
  inferred manager role)
- The physics-force names (`Suspension`, `WheelFriction`, `HelicopterRotorUpForce`, …) are keys into the
  per-vehicle tuning tables; each name is hashed and looked up when the ADF config assigns a magnitude/curve.
  (inferred — the registry is a name→id table; magnitudes live in config.)
- Behavior/condition names map 1:1 to the behavior-VM condition components authored on the vehicle's behavior
  graph asset (memory `[[behavior_system]]` sibling doc). (inferred)

## Notable constants / tunables

| Value | Where | Meaning (grade) |
|---|---|---|
| `0x41d90058` | `CWinchControllerSystem` world-system reg (`FUN_148f960c0` region) | winch-system class hash (proven) |
| `0x41d8ffe8` / `0x41cb1618` | adjacent | `CPhysicalRestraintsSystem` / `CPhysicsSystem` hashes — winch neighbours (proven) |
| `0xef9347ee` | `FUN_143101974` (line 7097667) | hash of `CConditional_IsObjectConnectedToNumWinches` (proven) |
| `+0x1c68` | `FUN_148355810` / `FUN_148355320` | controller "requested action" slot (`FUN_1402870a0` push target) (proven / inferred name) |
| `+0x970` | `FUN_148355320/600` | occupant's anim/behavior-graph sub-object (proven offset; inferred role) |
| `+0x2f0`, `+0x2a8`, `+0x260` | `FUN_140c53…` (~1466773–1466843) | winch / telescopic / mounted-weapon component slots on the vehicle (proven) |
| `+0x2d0`, `+0xaa8` | same | winch-component "enabled" flag + bitfield source (proven offsets) |
| `+0x306`,`+0x2e0`,`+0x2e4=0x3f000000` | same | telescopic enabled flag + 0 + 0.5f tunable (proven) |
| `vehicle+0xd0` bits `0x28`,`0x34` | `FUN_148355320` | vehicle busy/locked/occupied state flags checked before eject (proven bits; inferred meaning) |
| loop bound `4` | `FUN_148355600` | seat/door count the graph tracks (proven bound; inferred meaning) |

## Call-graph highlights

- `FUN_14085fd00` → all `FUN_140815xxx` vehicle-component registrars (`CVehicleData`, `…WinchController`, …). (proven)
- `FUN_1485e07e0` / `FUN_1485b9730` → all `FUN_14054dxxx` vehicle-condition registrars. (proven)
- startup action table `0x140008xxx` → `FUN_142f11750`/`…144c0`/`…06c30`/`…07210`/`…1c490` (enter/exit/door/hijack action interning). (proven)
- `FUN_1489358ec` → `FUN_148355810` (interns `ACT_ARMORED_HIJACK_EJECT` into the controller). (proven caller edge)
- `FUN_148355810`/`FUN_148355320` → `FUN_1402870a0` (push action atom to `controller+0x1c68`). (proven)
- `FUN_148355600` → `FUN_1406b0740(ctrl+0x970, i, 1)` ×4 (reset seats/doors). (proven)
- `FUN_140c28af0` (a hot shared helper) is called from many vehicle-type builders (`FUN_140c1edc0`,
  `FUN_140c5f980`, `FUN_140c65f50`, `FUN_140c6a3a0`, `FUN_140c53a40`) — the common vehicle-assembly path. (proven callers)

## Open questions / lower-confidence

- **Winch attach/pull math (vehicle side).** The `CVehicleWinchController` vtable (`141d9c560`) methods that
  perform attach/reel are not yet walked; the tether/reel physics is shared with the grapple and lives behind
  `CWinchControllerSystem` + `CPhysicalRestraintsSystem` — needs the grapple doc's tension/reel functions to
  be tied to the `WinchSuckerDamping`/`ReelInKick` force application. (open)
- **`vehicle+0xd0` flag layout.** Bits `0x28`/`0x34` are used as lock/occupied gates in `FUN_148355320` but
  the full flag word (which bit = locked-for-player vs locked-for-hijack vs occupied) isn't enumerated. The
  `CVehicleLockedForPlayer/Hijack` condition eval functions would resolve this. (open)
- **`CVehicleData`/`CVehicleConfig` field layout.** Only the class atoms and manager are proven; the ADF
  struct fields (mass, seat table, door table, force magnitudes) are not yet mapped. (open)
- **Car type name.** `&DAT_141d94258` (len 4) is inferred as `"CCar"` from context; not read as a literal. (low-confidence)
- **Exit-action atoms.** `ACT_EXIT_VEHICLE` / `ACT_EXIT_VEHICLE_RIGHT` registrar thunks were located by string
  (lines 3080904 / 3081040) but their `_DAT_` atom globals weren't individually captured. (minor)

## Appendix — decomp anchors

Strings and addresses used above (all in `jc4_all_functions_decomp.txt`):

- Component registrars: `FUN_140815630/6f0/7b0/870/930/9f0/ab0/b70/c30` (dispatched by `FUN_14085fd00`);
  standalone `CVehicleData` factory `FUN_1407dc2a0`; manager string `"CVehicleDataManager"` (3209700, 4139182).
- Type hierarchy: `FUN_140c28430/4c0/550/5e0/670/700/790/820/8b0` — `CAirVehicle`/`CLandVehicle`/`CSeaVehicle`/
  `CBoat`/car/`CDirigible`/`CHelicopter`/`CMotorBike`/`CTrain`.
- Conditions: `FUN_14054d290 … FUN_14054da90` (17 `CVehicle*Condition`/`State`); entry set at 502491–502548;
  graph set near 502985; `GetTypeName` virtuals `FUN_1483acc20`, `FUN_1483ad820`.
- Actions: `FUN_142f06c30` (CLOSE_LEFT_DOOR), `FUN_142f07210` (CLOSE_RIGHT_DOOR), `FUN_142f11750`
  (ENTER_VEHICLE), `FUN_142f11850`/`142f11c30`/`142f12080` (enter variants), `FUN_142f144c0`
  (EXIT_VEHICLE_HIJACK), `FUN_142f1c490` (GET_HIJACK), `FUN_148355810` (ARMORED_HIJACK_EJECT); exit strings at
  3080904/3081040.
- Winch: registrar `FUN_140815c30` (vtable `141d9c560`, atom `DAT_142cb9460`); world system
  `"CWinchControllerSystem"` @4138419 (hash `0x41d90058`); condition
  `CConditional_IsObjectConnectedToNumWinches` @618963/4140497/7097667 (hash `0xef9347ee`); sockets
  `"vehicle_winch_component"` @1466839, `"Vehicle Seat Winch"` @1449973; forces `WinchSuckerDamping` @3378899,
  `ReelInKick` @3378871.
- Hijack cluster: `FUN_148355320` (eject gate), `FUN_148355600` (seat reset), `FUN_148355810` (eject action).
- Physics force registry: `thunk_FUN_147728150(...)` table, lines 3378784–3378899.
- Force intern helper: `FUN_140f27f60(name,len,…)` (reflected-type/atom registrar); `thunk_FUN_14aadec10(name)`
  (condition-type registrar); `FUN_1402870a0` (action-atom push).

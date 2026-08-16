# Player Traversal & Movement — parachute, wingsuit, hoverboard, locomotion input & camera modifiers

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative). "proven" = read
directly from the Ghidra decomp (`output/_ghidra_jc4/jc4_all_functions_decomp.txt`); "inferred" = strong
structural deduction; "speculative" = plausible but unconfirmed. Every claim cites a `FUN_<addr>`.

## Overview

JC4's aerial/ground traversal is **one shared movement state machine** layered on the animation/behavior
graph, not three separate systems. A family of animation **state-tasks** — `NStateTask_MovementFallingTask`,
`NStateTask_MovementFreeFallingTask`, `NStateTask_MovementJumpTask`, `NStateTask_MovementParachuteTask`,
`NStateTask_MovementWingSuitTask`, `NStateTask_HoverboardTask`, `NStateTask_InputHoverboardInAirTask` —
implements each mode; they are named/profiled in `FUN_1405c9950` (parachute+wingsuit) and
`FUN_1486212e0`/`FUN_148621090` (hoverboard) (proven). Each mode is entered/left by pushing a named
**action** (`ACT_TRIGGER_*` / `ACT_CLOSE_*`) onto the character's behavior component; transitions between
free-fall → wingsuit → parachute → hoverboard are all selected by the same small set of dispatchers reading
data-driven input/animation flags (proven, `FUN_14862f290`, `FUN_1405d8bb0`, `FUN_1405d8f60`,
`FUN_148667d80`). Flight physics per mode is a large data-driven integrator: `FUN_140581340` for wingsuit
air-control and `FUN_14058c3c0` for parachute lift/drag/steering (proven). Separately, a cluster of **camera
modifiers** (`CMoveInput{Yaw,Pitch,Offset}Modifier`, `CRollModifier`, `CAlignToVelocityModifier`) reads the
player move-input vector and uses it to drive the camera — these are *camera* modifiers under
`camera/cameramodifiers/`, not locomotion modifiers (proven, `FUN_142eff900`/`142effe40`/`142f00040`/
`142f003f0` register their source paths).

Naming/lineage note: this is Avalanche Apex (JC2/3 lineage). Classes register their C++ names as string
constants (e.g. `CParachuteObject`), and actions/segments register interned strings (`ACT_*`, `SEG_*`) — these
strings are the primary anchors.

## Key classes & functions

| Name string | FUN_ (registration / implementation) | Role | Grade |
|---|---|---|---|
| `CParachuteObject` (CObject type id `0x10`) | `FUN_1406b7840` → `&DAT_142cb8110` | Parachute world-object type registration | proven |
| `CParachuteObjectState` / `CParachuteBlendState` | `FUN_14054b610` / `FUN_14054b590` | Anim state / blend-state name-hash getters | proven |
| `CWingsuitObject` (id `0xf`) | `FUN_140795db0` → `&DAT_142cb8838` | Wingsuit world-object type | proven |
| `CWingsuitObjectState` / `CWingsuitBlendState` | `FUN_14054e790` / `FUN_14054e710` | Anim state / blend-state getters | proven |
| `CPremiumWingsuitUnlocker` (id `0x18`) | `FUN_1407db480` → `&DAT_142cb8df0`; ctor `FUN_140811ab0` | Premium-wingsuit DLC unlock (prerequisite object) | proven / inferred |
| `CHoverboardObject` (id `0x11`) | `FUN_1406b7720` → `&DAT_142cb8228` | Hoverboard world-object type | proven |
| `CHoverboardUnlocker` (id `0x13`) | `FUN_140845f30` → `&DAT_142cb9764`; ctor `FUN_148d736d0` | Hoverboard DLC3 unlock entity | proven / inferred |
| `CHoverboardBlendState` / `CHoverboardPanningBlendState` | `FUN_140549390` / `FUN_140549490` | Hoverboard anim blend-states (base + lean/panning) | proven |
| `CHoverboardInputCondition` | `FUN_140549410` → `DAT_142cb4534` | Hoverboard input-graph condition | proven |
| `CMoveInputOffsetModifier` | `FUN_1482349f0` (`DAT_142cb2c10`); Update `FUN_148274e30`; reg `FUN_142eff900` | Camera positional-offset from move-input | proven |
| `CMoveInputPitchModifier` | `FUN_148234de0` (`DAT_142cb2ce0`); Update `FUN_148275ad0`; reg `FUN_142effe40` | Camera pitch from move-input | proven |
| `CMoveInputYawModifier` | `FUN_148235080` (`DAT_142cb2c58`); Update `FUN_148276190`; reg `FUN_142f00040` | Camera yaw from move-input | proven |
| `CRollModifier` | `FUN_148235350` (`DAT_142cb2b00`); Update `FUN_1404e96c0`; reg `FUN_142f003f0` | Camera roll (turn/velocity-driven) | proven |
| `CAlignToVelocityModifier` | `FUN_1482365c0` (`DAT_142cb2b70`); Update `FUN_148235f70` | Camera align-to-velocity (sibling modifier) | proven |
| Player-state conditionals | `CConditional_IsPlayerInParachute` (`FUN_148f99a90`), `CConditional_IsPlayerInWingsuit` (`FUN_140652d10`), `CConditional_IsPlayerOnHoverBoard` (getter @L619617) | Mission/scripting predicates | proven |
| **`FUN_140566980`** | thin wrapper → `FUN_1402870a0(actor + 0x1c68, actId)` | **QueueAct** — push an action onto the character's behavior/anim component (`+0x1c68`) | proven |
| **`FUN_140fd13a0`** | reflection read (`thunk_FUN_14aefd600`) | **Named flag test** — returns (float property != 0.0); used for all input/anim-tag gates | proven |
| `FUN_140fd0ad0` / `FUN_140fd0cd0` | reflection reads | Float property / 2-float vector property (the move-input vector) | inferred |
| **`FUN_140581340`** (size 10277) | — | **Shared air-control integrator** (wingsuit tick + others) | proven |
| **`FUN_14058c3c0`** (size 12028) | — | **Parachute flight/steering integrator** (called only by parachute tick) | proven |
| `FUN_1485c5fb0` (size 2876) | — | Wingsuit orientation + boost-impulse applier (reads MAINBODY anim segment tags) | proven |
| `FUN_148617e20` | — | Hoverboard aim/panning steering driver | inferred |

## How it works (from the decomp)

### 1. The action-dispatch / transition layer (the "brain")

All mode changes flow through two primitives (proven):

- **`FUN_140566980(actor, &ACT_id)`** just forwards to `FUN_1402870a0(actor + 0x1c68, ACT_id)` — i.e. it
  enqueues the interned action onto the behavior/anim-state-machine component living at `actor + 0x1c68`.
- **`FUN_140fd13a0(configPtr, keyHash)`** reads a named float/bool property off a data-driven config block
  via reflection and returns `value != 0.0` — used everywhere as "is this input/anim-tag active?".

The central input→action mapper is **`FUN_14862f290`** (proven): it fetches a config blob (from the object at
`obj+0x3e0`, or a default), then in priority order:

```
if flag(cfg, DAT_141d362ec)  -> QueueAct ACT_TRIGGER_WINGSUIT   (_DAT_142cb36f8)
elif flag(cfg, DAT_141d362f0) -> QueueAct ACT_TRIGGER_PARACHUTE  (_DAT_142cb3b18)
elif DAT_142cbc3d9 && flag(cfg, DAT_141d362f4)
                              -> QueueAct ACT_TRIGGER_HOVERBOARD (_DAT_142cb39e8)
```

`DAT_142cbc3d9` is the **global hoverboard-enable gate**, checked at every hoverboard trigger site
(`FUN_1405d8bb0` L557750/557900, `FUN_1405d8f60` L557900, `FUN_14862f290` L3917770, and @1333737). It is set
by **`FUN_148d460e0`**: `DAT_142cbc3d9 = 1` iff a player flag (`player+0x1d0`) is set **and** an entitlement
check `FUN_1409b17c0(...)` passes — i.e. the hoverboard DLC/unlocker must be owned (proven that it gates all
triggers; inferred that `FUN_1409b17c0` is the entitlement/unlock check consistent with `CHoverboardUnlocker`).
`FUN_148667d80` performs the same free-fall→wingsuit-vs-parachute-vs-hoverboard branch selection from the
falling/freefalling input slots (proven).

### 2. Parachute

- **Deploy**: `ACT_TRIGGER_PARACHUTE` (`_DAT_142cb3b18`, interned @L3088253) is queued by the dispatchers above
  and by the contextual handler `FUN_1405d5590` (which also branches to vehicle-exit/stunt acts, sharing one
  traversal handler) (proven).
- **Active flight tick**: while the anim node id (`DAT_142cb3b70`) is a parachute state, `FUN_1405d8bb0` runs
  each frame and calls the physics integrator `FUN_14058c3c0(actor, dt, inputVec, cfg = state+0x428)` (proven).
  The config block base is `movementData+0x428` (movementData = `character[0x128]`).
- **Physics** (`FUN_14058c3c0`, proven structure / inferred coefficients): applies lift/drag/steering from the
  `param_4` config block. It resolves anim segments `PARACHUTE_CUSTOM_LIFT_SEGMENT` (L529491),
  `SEG_PARACHUTE_STEERING` (L529927), `SEG_PARACHUTE_STEERING_BLENDIN` (L529932) and blends steering response.
- **Auto-idle**: inside `FUN_14058c3c0` (~L529148) a descent condition arms a timer `actor+0x2e6c = 0.4f`
  (`0x3ecccccd`), weight `actor+0x2e70 = 1.0f`; when it decrements to ≤0 while still in a parachute state it
  force-queues `ACT_FORCE_PARACHUTE_IDLE` (`_DAT_142cb388c`) via `FUN_1402870a0(actor+0x1c68, …)` (proven;
  consumer confirmed at L529173).
- **Close**: `ACT_CLOSE_PARACHUTE` (`_DAT_142cb38dc`) queued by `FUN_1405d8bb0` on input `DAT_141d37fdc`, and
  force-closed by `FUN_148458000` on a speed/lifetime condition. An **anti-tamper datablock string** in
  `FUN_148458000` (L3868875) embeds the close rule literally: `character->QueueAct(ACT_CLOSE_PARACHUTE)` when
  `m_LifeTime > 3600.f` (proven).
- **State identity**: the "in parachute" predicate `FUN_148576a80` checks the current anim state id against
  three grapple-entry states `S_GRAPPLE_REEL_TO_PARACHUTE_EARLY/FANCY/LATE`
  (`DAT_142cb3b10`/`3100`/`3208`, registered near `FUN_142fc9510`) (proven; whether these are the *only*
  parachute states is inferred).

### 3. Wingsuit

- **Enter**: `ACT_TRIGGER_WINGSUIT` (`_DAT_142cb36f8`) / `ACT_SEQUENCE_TRIGGER_WINGSUIT` (`_DAT_142cb3078`) /
  `ACT_GT2_JC_WINGSUIT_OPEN`; queued from `FUN_1405d8bb0`, `FUN_14862f290`, `FUN_148632ac0`, `FUN_148667d80`
  (proven).
- **Active tick** `FUN_1405d8f60` (proven): each frame it writes the wingsuit gravity into the movement struct
  `movementData+0x7b8` and calls the integrator `FUN_140581340(actor, dt, &accel, cfg=movementData+0x7b8)`.
  The gravity constant is **−25.0 m/s²** (`0xC1C80000`), and an **anti-tamper datablock string** at L557846
  names it exactly: `SCustomMovementSettings::m_WingsuitSettings.m_AirControl.m_Gravity = -25.f` — a tamper
  resets `movementData+0x7b8` to `0xc1c80000` (proven; strongest single tunable in the whole system).
- **Integrator** `FUN_140581340` (proven structure): integrates velocity against gravity using the
  `m_AirControl` coefficients passed as `param_4` (per-axis critically-damped smoothing via
  `thunk_FUN_1482c3c40`; damping/lerp coeffs at `param_4[0x35],[0x37],[0x38],[0x3a]` — inferred meanings).
- **Orientation & boost** `FUN_1485c5fb0` (proven names / inferred effect): reads MAINBODY anim segment tags and
  turns them into velocity/orientation changes — align (`SEG_WINSGUIT_TRANSITION_ALIGN`, `_ALIGN_VEL`,
  `_ALIGN_XZ`, `_ALIGN_Y`; note the shipped misspelling "WINSGUIT"), roll (`_TRANSITION_ROLL`), and boosts:
  forward `SEG_WINGSUIT_TRANSITION_BOOST_FWD` adds `(1-t)·t·scale·DAT_141ce55d0 · fwdDir` (the dive-to-accelerate
  impulse) and up `SEG_WINGSUIT_TRANSITION_BOOST_UP` adds an up-vector impulse scaled by `DAT_141cae1bc`. Blend
  weights use a smoothstep `t²·D8 − t³·D4` (proven form, `DAT_141ca70d8`/`DAT_141ca70d4`; values ≈3.0/2.0
  inferred). Boost availability is gated by `FUN_149812cf0` (`SEG_WINGSUIT_BOOST_ENABLED`) (inferred).
- **Exit**: `FUN_1405d8f60` queues `ACT_CLOSE_WINGSUIT` (`_DAT_142cb3b1c`) when the vertical component ≤ 0 and a
  close anim-tag fires (input `DAT_141d37fe0`); it can also branch straight to `ACT_TRIGGER_PARACHUTE`
  (`DAT_141d37fe8`) or hoverboard, and to `ACT_SLINGSUIT_TO_REEL` (`_DAT_142cb3a34`, the wingsuit→grapple-reel
  hand-off — see grappling_hook.md) (proven).
- **Premium unlock**: `CPremiumWingsuitUnlocker` (type `0x18`) is constructed by `FUN_140811ab0` as a sibling of
  `CPrerequisiteObject` (`FUN_140811b70`) → a prerequisite/entitlement gate for the DLC "premium wingsuit"
  (booster/missile variant); front-end strings `dlc_premium_wingsuit_unlock_*`, HUD `HoverPremiumWingsuit` /
  `SetPremiumWingsuitState` confirm the UX (proven strings / inferred gate role).

### 4. Hoverboard

- **Enter/leave**: `ACT_TRIGGER_HOVERBOARD` (`_DAT_142cb39e8`) / `ACT_CLOSE_HOVERBOARD` (`_DAT_142cb3868`),
  queued through the shared dispatchers, always behind the `DAT_142cbc3d9` gate (proven).
- **Tasks**: mounted vs airborne control are two anim state-tasks — `NStateTask_HoverboardTask`
  (`FUN_1486212e0`, input context at object`+0x420`/`+0x430`) and `NStateTask_InputHoverboardInAirTask`
  (`FUN_148621090`, context `+0x430`/`+0x440`) (proven). The input contexts `player_hoverboard_input` (with the
  `move_y` analog axis bound via `FUN_140e87c30`/`FUN_140e87e00`) and `player_hoverboard_inair_input` are built
  in `FUN_140b149f0` (proven).
- **Steering** `FUN_148617e20` (inferred): computes the signed aim angle between camera-forward and board
  facing; if the angle falls in the band `[_DAT_141d37f40, DAT_141d37f20]` it queues
  `ACT_HOVERBOARD_AIM_TRANS_L_TO_R` (`&DAT_142cb3394`) else `ACT_HOVERBOARD_AIM_TRANS_R_TO_L`
  (`&DAT_142cb3340`), then continuously remaps the angle (piecewise, thresholds `DAT_141ca6cb8`/`cd4`,
  offsets `cc0`/`cdc`, scale `DAT_141cb8224`) and writes it as the anim "aim-lean" parameter hash `0xa5c51048`
  consumed by `CHoverboardPanningBlendState`. So steering is **aim-driven lean**, with `move_y` as
  throttle/steer. `SEG_NO_HOVERBOARD_DRIFT` marks segments where drift is disabled (proven string).
- **Sub-modes**: game-modifier tags `MOD_HOVERBOARD`, `MOD_HOVERBOARD_MAGRAIL`, `MOD_HOVERBOARD_REEL_SKI`, plus
  `Hoverboard_SwitchMagrail`, indicate base / magrail-grind / reel-ski variants (proven strings; runtime effect
  inferred).
- **Unlock**: `CHoverboardUnlocker` (type `0x13`) built by `FUN_148d736d0` (vtable `PTR_LAB_141d9b4d0`),
  registered as a world entity; DLC3 content strings `quest_dlc3_hoverboard_course_01..06`,
  reward `dlc3_rp_hoverboarding` confirm it is DLC3 (proven strings / inferred gate role).
- **Not located**: the hover-height / ground-follow raycast physics — not present in the anim/camera/task
  functions inspected; presumably in the `CHoverboardObject` movement component (open question).

### 5. Move-input CAMERA modifiers (yaw / pitch / offset / roll)

These five modifiers live under `source/project/game/camera/cameramodifiers/` (proven from the registration
source-path strings in `FUN_142eff900`/`142effe40`/`142f00040`/`142f003f0`/`142eff630`). They read the player
move-input vector via the property backend (`FUN_140fd0cd0` for the 2D vector with keys
`DAT_141d1446c`/`DAT_141d14470`, `FUN_140fd0ad0` for scalars, `FUN_140fd13a0` for bools) and write a camera
delta with `thunk_FUN_147b11c50(cam+0xc8, dYaw, dPitch, dRoll)` (camera yaw at `cam+0xc8`, pitch at `cam+0xcc`).
First-hand cross-check: the apply `FUN_148234330` multiplies a 4×4 by the context matrix, extracts yaw/pitch
(`thunk_FUN_147b3d460`/`147b36680`), subtracts the stored `cam+0xc8`/`+0xcc`, and writes the delta — confirming
the mechanism (proven). Per-modifier math (inferred coefficient meanings; the tunables are data-driven config
fields `cfg[+0x08..0x38]`):

- **Yaw** (`FUN_148276190`/helper `FUN_1482822b0`): `t = clamp((|move| − cfg[+0x10]) / (1 − cfg[+0x10]),0,1)`,
  `dYaw = t · cfg[+0x20] · dt`, accumulated & clamped to `cfg[+0x24]`; gated by a deadzone `cfg[+0x08]`.
- **Pitch** (`FUN_148275ad0`): same shape with an engage-delay timer `state+0x18` (hysteresis), rate
  `cfg[+0x1c]`, clamp `cfg[+0x20]` plus camera pitch limits.
- **Offset** (`FUN_148274e30`): each axis normalized by `cfg[+0x18]`, fed through a critically-damped spring
  `thunk_FUN_1481361d0`, then added into a positional pivot offset (`param_4+0x30/+0x38`).
- **Roll** (`FUN_1404e96c0`): fixed roll `cfg[+0x38]` (if bool `DAT_141d1445c`), plus turn-rate roll
  (`cfg[+0x28]`), plus explicit roll-axis input (`cfg[+0x2c]`, key `DAT_141d14464`), plus a context lookup
  `CRollModifier::ProcessCameraContext` (`FUN_1404b0070`).

### 6. Ground locomotion actions (crouch / accelerate / climb)

`ACT_ACC_FORWARD`, `ACT_ACC_BACKWARD`, `ACT_CLIMB_OBSTACLE`, `ACT_CROUCH` each have **exactly one** reference in
the decomp — their own registration line — and no `FUN_140566980` consumer (proven, ref-count = 1). They are
therefore **fired by the animation/behavior-graph data tables**, not by hardcoded C. The related anim states
(`CLocomotionBlendState`, `CLocomotionAdditiveBlendState`, `CLocomotionCycleSyncState`,
`CLocomotionTypeDirSpeedBlendState`, `CFreeFallBlendState`) and the `ControllerCrouchToggle` input option
confirm this is graph-driven (proven).

## Data & config integration

- **CObject type-id table** (`FUN_140f27f60(name, id)` registry, all proven): `CWingsuitObject`=0xf,
  `CParachuteObject`=0x10, `CHoverboardObject`=0x11, `CGrapplingHookWire`=0x12, `CHoverboardUnlocker`=0x13,
  `CPremiumWingsuitUnlocker`=0x18. These are the composite-asset object classes (see `[[composite-assets]]`).
- **Config struct**: `SCustomMovementSettings` with `m_WingsuitSettings.m_AirControl.m_Gravity` (proven from the
  anti-tamper string, `FUN_1405d8f60`). Per-mode config blocks are sub-ranges of the character's movement data
  (`character[0x128]`): parachute at `+0x428`, wingsuit at `+0x7b8`. Fields are read by reflection
  (`FUN_140fd13a0`/`140fd0ad0`/`140fd0cd0`) using name-hash keys — so deploy speeds, lift/drag, blend times and
  camera-modifier rates are **data-driven** (RTPC/ADF config assets), not code literals (inferred, consistent
  with `[[rtpc-entity-assembly]]`).
- **Actions & segments**: interned via `FUN_140f27f60("ACT_…"/"SEG_…", strlen)` — the 2nd arg is just the
  string length, so these are string-intern IDs for the behavior VM, not enum ordinals (proven).
- **Scripting predicates**: `CConditional_IsPlayerInParachute` / `…InWingsuit` / `…OnHoverBoard` expose the
  player traversal state to missions/RTPC logic (proven).
- **Input/control options**: `Controller*` settings parsed in the block around `FUN_140bab800` —
  `ControllerSensitivityPitch`/`Yaw`, `ControllerInvertAimingX`/`Y`, `ControllerFlightYAxis` (invert flight
  pitch), `ControllerCrouchToggle`, `ControllerVibration`, `ControllerDisableInputWhenOutOfFocus` (proven).
- **Wind intersection**: during parachute/wingsuit flight, `FUN_1405d8bb0` fires `ACT_WIND_GUST`
  (`_DAT_142cb33f4`) + a physics event `0x2e7e3f09` when a vertical-speed threshold `DAT_141d3b230` is crossed
  (proven). This is the hand-off to the wind system — see wind_and_weather.md (not duplicated here).

## Notable constants / tunables

| Value | Meaning | FUN_ | Grade |
|---|---|---|---|
| `-25.0f` (`0xC1C80000`) | wingsuit `m_AirControl.m_Gravity`, written to `movementData+0x7b8`; anti-tamper enforced | `FUN_1405d8f60` (L557846/557850) | proven |
| `3600.f` | max parachute `m_LifeTime` before forced `ACT_CLOSE_PARACHUTE` (anti-tamper string) | `FUN_148458000` (L3868875) | proven |
| `0.4f` (`0x3ecccccd`) → `actor+0x2e6c`; `1.0f` → `actor+0x2e70` | parachute auto-force-idle countdown + weight, armed on descent | `FUN_14058c3c0` (~L529148) | proven |
| `4.0 / 1.0` (`DAT_142cb4da8/dac`), `5.0 / 1.0` (`DAT_142cb4db8/dbc`) | parachute steering blend-in rate/target pairs | `FUN_14058c3c0` | proven (values), inferred (meaning) |
| smoothstep `t²·D8 − t³·D4` (≈3.0/2.0) | wingsuit boost/align blend curve | `FUN_1485c5fb0` (`DAT_141ca70d8`/`d4`) | proven form, inferred values |
| `DAT_141ce55d0` / `DAT_141cae1bc` | wingsuit forward-boost / up-boost impulse scales | `FUN_1485c5fb0` | proven ref, value not in dump |
| `DAT_141d3b230` | vertical-speed threshold gating `ACT_WIND_GUST` in flight | `FUN_1405d8bb0` (L557757) | proven ref |
| `DAT_141cb8224` | angle-unit scale (deg↔rad-type factor) used by camera modifiers + hoverboard aim | `FUN_148274e30`, `FUN_148617e20` | inferred |
| aim band `_DAT_141d37f40 .. DAT_141d37f20` | hoverboard L→R vs R→L aim-transition select | `FUN_148617e20` | proven (usage), inferred (meaning) |
| anim param hash `0xa5c51048` | hoverboard aim-lean/panning blend parameter | `FUN_148617e20` | inferred |
| camera-modifier `cfg[+0x10]` (input threshold), `[+0x1c]`/`[+0x20]` (rate), `[+0x24]` (max-offset), `[+0x28..0x38]` (roll terms) | data-driven per-modifier tunables | `FUN_148274e30`/`148275ad0`/`148276190`/`1404e96c0` | inferred |

> Numeric values of `.rdata` float DATs (e.g. `DAT_141ca6c*`, `DAT_141ce55d0`, `DAT_141cb8224`) are **not**
> present in this function-only decomp — only their addresses. They live in the read-only data section / config
> assets; recover them from a memory dump if exact figures are needed.

## Call-graph highlights

- **QueueAct spine**: `FUN_14862f290` / `FUN_1405d8bb0` / `FUN_1405d8f60` / `FUN_148667d80` →
  `FUN_140566980` → `FUN_1402870a0(actor+0x1c68, ACT_id)`.
- **Flag/config reads**: `FUN_140fd13a0` / `FUN_140fd0ad0` / `FUN_140fd0cd0` → reflection
  (`thunk_FUN_14aefd600`).
- **Parachute tick**: `FUN_1405d8bb0` → `FUN_14058c3c0` (physics) + `FUN_14058c3c0` → `FUN_1402870a0`
  (force-idle).
- **Wingsuit tick**: `FUN_1405d8f60` → `FUN_140581340` (integrator, also called by `FUN_14866bf30`/
  `FUN_14866c480`) → `FUN_1485c5fb0` (boost/orientation).
- **Hoverboard gate**: `FUN_148d460e0` sets `DAT_142cbc3d9` from player flag + `FUN_1409b17c0` entitlement;
  read by all hoverboard trigger sites.
- **Camera modifiers**: registration `FUN_142eff900`/… ; per-frame Updates `FUN_148274e30`/`148275ad0`/
  `148276190`/`1404e96c0`/`148235f70`; apply `FUN_148234330` → `thunk_FUN_147b11c50(cam+0xc8, …)`.
- Type/object factory: the object-type getters (`FUN_1406b7840` etc.) are all called by the big factory
  `FUN_14085fd00` (parachute object ctor path reported DECOMP FAIL — see open questions).

## Open questions / lower-confidence

- **Object-creation path**: `FUN_14085fd00` (CObject factory, size 17436) decompiled poorly; the
  `CParachuteObject`/`CWingsuitObject`/`CHoverboardObject` construction and initial config-load were not read.
- **Hover physics**: hoverboard hover-height / ground-follow was not located; likely in `CHoverboardObject`'s
  movement/vehicle component outside the anim/camera cluster.
- **rdata values**: exact floats for air-control coefficients, boost scales, camera-modifier rates, and
  `DAT_141d3b230` / `DAT_141cb8224` are not in the decomp text (data section / config assets).
- **Property-key hashes**: the input/anim-tag keys (`DAT_141d362ec/f0/f4`, `DAT_141d37fdc/e0/e4/e8/010`,
  `DAT_141d1445c/1446c/14470/14464`) are opaque name-hashes — not resolved to strings here.
- **Parachute state set**: `FUN_148576a80` checks three `S_GRAPPLE_REEL_TO_PARACHUTE_*` states; whether steady
  parachute flight uses additional states on another anim layer is unconfirmed.
- **`ACT_GT2_JC_PARACHUTE_OPEN`** / **`ACT_GT2_JC_WINGSUIT_OPEN`** are registered but have no direct code
  consumer in the decomp — presumably fired via a computed/vtable act table (graph data).
- Semantics of `thunk_FUN_147b11c50` arg order (which float is yaw vs pitch vs roll) and of the spring
  primitive `thunk_FUN_1481361d0` are inferred from call sites.

## Appendix — decomp anchors

Class/type strings & getters (all proven):
- `CParachuteObject` L675643 `FUN_1406b7840` → `&DAT_142cb8110` (id 0x10)
- `CParachuteObjectState` L504790 `FUN_14054b610`; `CParachuteBlendState` L504771 `FUN_14054b590`
- `CWingsuitObject` L804534 `FUN_140795db0` → `&DAT_142cb8838` (id 0xf)
- `CWingsuitObjectState` L506671 `FUN_14054e790`; `CWingsuitBlendState` L506652 `FUN_14054e710`
- `CPremiumWingsuitUnlocker` L840472 `FUN_1407db480` → `&DAT_142cb8df0` (id 0x18); ctor `FUN_140811ab0`
- `CHoverboardObject` L675593 `FUN_1406b7720` → `&DAT_142cb8228` (id 0x11)
- `CHoverboardUnlocker` L900935 `FUN_140845f30` → `&DAT_142cb9764` (id 0x13); ctor `FUN_148d736d0`
- `CHoverboardBlendState` L503479 `FUN_140549390`; `CHoverboardPanningBlendState` L503517 `FUN_140549490`;
  `CHoverboardInputCondition` L503498 `FUN_140549410`
- `CMoveInputOffsetModifier` L3814624 `FUN_1482349f0`/Update `FUN_148274e30`;
  `CMoveInputPitchModifier` L3814673 `FUN_148234de0`/Update `FUN_148275ad0`;
  `CMoveInputYawModifier` L3814686 `FUN_148235080`/Update `FUN_148276190`;
  `CRollModifier` L3814702 `FUN_148235350`/Update `FUN_1404e96c0`;
  `CAlignToVelocityModifier` `FUN_1482365c0`/Update `FUN_148235f70`
- `CConditional_IsPlayerInParachute` `FUN_148f99a90`; `…InWingsuit` `FUN_140652d10`;
  `…OnHoverBoard` getter @L619617

Action strings (`FUN_140f27f60("ACT_…", strlen)` → `_DAT_…`, all proven):
- `ACT_TRIGGER_PARACHUTE` L3088253 `_DAT_142cb3b18`; `ACT_CLOSE_PARACHUTE` L3079944 `_DAT_142cb38dc`;
  `ACT_FORCE_PARACHUTE_IDLE` L3081726 `_DAT_142cb388c`
- `ACT_TRIGGER_WINGSUIT` L3088266 `_DAT_142cb36f8`; `ACT_CLOSE_WINGSUIT` L3080007 `_DAT_142cb3b1c`;
  `ACT_SEQUENCE_TRIGGER_WINGSUIT` L3086837 `_DAT_142cb3078`
- `ACT_TRIGGER_HOVERBOARD` L3088240 `_DAT_142cb39e8`; `ACT_CLOSE_HOVERBOARD` L3079888 `_DAT_142cb3868`;
  `ACT_HOVERBOARD_AIM_TRANS_L_TO_R` L3083173 `&DAT_142cb3394`; `ACT_HOVERBOARD_AIM_TRANS_R_TO_L` L3083186
  `&DAT_142cb3340`
- `ACT_WIND_GUST` L3088789 `_DAT_142cb33f4`; `ACT_SLINGSUIT_TO_REEL` L3087015 `_DAT_142cb3a34`
- `ACT_ACC_FORWARD` L3079536, `ACT_ACC_BACKWARD` L3079523, `ACT_CLIMB_OBSTACLE` L3079875, `ACT_CROUCH` L3080033
  (data-driven; single ref each)

Segment strings (proven): `PARACHUTE_CUSTOM_LIFT_SEGMENT`, `SEG_PARACHUTE_STEERING`,
`SEG_PARACHUTE_STEERING_BLENDIN`, `SEG_WINGSUIT_BOOST_ENABLED`, `SEG_WINGSUIT_TRANSITION_BOOST_FWD/_UP`,
`SEG_WINSGUIT_TRANSITION_ALIGN/_ALIGN_VEL/_ALIGN_XZ/_ALIGN_Y/_ROLL` (sic), `SEG_NO_HOVERBOARD_DRIFT`.

State-tasks (proven): `NStateTask_MovementFallingTask`, `NStateTask_MovementFreeFallingTask`,
`NStateTask_MovementJumpTask`, `NStateTask_MovementParachuteTask` (`FUN_1405c9950` L553007, `FUN_1486632b0`),
`NStateTask_MovementWingSuitTask` (`FUN_1405c9950` L553175, `FUN_148663ce0`), `NStateTask_HoverboardTask`
(`FUN_1486212e0`), `NStateTask_InputHoverboardInAirTask` (`FUN_148621090`).

Key helpers (proven): QueueAct `FUN_140566980` → `FUN_1402870a0`; flag-test `FUN_140fd13a0`; input dispatchers
`FUN_14862f290`, `FUN_1405d8bb0`, `FUN_1405d8f60`, `FUN_148667d80`; integrators `FUN_140581340` (wingsuit/air),
`FUN_14058c3c0` (parachute); wingsuit boost `FUN_1485c5fb0`; hoverboard steering `FUN_148617e20`; hoverboard
gate setter `FUN_148d460e0` (`DAT_142cbc3d9`); input-controller builder `FUN_140b149f0`
(`player_parachute_input` L1310311, `player_wingsuit_input` L1310344, `player_hoverboard_input` L1310359,
`player_hoverboard_inair_input` L1310362); control-options parse near `FUN_140bab800`.

Config: `SCustomMovementSettings::m_WingsuitSettings.m_AirControl.m_Gravity` (anti-tamper, `FUN_1405d8f60`
L557846); parachute close lifetime `3600.f` (anti-tamper, `FUN_148458000` L3868875).

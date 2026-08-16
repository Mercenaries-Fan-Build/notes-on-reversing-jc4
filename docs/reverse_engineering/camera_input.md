# Camera & Input — follow-camera modifier stack, camera shake/FOV, and the device→action input system

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative). "proven" = read
directly from the Ghidra decomp (`output/_ghidra_jc4/jc4_all_functions_decomp.txt`); "inferred" = strong
structural deduction from surrounding code; "speculative" = plausible but unconfirmed. Every claim cites a
`FUN_<addr>` / `DAT_<addr>` so it is re-verifiable.

> **Builds on [traversal_movement.md](traversal_movement.md) §5.** That doc established (proven) that
> `CMoveInput{Yaw,Pitch,Offset}Modifier` / `CRollModifier` / `CAlignToVelocityModifier` are **camera**
> modifiers under `source/project/game/camera/cameramodifiers/`, *not* locomotion modifiers, and worked out
> the per-modifier yaw/pitch/offset/roll math. **This doc does not repeat that math.** It documents the
> layers around it: the camera manager pipeline, how the modifiers are registered + bound to config, the
> FOV-override and camera-shake mechanisms, and the full input system (device abstraction → action maps /
> input contexts → the move-input vector and button actions, plus UI-vs-gameplay routing).

---

## Overview

JC4's camera is a **manager + modifier-stack** design (Apex-lineage). `CCameraManager` runs a per-frame
pipeline whose phases are visible as two hashed blackboard persistence keys —
`BLACKBOARD_PERSISTENCE_ID_CAMERA_MANAGER_POST_SELECTION` and `..._POST_UPDATE_CONTEXTS` (`FUN_142f01370`,
`FUN_142f016b0`, proven) — i.e. **select active camera → update camera contexts → apply modifiers → post
publish**. The follow camera's orientation/position/FOV is produced by a stack of small **camera modifiers**,
each a reflected class registered by name from `camera/cameramodifiers/*.cpp` and bound to a data-driven
config asset (`camera_modifier_*`). Camera **shake** is a separate subsystem driven by `CCameraEffectEmitter`
game objects and three preset console commands (`effect_shake_small/medium/large`). Camera **FOV** is applied
by reading a hashed `float_fov` attribute out of the camera context and scaling it (`FUN_1404e2600`, proven).

Input is a classic **device → action-map** stack. `CInputSystem` loads three config assets
(`input/keymapdefs.keymapdefsc`, `input/keymap.inputc`, `input/input.inputc`) and installs axis transforms
(`sensitivity_x/y`, `invert_x/y`). Raw device input is resolved against named **input contexts**
(`player_onground_input`, `player_parachute_input`, … — one per locomotion state, plus `player_global_input`
always-on) that the player input controller (`FUN_140b149f0`) holds as an array of handles and swaps as the
character changes state. Actions become the move-input vector (`move_x`/`move_y`, `global_move_y`, …) and
button actions consumed by the behavior graph. **UI input** is routed separately: `CUIInputManager` forwards
raw input into Scaleform GFx events (`scaleform.gfx.MouseEventEx`, `KeyboardEventEx`, `GamePadAnalogEvent`, …),
and the `activate_ui_input` / `deactivate_ui_input` commands toggle capture between gameplay and menus.

Consistent with the project-wide finding: **the executable holds the mechanism; the magnitudes live in data.**
Camera-modifier rates/thresholds and shake amplitudes are RTPC/ADF config, and the key-bindings are
data-driven (`NInput::SMappingDef`/`SKeyslot`/`SKeymapBundle`, `CSettingsManager::SInputPlayerBindings`).

---

## Key classes & functions

### Camera

| Name (string) | Registration / implementation | Role | Grade |
|---|---|---|---|
| `CCameraManager` | system-registrar `FUN_148f958e0` (registers name alongside `CInputSystem`, `CGraphicsEngine`, …) | The camera singleton; runs select/update-contexts pipeline | proven |
| `CCameraObject` | reflected class id `DAT_142cb0824 = FUN_140f27f60("CCameraObject",0xd)` (`FUN_140281520` cluster) | A placeable/scriptable camera instance | proven |
| `CCameraEffectEmitter` | reflected class id `DAT_142cb88a8 = FUN_140f27f60("CCameraEffectEmitter",0x14)` (`FUN_140794df0`/`FUN_14085fd00`-family) | Emits camera shake effects | proven |
| `CMoveInputOffsetModifier` | cfg-bind `FUN_142eff900` → `DAT_142cb2c10` = `"camera_modifier_move_offset"` | Camera positional offset from move-input | proven |
| `CMoveInputPitchModifier` | cfg-bind `FUN_142effe40` → `DAT_142cb2ce0` = `"camera_modifier_move_pitch"` | Camera pitch from move-input | proven |
| `CMoveInputYawModifier` | cfg-bind `FUN_142f00040` → `DAT_142cb2c58` = `"camera_modifier_move_yaw"` | Camera yaw from move-input | proven |
| `CRollModifier` | cfg-bind `FUN_142f003f0` → `DAT_142cb2b00` = `"camera_modifier_roll"` | Camera roll (turn/velocity driven) | proven |
| `CAlignToVelocityModifier` | cfg-bind `FUN_142efe5e0` → `DAT_142cb2b70` = `"camera_modifier_align_to_velocity"` | Aligns camera to velocity vector | proven |
| `CAutoPitchModifier` | cfg-bind `FUN_142efe780` → `DAT_142cb2bc8` = `"camera_modifier_auto_pitch"` | Automatic pitch follow | proven |
| `CFollowSpringModifier` | cfg-bind `FUN_142efe8f0` → `DAT_142cb2ac0` = `"camera_modifier_follow_spring"` | Spring-damped follow position | proven |
| `CGenericVehicleModifier` (`VehicleCameraModifier`) | cfg-bind `FUN_142eff630` → `DAT_142cb2b38` = `"camera_modifier_generic_vehicle"` | Vehicle follow camera | proven |
| FOV-override apply | `FUN_1404e2600` (reads hashed `float_fov`) | Reads `float_fov` from camera context, writes FOV | proven |
| Manager pipeline keys | `FUN_142f01370` → `DAT_142cb3894` (POST_SELECTION); `FUN_142f016b0` → `DAT_142cb3a3c` (POST_UPDATE_CONTEXTS) | Blackboard persistence IDs for camera-manager phases | proven |

Additional camera names seen in the string inventory (present, not deep-dived here): `CCameraGlobalAttachmentModifier::Init`, `CCameraSwapWeaponComponent`, `CCameraFollowWeaponComponent`, `CDistanceToCameraCondition` (`DAT_142cb4484`), config keys `vehicle_alternate_camera`, `CameraCentering`, `VEHICLE_CAM_RAY`, `cam.switch.lastcamera`, `SHADOW_REFLECTIVE_CAMERA`, `CAMERA_ANGLE_CORRECTION_SEG`.

### Input

| Name (string) | Registration / implementation | Role | Grade |
|---|---|---|---|
| `CInputSystem` | system-registrar `FUN_148f958e0`; init/loader `FUN_140964680` | Loads keymap/input config, owns axis transforms | proven |
| Input singleton globals | `DAT_142cba918` = `DAT_142cba830` = `DAT_142cb0c98` = `DAT_142ce1af0` (set in `FUN_140964680`) | The live keymap/context registry | proven |
| Player input controller | builder `FUN_140b149f0` (`callers=[FUN_140b03f70]`) | Resolves per-state input contexts into handle slots | proven |
| Action→context bind | `FUN_140e88f50(context, action, ctrl, 1)` (called from `FUN_140e87a20`) | Adds an action into an input context | inferred |
| Context resolve-or-create | `FUN_140fdf660` (lookup) / `FUN_140fdc0c0` (create) / `FUN_140af7430` (helper), all keyed by `DAT_142cba830` | Resolve a named input context handle | proven |
| Gamepad polling thread | `FUN_140f2a140("InputGamepadUpdateThread", &LAB_140fc5a00, …, 0x1000)` (`FUN_140??` @ L1776151) | Dedicated gamepad update thread | proven |
| `CUIInputManager` | reflected `ArGetTypeId<class CUIInputManager>`; system-registrar refs (L3209640, L4139122) | Routes input into Scaleform GFx UI | proven |
| `CUIInputPrompt` | reflected class; string `ui_input_prompt` | On-screen button-prompt UI | proven |
| Player input gating cmds | `FUN_140ae98d0` (`CPlayerManager` cmd block): `ply.input.enable/disable/disable_except_look` | Enable/disable player input, look-only mode | proven |
| Settings read | `FUN_140b964b0` → setter `FUN_140bab800(struct, idx, val)` | Reads controller/mouse options from `CSettingsManager` | proven |

Input reflected data types (in the ADF/settings save path, proven from `ArGetTypeId<…>` strings):
`NInput::SMappingDef`, `NInput::SKeyslot`, `NInput::SKeymapBundle`, `std::pair<SMappingDef,SKeyslot>`,
`SInputActionSaveData`, `CSettingsManager::SInputPlayerBindings`. Save/list asset: `settings/input_action_list.bin`
(loaded in `FUN_140e8dd90`).

---

## How it works (from the decomp)

### 1. The camera-modifier registration + config binding (proven)

Each modifier has a tiny CRT-init function that binds its **config singleton DAT** to a **config-asset name**
via `thunk_FUN_14af1e100(DAT, "camera_modifier_<x>", "<...>.cpp", len, 0)` and installs an `atexit` teardown.
The full map (all in the `FUN_142efe5e0…FUN_142f003f0` block, callers `0x140007cb0…0x140007e70` — the global
init table):

```
FUN_142efe5e0  DAT_142cb2b70  camera_modifier_align_to_velocity   (aligntovelocitymodifier.cpp)
FUN_142efe780  DAT_142cb2bc8  camera_modifier_auto_pitch          (autopitchmodifier.cpp)
FUN_142efe8f0  DAT_142cb2ac0  camera_modifier_follow_spring       (followspringmodifier.cpp)
FUN_142eff630  DAT_142cb2b38  camera_modifier_generic_vehicle     (genericvehiclemodifier.cpp)
FUN_142eff900  DAT_142cb2c10  camera_modifier_move_offset         (moveinputoffsetmodifier.cpp)
FUN_142effe40  DAT_142cb2ce0  camera_modifier_move_pitch          (moveinputpitchmodifier.cpp)
FUN_142f00040  DAT_142cb2c58  camera_modifier_move_yaw            (moveinputyawmodifier.cpp)
FUN_142f003f0  DAT_142cb2b00  camera_modifier_roll                (rollmodifier.cpp)
```

The `DAT_142cb2c10` / `DAT_142cb2ce0` / `DAT_142cb2c58` / `DAT_142cb2b00` here are **the same config DATs** the
traversal doc cited for the move-offset/pitch/yaw/roll modifier Update functions — confirming those Update
bodies read their tunables (thresholds, rates, clamps) out of the `camera_modifier_*` config asset bound here,
not from code literals (proven cross-check with traversal_movement.md §5). The three *new* modifiers surfaced
by this doc are **follow-spring** (`DAT_142cb2ac0`), **auto-pitch** (`DAT_142cb2bc8`), and
**align-to-velocity** (`DAT_142cb2b70`) — the follow-camera's core spring + auto-pitch behaviour.

### 2. Camera manager pipeline (partial — manager tick is walled) (proven / inferred)

`CCameraManager` is registered by name in the system-registrar `FUN_148f958e0` (same table as `CInputSystem`,
`CGraphicsEngine`, `CClock`). Its per-frame structure is visible only through two hashed blackboard keys it
publishes to (`thunk_FUN_14aadec10` = HashString register, the same primitive as `float_fov`):

- `DAT_142cb3894` = `hash("BLACKBOARD_PERSISTENCE_ID_CAMERA_MANAGER_POST_SELECTION")` (`FUN_142f01370`)
- `DAT_142cb3a3c` = `hash("BLACKBOARD_PERSISTENCE_ID_CAMERA_MANAGER_POST_UPDATE_CONTEXTS")` (`FUN_142f016b0`)

These name two ordered phases: **camera selection** (choose the active camera / camera context among candidates)
then **context update** (run the active camera context, whose modifier stack produces the final transform).
The manager then applies the modifier stack to the follow camera; the traversal doc proved the apply path
(`FUN_148234330` extracts yaw/pitch and writes camera deltas via `thunk_FUN_147b11c50(cam+0xc8, dYaw, dPitch,
dRoll)`, camera yaw at `cam+0xc8`, pitch at `cam+0xcc`). **The manager's own tick body and camera-mode/stack
selection logic are not recoverable** from the functions-only export (data-section vtable + omitted ctor) —
see Open Questions. What *is* proven is the modifier layer and the pipeline phase ordering.

### 3. FOV override (proven)

`FUN_1404e2600` is a camera-modifier apply: it composes a 4×4 matrix into the camera context (`param_4[0..0xf]`)
then reads an FOV attribute out of the context's typed attribute set:

```c
// thread-safe one-time hash of the attribute name
_DAT_142cb2d5c = thunk_FUN_14aadec10("float_fov");                 // hashed attribute id
iVar18 = thunk_FUN_147b1f2b0(uVar19, _DAT_142cb2d5c & 0xffff);     // find attribute slot
if (-1 < iVar18) {
    fVar23 = (float)thunk_FUN_147b1f060(uVar19, iVar18);           // read float value
    if (0.0 < fVar23)
        param_4[0x30] = fVar23 * DAT_141ca6c98;                    // write FOV (scaled)
}
```

So **FOV is a named `float_fov` attribute** carried in the camera context; when present and positive it
overrides the camera FOV at struct offset `+0x30` (in float units, i.e. `param_4[0x30]`), scaled by the
constant `DAT_141ca6c98` (a degrees→radians / half-angle factor, inferred — the constant is shared by 296 call
sites). This is the hook a mod would use to force FOV: inject/override the `float_fov` context attribute.

### 4. Camera shake (proven mechanism, walled magnitudes)

Camera shake is emitted by `CCameraEffectEmitter` (reflected game-object class `DAT_142cb88a8`). A shake
**manager** (`DAT_142cafd50`, set up in the large init `FUN_140281520`) registers three preset trigger
commands and preloads their preset data:

```c
thunk_FUN_14762d8a0(lVar12 + 0xc78, "effect_shake_small",  0xff, 1);
thunk_FUN_14762d8a0(lVar12 + 0xc80, "effect_shake_medium", 0xff, 1);
thunk_FUN_14762d8a0(lVar12 + 0xc88, "effect_shake_large",  0xff, 1);
// then bulk-copies preset blocks _DAT_142abe0a0.. into manager offsets +0xc30..+0xc5c
```

`thunk_FUN_14762d8a0(target, name, 0xff, 1)` is the **console/debug-command register** primitive (name,
permission level `0xff`, enabled `1`) — the same primitive used for all `ply.*` commands. The preset
amplitude/frequency/duration values live in the `_DAT_142abe0a0…_142abe0dc` data blocks (data-section,
**walled** — magnitudes not in the functions export). The mechanism (three named shake intensities driven by
emitter objects, copied into the manager at fixed offsets) is proven.

### 5. Input system init & axis transforms (proven)

`FUN_140964680` is `CInputSystem` init. It:

1. Loads the three input config assets in one call:
   `FUN_140fe8cb0(obj, …, "input/keymapdefs.keymapdefsc", "input/keymap.inputc", "input/input.inputc")`.
2. Stores the resulting keymap/context registry into the input singleton globals
   `DAT_142cba918 = DAT_142cba830 = DAT_142cb0c98 = DAT_142ce1af0` (proven — these aliases are how the rest of
   the game reaches the input system; e.g. the player controller uses `DAT_142cba830`).
3. Installs four **axis transforms** by name, each with a code-pointer handler:
   `thunk_FUN_14af24ec0(input, "sensitivity_x", &PTR_LAB_141dc33c0)`, then `"sensitivity_y"`
   (`PTR_LAB_141dc3430`), `"invert_x"` (`PTR_LAB_141dc34a0`), `"invert_y"` (`PTR_LAB_141dc3510`).

These are the per-axis processors that turn raw stick/mouse deltas into the normalized aim/move axes,
parameterised by the settings values (below).

A dedicated **gamepad polling thread** is spawned: `FUN_140f2a140("InputGamepadUpdateThread",
&LAB_140fc5a00, param, 0x1000)` — device polling runs off the main thread.

### 6. Input contexts = action maps, one per locomotion state (proven)

The player input controller builder `FUN_140b149f0` (4632 bytes) resolves **14 named input contexts** and
stores each resolved handle into a fixed slot of the controller struct. Resolution is
`FUN_140fdf660(DAT_142cba830, &out, hash(name))` (find existing) else `FUN_140fdc0c0(DAT_142cba830, …)`
(create); the helper `FUN_140af7430(pool, out, "name")` wraps this. Slot map:

```
+0x3c0  player_onground_input
+0x3d0  player_parachute_input
+0x3e0  player_wingsuit_input
+0x3f0  player_falling_input
+0x400  player_freefalling_input
+0x410  player_surface_swimming_input
+0x420  player_underwater_swimming_input
+0x430  player_hoverboard_input
+0x440  player_hoverboard_inair_input
+0x450  player_ragdolling_input
+0x460  player_reeling_in_input
+0x470  player_global_input           (always-on base map)
+0x480  player_camera_interest_input
+0x490  player_interactions
```

This is the **action-map / input-context stack**: each locomotion state (which maps 1:1 to the state-tasks in
traversal_movement.md — `player_parachute_input` ↔ parachute task, `player_hoverboard_input` ↔ hoverboard,
etc.) activates its own context so the same physical buttons mean different actions per state, layered over the
always-on `player_global_input`. `player_camera_interest_input` is the camera "look-at point-of-interest" map.

Individual actions are bound into contexts by `FUN_140e88f50(contextId, actionId, controller, 1)`. Example
from `FUN_140e87a20`: it binds `move_y` / `global_move_y` / `reeled_in` actions, gated on a mode flag
`*(DAT_142cb7dc8 + 0x59c)` (proven) — i.e. the **move-input vector** is assembled from named axis actions
(`move_x`/`move_y`, and a `global_move_y` variant used while reeling), and these feed both locomotion and the
`CMoveInput*Modifier` camera modifiers documented in the traversal doc.

### 7. Input gating (proven)

`CPlayerManager` command block `FUN_140ae98d0` registers the runtime input-gating commands via the same
command-register primitive:

```c
thunk_FUN_14762d8a0(param_1 + 0x141, "ply.input.enable",              0xff, 1);
thunk_FUN_14762d8a0(param_1 + 0x142, "ply.input.disable",             0xff, 1);
thunk_FUN_14762d8a0(param_1 + 0x143, "ply.input.disable_except_look", 0xff, 1);
```

`disable_except_look` is the cutscene/scripted-moment mode that suppresses movement/action input while leaving
camera look active (proven from the command name + its placement in the player command table).

### 8. UI input routing (Scaleform) (proven)

UI input is a **separate path** from gameplay actions. `CUIInputManager` forwards raw device events into
Scaleform GFx as named events (all present as string constants): `scaleform.gfx.MouseEventEx`,
`scaleform.gfx.MouseCursorEvent`, `scaleform.gfx.KeyboardEventEx`, `scaleform.gfx.TextEventEx`,
`scaleform.gfx.FocusEventEx`, `scaleform.gfx.GamePadAnalogEvent`. The ActionScript-side analog callback is
named `gamePadAnalogChange` (`FUN_141a??` @ L2836172 sets `puVar2[10] = "gamePadAnalogChange"`), and menus get a
gamepad-driven cursor via `CreateGamepadCursor`. Text entry uses `KB_TEXTINPUT`.

The switch between gameplay and UI input is toggled by two commands registered in the overlay/scaleform builder
`FUN_140e8e920` (which also loads `ui/overlay.swf`):

```c
thunk_FUN_14762d8a0(param_1 + 0x578, "activate_ui_input",   3, 1);
thunk_FUN_14762d8a0(param_1 + 0x580, "deactivate_ui_input", 3, 1);
```

So opening a menu calls `activate_ui_input` (capture → GFx), closing calls `deactivate_ui_input` (release →
gameplay contexts). (proven mechanism; the exact capture/priority semantics are inferred.)

---

## Data & config integration

- **Camera-modifier tunables** are the `camera_modifier_*` config assets bound in §1 (RTPC/ADF). Thresholds,
  rates, clamps, spring constants, auto-pitch curves, FOV values → all data-side (proven the binding exists;
  values walled).
- **FOV** is a `float_fov` typed attribute on the camera context (§3), i.e. authored per camera/context in
  data, not a code literal.
- **Key bindings** are reflected ADF/save types: `NInput::SMappingDef` (action→physical mapping),
  `NInput::SKeyslot` (a bindable slot), `NInput::SKeymapBundle` (a set of maps), aggregated into
  `CSettingsManager::SInputPlayerBindings` and `SInputActionSaveData` (`settings/input_action_list.bin`). This
  is the rebind system's persistence — bindings are data, editable/serialisable (proven from `ArGetTypeId<…>`).
- **Controller/mouse options** are read from `CSettingsManager` by `FUN_140b964b0` and applied through
  `FUN_140bab800(struct, index, value)`. Recovered option→index map (proven):

  | idx | option | idx | option |
  |---|---|---|---|
  | 0x1e | ControllerVibration | 0x27 | PCUseGamepad |
  | 0x1f | ControllerInvertAimingY | 0x28 | PCGamepadExclusiveLockControllerMode |
  | 0x20 | ControllerInvertAimingX | 0x29 | MouseInvertAimingX |
  | 0x21 | ControllerFlightYAxis | 0x2a | MouseInvertAimingY |
  | 0x22 | ControllerFireButton | 0x2b | MouseSensitivityX |
  | 0x23 | ControllerSensitivityPitch | 0x2c | MouseSensitivityY |
  | 0x24 | ControllerSensitivityYaw | 0x2d | CameraCentering |
  | 0x25 | ControllerCrouchToggle | | |
  | 0x26 | ControllerDisableInputWhenOutOfFocus | | |

  The serialize-out counterpart (`FUN_140b4e…` block @ L1392084+) writes these back from the settings struct;
  observed struct offsets: `ControllerVibration@+0xf0`, `ControllerInvertAimingX@+0xf8`,
  `ControllerFlightYAxis@+0xfc`, `ControllerSensitivityYaw@+0x108`, `PCUseGamepad@+0x114`,
  `MouseSensitivityX@+0x124` (proven). `CameraCentering@+0x12c`-region controls auto-recenter (config key also
  read at the gameplay side, L1377071/L1392101).

---

## Notable constants / tunables

| Constant | Meaning | Where | Grade |
|---|---|---|---|
| `DAT_141ca6c98` | FOV scale factor (deg→rad / half-angle) applied to `float_fov` | `FUN_1404e2600` (296 call sites total) | inferred |
| `_DAT_142cb2d5c` | cached hash id of `"float_fov"` attribute | `FUN_1404e2600` | proven |
| `DAT_142cb3894` / `DAT_142cb3a3c` | camera-manager POST_SELECTION / POST_UPDATE_CONTEXTS blackboard ids | `FUN_142f01370` / `FUN_142f016b0` | proven |
| `DAT_142cb2ac0/…/2ce0` | the 8 camera-modifier config singletons (see §1 table) | `FUN_142efe5e0…FUN_142f003f0` | proven |
| `_DAT_142abe0a0…0dc` | shake preset data (amplitude/freq/duration) copied to shake mgr `+0xc30..+0xc5c` | `FUN_140281520` | proven (values walled) |
| `DAT_142cba830` (= `142cba918/…`) | input keymap/context registry singleton | `FUN_140964680` | proven |
| `DAT_142cb7dc8 + 0x59c` | input-mode flag gating action binding (`move_y` vs `global_move_y`) | `FUN_140e87a20` | proven |
| settings idx `0x1e–0x2d` | controller/mouse/camera option indices | `FUN_140b964b0`/`FUN_140bab800` | proven |
| `DAT_141cb8224` | angle-unit scale used by camera modifiers (from traversal doc) | `FUN_148274e30` etc. | inferred |

(Per-modifier rate/threshold/clamp constants — `cfg[+0x08/+0x10/+0x1c/+0x20/+0x24/+0x28..0x38]` — are covered in
traversal_movement.md §5 and are data-driven config, not code literals.)

---

## Call-graph highlights

- Global init table (`callers 0x140007cb0…0x140007e70`) → the 8 `FUN_142efe5e0…FUN_142f003f0` modifier
  config-binders → `thunk_FUN_14af1e100(DAT, "camera_modifier_*", …)`.
- `FUN_148f958e0` (system registrar) registers `CInputSystem` **and** `CCameraManager` names (they sit in the
  same manager list — the two systems are siblings at registration).
- `CInputSystem` init `FUN_140964680` → `FUN_140fe8cb0` (load 3 input assets) → sets `DAT_142cba830` → binds
  `sensitivity_x/y`, `invert_x/y`.
- Player controller `FUN_140b03f70` → `FUN_140b149f0` (build) → `FUN_140af7430`/`FUN_140fdf660`/`FUN_140fdc0c0`
  (resolve 14 input contexts) ; per-action bind via `FUN_140e88f50` (from `FUN_140e87a20`/`FUN_140e87c30`).
- `FUN_1404e2600` (FOV apply) → `thunk_FUN_147b1f2b0`/`thunk_FUN_147b1f060` (context attribute find/read).
- Shake: `FUN_140281520` → registers `effect_shake_*` on mgr `DAT_142cafd50`; `CCameraEffectEmitter`
  (`DAT_142cb88a8`) emits at runtime.
- UI: `FUN_140e8e920` (overlay builder) registers `activate/deactivate_ui_input`; `CUIInputManager` → Scaleform
  `scaleform.gfx.*` events.

---

## Open questions / lower-confidence

- **`CCameraManager` tick body + camera-mode/stack selection is walled.** Only the two pipeline-phase blackboard
  keys and the modifier layer are recoverable from the functions-only export; the manager's own update loop and
  how it *chooses* the active camera (third-person follow vs vehicle vs aim vs cutscene) sit behind a
  data-section vtable / omitted ctor. **Resolve route:** live trace in x64dbg — break on the modifier apply
  `FUN_148234330` or on a write to a camera object's `+0xc8` (yaw), walk the caller frames to the manager tick;
  or ingest the manager's ADF camera-context assets.
- **FOV scale `DAT_141ca6c98`** — graded inferred as deg→rad/half-angle; the exact float is in `.rdata` (not in
  the functions export). Resolve: read the constant at `0x141ca6c98` in the mapped binary.
- **Shake preset magnitudes** (`_DAT_142abe0a0…`) — mechanism proven, values walled (data section). Resolve:
  dump the `.data` block, or read the `camera_effect`/shake config asset.
- **`FUN_140e88f50` bind semantics** (does arg4=`1` mean "override" vs "append"?) — inferred; confirm by reading
  the callee body.
- **Camera-modifier stack ordering / which modifiers are active per camera mode** — the per-mode composition
  (e.g. vehicle uses generic_vehicle + follow_spring + roll) is data-driven in the camera-context assets; not
  yet enumerated from code.
- **`CameraCentering` gameplay behaviour** (auto-recenter timing) — the option is read (idx 0x2d) but the
  consuming logic wasn't traced.

---

## Appendix — decomp anchors

Camera modifiers (config binders): `FUN_142efe5e0`(align_to_velocity/`DAT_142cb2b70`),
`FUN_142efe780`(auto_pitch/`DAT_142cb2bc8`), `FUN_142efe8f0`(follow_spring/`DAT_142cb2ac0`),
`FUN_142eff630`(generic_vehicle/`DAT_142cb2b38`), `FUN_142eff900`(move_offset/`DAT_142cb2c10`),
`FUN_142effe40`(move_pitch/`DAT_142cb2ce0`), `FUN_142f00040`(move_yaw/`DAT_142cb2c58`),
`FUN_142f003f0`(roll/`DAT_142cb2b00`) — L3079214–3079336.
Modifier cpp paths: `source\project\game\camera\cameramodifiers\{aligntovelocity,autopitch,followspring,genericvehicle,moveinputoffset,moveinputpitch,moveinputyaw,roll}modifier.cpp`.

Camera manager: name in `FUN_148f958e0` (L4138351); pipeline keys `FUN_142f01370`(POST_SELECTION `DAT_142cb3894`, L3079369), `FUN_142f016b0`(POST_UPDATE_CONTEXTS `DAT_142cb3a3c`, L3079381). `CCameraObject` `DAT_142cb0824` (L180986). `CCameraEffectEmitter` `DAT_142cb88a8` (L803888/L861369). `CDistanceToCameraCondition` `DAT_142cb4484` (L502320).

FOV: `FUN_1404e2600` reads `"float_fov"` → `_DAT_142cb2d5c`, writes `param_4[0x30]*DAT_141ca6c98` (L458635–458772).

Shake: `FUN_140281520` registers `effect_shake_small/medium/large` on `DAT_142cafd50` (+0xc78/0xc80/0xc88), presets `_DAT_142abe0a0…` (L184197–184219).

Input system: `FUN_140964680` loads `input/keymapdefs.keymapdefsc` + `input/keymap.inputc` + `input/input.inputc`, sets `DAT_142cba918/830/…`, binds `sensitivity_x/y`,`invert_x/y` (L1074769–1074851). Gamepad thread `InputGamepadUpdateThread` (L1776151). Save asset `settings/input_action_list.bin` (`FUN_140e8dd90`, L1702908).

Input contexts: builder `FUN_140b149f0` (L1309998), context names + slots L1310243–1310378. Action bind `FUN_140e88f50` via `FUN_140e87a20` (L1698947), `player_global_input` L1699020.

Input gating: `FUN_140ae98d0`, `ply.input.enable/disable/disable_except_look` (L1283625–1283632).

Settings: `FUN_140b964b0` reads options idx 0x1e–0x2d via `FUN_140bab800` (L1377011–1377074); serialize-out `FUN_140b4e…` (L1392084–1392101).

UI input: `CUIInputManager` (L3209640, L4139122); Scaleform events `scaleform.gfx.{Mouse,Keyboard,GamePadAnalog,Text,Focus,MouseCursor}Event(Ex)`; `gamePadAnalogChange` (L2836172); `activate/deactivate_ui_input` in `FUN_140e8e920` (L1704077–1704080).

Reflected input types: `ArGetTypeId<NInput::SMappingDef | SKeyslot | SKeymapBundle | pair<SMappingDef,SKeyslot>>`, `ArGetTypeId<SInputActionSaveData>`, `ArGetTypeId<CSettingsManager::SInputPlayerBindings>`.

Shared primitives: `FUN_140f27f60` = name→type/HashString registration (6,138 call sites; key = lookup3 `hashlittle`, per README). `thunk_FUN_14aadec10` = HashString register (used for `float_fov`, blackboard ids). `thunk_FUN_14762d8a0(target,name,perm,enabled)` = console/debug-command register.

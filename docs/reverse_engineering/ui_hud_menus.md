# UI, HUD & Menus — Scaleform-driven front-end, HUD, notifications, tutorials, map & world-space UI

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative). "proven" = read
directly from the decomp; `FUN_<addr>` cited so any claim is re-verifiable against
`output/_ghidra_jc4/jc4_all_functions_decomp.txt`.

> **Methodology caveat (proven).** This is a *functions-only* export. What survives: the registration
> skeletons, the call graph, the reachable CPU-side logic, and — crucially for UI — the **string constants**:
> Scaleform movie file names, GPU/profiler zone names, AS3 event-class names, and the **game→ActionScript
> method-invocation names** ("ShowUserWaypoint", "UpdateHealth", …). What is **NOT** here: the per-widget
> layout, the visual logic, and the actual widget update code — those live inside the Scaleform `.gfx`/`.swf`
> assets (compiled ActionScript), not in the executable. Every such wall is tagged **[walled: in .gfx asset]**.

---

## Overview

JC4's entire 2D front-end — main menu, HUD, map, notifications, tutorials, button hints — is built on
**Autodesk Scaleform GFx 4.x** (the `Scaleform::Render::*`, `GFx_*` loader, and `scaleform.gfx.*` AS3 event
strings are all present in the binary; the render backend is `Scaleform::Render::D3D1x` = Direct3D 11).
(proven — `Scaleform` / `GFx` strings at FUN_141933e90, FUN_1417*, FUN_2881xxx render-HAL functions.)

The C++ side is a thin driver around this:

- **`CUIManager`** (singleton `DAT_142cb7dc8`) owns the loaded Scaleform movies, advances (ticks) them each
  frame, and renders them — both directly to the backbuffer and to offscreen render targets. Registered under
  the reflected class name `CUIManager` (hash `0x41d8e380`). (proven — FUN_148f960c0, FUN_140ea71d0, FUN_140e9fb80)
- **The bridge is bidirectional and string-keyed.** Game → AS: the game marshals arguments into an array of
  **`GFxValue` (48-byte / `0x30`) slots** and calls the movie object's **`Invoke("MethodName", args, argc, …)`**
  virtual. AS → game: input is delivered by constructing **AS3 event objects** (`scaleform.gfx.MouseEventEx`,
  `KeyboardEventEx`, `GamePadAnalogEvent`, `FocusEventEx`, `TextEventEx`) and dispatching them into the movie.
  (proven — invoke pattern in FUN_140e60340 etc.; event-class cache in FUN_141933e90)
- The **HUD, map, notifications, tutorials and button hints are not separate movies with separate C++
  classes so much as separate *method families* invoked on the UI movies** — the C++ managers
  (`CNotificationManager`, `CButtonHintManager`, `CTutorialManager`, `CGPSController`) collect game state and
  push it across the invoke bridge. (inferred from the invoke-name families; the receiving widgets are
  **[walled: in .gfx asset]**.)
- **World-space UI** (health bars over enemies, interaction outlines, inspectable prompts) is a separate,
  non-Scaleform path implemented as **entity components** (`CHealthbarComponent`, `COutline`,
  `CInspectableComponent`) registered through the component registrar. (proven — FUN_148f99a90, FUN_14085fd00)

---

## Key classes & functions

| String / class | FUN_ that registers/implements | Role |
|---|---|---|
| `CUIManager` (hash `0x41d8e380`) | registered FUN_148f960c0 / FUN_146cd0bea; render FUN_140ea71d0; tick FUN_140e9fb80; singleton `DAT_142cb7dc8` | Owns + advances + renders all Scaleform movies |
| `CUIManagerGO` | factory-lazy-init FUN_140846830 (`FUN_140f27f60("CUIManagerGO",…)`) | The UI manager "game object" reflected type |
| `CUIController` (hash tag `0x0d`) | FUN_140e85bf0 / FUN_140815410 (`FUN_140f27f60("CUIController",…)`) | UI controller reflected type |
| `CUIInputManager` | mgr-registry FUN_148f960c0 (vtable `PTR_LAB_141d90ca8`); type-id FUN_140e701c0 | Routes raw input → Scaleform AS3 events |
| `CHUDUI` | HUD compositor FUN_140e64120 (double-buffered target tagged `"CHUDUI"`) | HUD render/compositing layer |
| Scaleform bootstrap | FUN_140e8e920 | Loads `ui/shared_lib.swf`, `ui/overlay.swf`, `text/master` |
| Movie register | FUN_140ea3600 (`Add movie(nameHash, label, handle, prio)`) | Inserts a movie into `CUIManager`'s lists |
| Movie load | FUN_140e827c0 | Creates/loads a GFx movie instance from a path |
| Title/menu movie | FUN_1408572c0 (loads `ui\title.gfx`) | Front-end title screen |
| AS3 event-class cache | FUN_141933e90 (`scaleform.gfx.*Event*`) | Caches AS3 event classes for input injection |
| `CNotificationManager` | mgr-registry FUN_148f960c0 (vtable `PTR_LAB_141d90a48`); getter FUN_147fc30ec | Toast/notification queue |
| `CButtonHintManager` | mgr-registry FUN_148f960c0 (vtable `PTR_LAB_141d90ba8`); type-id FUN_140adbf60 | Context button-prompt hints |
| `CTutorialManager` | mgr-registry FUN_148f960c0 (vtable `PTR_LAB_141d90ce8`); type-id FUN_140bc11f0 | Tutorial boxes/arrows/images |
| `CGPSController` (hash tag `0x0e`) | FUN_14080caf0 (`FUN_140f27f60("CGPSController",…)`) | GPS route / waypoint pathing to the map |
| `CVideoManager` (hash `0x41d8e998`) / `CVideoRecordingManager` (`PTR_LAB_141d90ac8`) | FUN_148f960c0 | Full-motion video / capture |
| `CHealthbarComponent` | component-registry FUN_148f99a90 (`thunk_FUN_147cafcd0`) | World-space enemy/vehicle health bar |
| `COutline` | FUN_1407db240 (`FUN_140f27f60("COutline",8)`); component path FUN_14085fd00 | World-space silhouette/highlight |
| `CInspectableComponent` (hash tag `0x15`) | FUN_14080d250 (`FUN_140f27f60("CInspectableComponent",…)`) | "Inspect / interact" world prompt |
| `CFpsCounter` | FUN_148f960c0 (vtable `PTR_LAB_141d90b68`) | Debug FPS overlay |

---

## How it works (from the decomp)

### 1. Scaleform bootstrap — which movies are loaded (proven)

`FUN_140e8e920` is the UI/Scaleform init. It loads the localization table and the two always-resident base
movies, registering each into the manager via `FUN_140ea3600(mgr, nameHash, label, movieHandle, …)`:

| Path | Label | Name hash (arg2 of FUN_140ea3600) | Priority (arg) |
|---|---|---|---|
| `text/master` | (localization master) | — | — |
| `ui/shared_lib.swf` | `shared_lib` | `0x684f18a3` | `99` |
| `ui/overlay.swf` | `overlay` | `0xf7eaa450` | `0x6e` (110) |

(proven — FUN_140e8e920 lines ~1703950–1704045.) The front-end title screen is a separate movie
`ui\title.gfx` loaded by `FUN_1408572c0` (proven). The name hashes are 32-bit and consistent with the
project's `lookup3 hashlittle` identity (memory `[[name-hash-cracked]]`); label→hash equivalence is
**inferred** (not byte-verified here).

`FUN_140ea3600` (the "register movie" function, 1139 bytes) takes the movie handle plus flags (a priority
byte and several booleans) and links it into one of the `CUIManager` movie lists. It is also called from
`FUN_140eb29f0`, i.e. movies can be added dynamically after boot. (proven)

### 2. CUIManager render (proven — FUN_140ea71d0)

`FUN_140ea71d0(CUIManager*, renderCtx*)` is the render entrypoint. It is guarded by three instance flags
(`+0x2b1`, `+0x2b2`, `+0x5a2` all non-zero) and walks **three movie lists** on the manager:

- `+0x70..+0x71` — the primary movie array (rendered in *"CUIManager_RenderToTarget_Render"*).
- `+0x9f..+0xa0` — a second array (rendered in *"CUIManager_RenderDynUI"*).
- `+0xa2..+0xa3` — offscreen movies (rendered under the *"SCALEFORM_OFFSCREEN"* pass).

Those quoted strings are **GPU debug/profiler scope markers** pushed via the render context (they appear as
`_guard_check_icall(renderCtx, "…")` — a decompiler artifact for the marker-push indirect call). Per movie
that is renderable, it also pushes the movie's own **name string** as a marker (SSO `std::string` at movie
`+0x2c`). The actual draw is the movie object's virtual at vtable `+0x20`/`+0x88`/`+0x90` sequence (begin /
draw / end), targeting a render-target surface. (proven — FUN_140ea71d0 lines 1718543–1718767.)

The renderer also has dedicated **profiler/render-pass zones** for UI: an enum→name switch returns
`"UI"`, `"UI_Texture"`, `"UI_ActionScript"`, `"UI_Video"` (cases 10–13). (proven — FUN around line 3321564.)

### 3. CUIManager tick / Advance (proven — FUN_140e9fb80)

`FUN_140e9fb80(CUIManager*, dt, …)` is the per-frame update. Same flag guard as render
(`+0x2b0/+0x2b1/+0x2b2/+0x5a2`), then it iterates the movie list at `+0x380..+0x388` and advances each movie
(refcount bump + call through the movie vtable). Its callers include a manager tick loop `FUN_148fe9ec0` and
the boot path `FUN_14086d1e0`. `DAT_142cb7dc8` is the manager singleton (`FUN_140e9fb80(DAT_142cb7dc8, dt)`
at line 918806). (proven)

### 4. HUD compositing (proven — FUN_140e64120)

`FUN_140e64120` is the HUD compositor. It **double-buffers** two HUD surfaces, acquiring/releasing them under
the resource tag `"CHUDUI"` (`thunk_FUN_14af0c550`/`14af0c7a0` = acquire/lock, `FUN_140fd14d0` = readiness
test) and picking the front/back buffer per frame. The chosen surface is then composited. The individual HUD
elements it drives are pushed via the invoke bridge (see §5); the visual layout itself is **[walled: in
.gfx asset]**. (proven for the compositing/double-buffer mechanism; walled for element visuals.)

### 5. Game → ActionScript: the Invoke bridge (proven)

The single most important mechanism. To push data into a movie the game builds an array of **`GFxValue`
slots — 48 bytes (`0x30`) each** — marshals its arguments in, then calls the movie/widget object's **Invoke
virtual**, passing the AS3 method name as a string plus the arg array and count:

```c
// FUN_140e60340 (HUD health):
(**(code **)(*param_1 + 0x20))(param_1, "UpdateHealth", local_a8, 2, 0, 0);
_eh_vector_destructor_iterator_(local_a8, 0x30, 2, FUN_1408d8030);   // 2 × GFxValue(0x30), dtor

// FUN_140dde200 (waypoint):
(**(code **)(*param_1 + 0x38))(param_1, "ShowUserWaypoint", local_88, 2, 0, 0);
_eh_vector_destructor_iterator_(local_88, 0x30, 2, FUN_1408d8030);
```

The `0x30`-stride vector destructor with element-dtor `FUN_1408d8030` confirms the arg element is a
Scaleform `GFx::Value` (48 bytes). The **vtable offset of Invoke varies by wrapper type** (`+0x20` on the HUD
object, `+0x38` on the map/waypoint object) but the **calling convention is uniform**: `(name, GFxValue[],
argc, …)`. (proven.)

**The method-name string table *is* the HUD/menu interface surface.** Harvested game→AS invoke names, grouped
by subsystem (all proven present as string constants; the FUN_ builds+sends them):

- **Health / status HUD:** `UpdateHealth` (FUN_140e60340), `UpdateOxygen`, `HideEmptyIndicator` /
  `ShowEmptyIndicator`, `DisplayHudState` (FUN_140b964b0).
- **Reticle / aiming:** `UpdateWeaponReticle`, `SetReticlePosition` (FUN_140ebb120), `UpdateGrappleReticle`,
  `ShowFixedAimReticles`.
- **Compass / waypoint:** `UpdateCompass` (FUN_140e614f0), `ShowUserWaypoint` / `HideUserWaypoint`
  (FUN_140dde200 / FUN_1596xxx), plus SFX cues `sfx_gui_map_place_waypoint` / `sfx_gui_map_remove_waypoint`
  and legend key `gui_map_legend_waypoint`.
- **Map POI (points-of-interest) — a whole push-based sub-protocol** (FUN_140e08170): `PushPOIState`,
  `PushPOIShowLabel`, `PushPOIFlashing`, `PushPOIDisplayType`, `PushPOIBackground`, `PushPOICollapsed`,
  `PushPOIChildIndex`, `PushPOIAOI`, `PushPOIUnavailable`, `UpdateLightPOI` / `UpdateHeavyPOI`,
  `RemoveLightPOI` / `RemoveHeavyPOI`.
- **Button hints:** `ShowButtonHints` / `HideButtonHints` (FUN_140dde200 / FUN_140e64* area),
  `UpdateButtonHints`, `SetRetoolerWidgetButtons`, `map_button_hint_toggle_waypoint`.
- **Tutorials:** `RequestTutorialBox` (FUN_140dbad00), `FreeTutorialBox`, `SetTutorialBoxInput`,
  `SetTutorialBoxSize`, `SetTutorialBoxText`, `SetTutorialImages`, `SetTutorialTexts`, `RequestTutorialArrow`,
  `FreeTutorialArrow`, `GetTutorialLocations`, `UpdateSkipCutsceneHint`.
- **Retooler / loadout widget:** `ShowRetoolerWidget` / `HideRetoolerWidget`, `SetRetoolerWidgetButtons`.
- **Options/graphics menu (a `Display*` family, FUN_140b964b0):** `DisplayMode`, `DisplayVSync`,
  `DisplayGamma`, `DisplayBrightness`, `DisplayWindowType`, `DisplayFullscreenWidth/Height`,
  `DisplayRefreshRateNumerator/Denominator`, `DisplayHDREnabled`, `DisplayHDRBrightness`,
  `DisplayHDRMaxBrightness`.

The **receiving ActionScript handlers for every one of these names are [walled: in .gfx asset]** — the exe
only holds the caller side.

### 6. ActionScript → game: input & callbacks (proven)

`FUN_141933e90` resolves and caches the five AS3 event classes from the movie's AS3 VM (via `FUN_14196dff0`,
a "find AS3 class by qualified name") into manager slots `+0x69..+0x6f`:

`scaleform.gfx.MouseEventEx`, `scaleform.gfx.KeyboardEventEx`, `scaleform.gfx.GamePadAnalogEvent`,
`scaleform.gfx.FocusEventEx`, `scaleform.gfx.TextEventEx` (also `scaleform.gfx.MouseCursorEvent` in
FUN_141a20060). Input is delivered by instantiating one of these and dispatching it into the movie — the
standard Scaleform GFx4 AS3 input model. `CUIInputManager` (vtable `PTR_LAB_141d90ca8`) is the C++ owner that
feeds the platform input into this path. (proven for the event-class bridge; the exact per-frame injection
loop is only partially reachable.)

The AS→native command channel also exists: the strings `"invoke"` / `"browserInvoke"` (FUN_141a2d480) are the
Scaleform external-interface/`fscommand`-style callback tags, i.e. AS can call back into native. Full handler
routing is only partially recoverable. (inferred.)

### 7. World-space UI (non-Scaleform, proven registration)

Health bars, outlines and interaction prompts are **entity components**, not movies:

- `CHealthbarComponent` — registered in the component registry `FUN_148f99a90` via
  `thunk_FUN_147cafcd0(registry,"CHealthbarComponent",vtable)`, adjacent to `CObjectiveParam_HealthBar`
  (an objective-driven health-bar parameter). (proven)
- `COutline` — reflected type via `FUN_140f27f60("COutline",8)` (FUN_1407db240), instantiated on the
  component path under `FUN_14085fd00`. The silhouette render itself is a GPU effect; only the component
  registration is in-scope here. (proven registration; render effect walled.)
- `CInspectableComponent` — reflected type `FUN_140f27f60("CInspectableComponent",0x15)` (FUN_14080d250,
  called from the component registrar `FUN_14085fd00`). Drives the "inspect/interact" world prompt. (proven)

These are placed on entities via RTPC components (memory `[[rtpc-entity-assembly]]`) and their screen
positions are projected each frame; the *label/bar visuals* that ride on top are still Scaleform
(the `CObjectiveParam_HealthBar` → HUD invoke path). (inferred for the projection→invoke linkage.)

---

## Data & config integration

- Every UI class is instantiated **by string name through the one global reflection factory** `FUN_140f27f60`
  (6,138 call sites project-wide; memory `[[operating-model]]`, README "cross-cutting architecture"). UI
  managers are installed by the **singleton-manager registrar `FUN_148f960c0`** (mirrored in `FUN_146cd0bea`);
  UI world-components by the **component registrar `FUN_14085fd00` / `FUN_148f99a90`**; UI-related conditions
  (`CConditional_IsMenuIconHovered`, `CConditional_IsHoveringOverMapIcon`, `CConditional_IsTutorialPhaseActive`)
  by the condition registrars. (proven — all four registrar tables contain UI rows.)
- Tutorial content is data-typed: reflected classes `CTutorialArrow`, `CTutorialBox`, `CTutorialImage`,
  `CTutorialObject`, `CTutorialPhase`, `CConditionalTutorialText` (FUN around 1411822–1412282) and a
  `"tutorials"` collection — configured in ADF/RTPC, driven by `CTutorialManager`. (proven the classes exist.)
- Objective→HUD glue: `HUDObjectiveUI` (FUN_14a646ed0 etc.), `CObjectiveParam_TornadoWidget`,
  `CObjectiveParam_HealthBar` — objective params that select which HUD widget to show. Ties this system to
  `missions_progression.md`. (proven strings.)
- The **actual layout/skin/animation of every widget lives in the `.gfx`/`.swf` Scaleform assets**
  (`ui/shared_lib.swf`, `ui/overlay.swf`, `ui/title.gfx`, plus per-screen movies), i.e. in the archive UI
  units (memory `[[composite-assets]]`: UI `.gfx` = Scaleform CFX). The tunable/visual side is entirely
  data-side. (proven that the movies are the data carrier; their contents are out of scope for a
  functions-only export.)

---

## Notable constants / identifiers (straight from the decomp)

| Constant | Meaning | Where |
|---|---|---|
| `0x41d8e380` | reflected hash of `CUIManager` | FUN_148f960c0 (line 4138593) |
| `0x41d8e998` | reflected hash of `CVideoManager` | FUN_148f960c0 (line 4138576) |
| `0x684f18a3` | movie id for `shared_lib` (`ui/shared_lib.swf`) | FUN_140e8e920 (line 1704000) |
| `0xf7eaa450` | movie id for `overlay` (`ui/overlay.swf`) | FUN_140e8e920 (line 1704045) |
| `0x30` (48) | `GFx::Value` element stride in every invoke arg array | FUN_140e60340, FUN_140dde200, … |
| vtable `+0x20` / `+0x38` | movie/widget **Invoke** virtual (type-dependent) | FUN_140e60340 / FUN_140dde200 |
| `DAT_142cb7dc8` | `CUIManager` singleton pointer | FUN_140e9fb80 (line 918806), FUN_140ea71d0 |
| `PTR_LAB_141d90a48 / …ba8 / …ca8 / …ce8 / …ac8` | vtables: Notification / ButtonHint / UIInput / Tutorial / VideoRecording managers | FUN_146cd0bea |
| profiler zones `UI` / `UI_Texture` / `UI_ActionScript` / `UI_Video` | render-pass names (enum 10–13) | FUN around line 3321564 |
| marker strings `CUIManager_RenderDynUI`, `CUIManager_RenderToTarget_Render`, `SCALEFORM_OFFSCREEN` | GPU debug scopes in the UI render | FUN_140ea71d0 |

---

## Call-graph highlights

- **Boot/load:** `FUN_14086d1e0` → `FUN_140e8e920` (loads `shared_lib`/`overlay`/`text/master`) →
  `FUN_140e827c0` (load movie) + `FUN_140ea3600` (register into `CUIManager` lists). `FUN_1408572c0` loads
  `ui\title.gfx` for the front-end.
- **Per frame — tick:** manager tick loop `FUN_148fe9ec0` → `FUN_140e9fb80(DAT_142cb7dc8, dt)` advances every
  movie.
- **Per frame — render:** `FUN_140ea71d0(CUIManager*, ctx)` walks the three movie lists → per-movie draw
  virtuals (`+0x20/+0x88/+0x90`), pushing `SCALEFORM_OFFSCREEN` / `CUIManager_RenderDynUI` /
  `CUIManager_RenderToTarget_Render` GPU markers; HUD composited by `FUN_140e64120` (`"CHUDUI"` double buffer).
- **Game → AS:** subsystem code (`FUN_140e60340` health, `FUN_140e614f0` compass, `FUN_140e08170` POI,
  `FUN_140dbad00` tutorial, `FUN_140dde200` hints/waypoint, `FUN_140b964b0` options) →
  build `GFx::Value[]` → movie `Invoke(name,…)`.
- **AS → game:** `FUN_141933e90` caches the `scaleform.gfx.*Event*` AS3 classes (via `FUN_14196dff0`) that
  `CUIInputManager` uses to inject input; `FUN_141a2d480` (`invoke`/`browserInvoke`) is the external-interface
  callback tag.

---

## Open questions / lower-confidence

- **Every widget's actual behavior is [walled: in .gfx asset].** The exe holds the caller side of the invoke
  bridge (method names + arg marshaling) but not the ActionScript that implements them. Decoding
  `ui/shared_lib.swf`, `ui/overlay.swf`, `ui/title.gfx` and the per-screen `.gfx` movies from the archives is
  the only way to recover the receiving logic (memory `[[composite-assets]]`, Scaleform CFX). (open)
- **Invoke vtable offset varies by wrapper type** (`+0x20` HUD, `+0x38` map). I did not enumerate every
  wrapper class / offset; a full map of "UI-object type → Invoke offset → owned method set" is a follow-up.
  (inferred; partial.)
- **`label → movie name-hash` equivalence** (e.g. `shared_lib` ↔ `0x684f18a3`) is assumed from the project's
  `lookup3` identity but not byte-verified in this pass. Verify with `jc4_arc hash "shared_lib"`. (inferred)
- **`CNotificationManager` queue mechanics** (dedupe, priority, timing) are not fully reconstructed — the
  registration + getter (`FUN_147fc30ec`) are confirmed but the enqueue/dequeue loop wasn't traced. (open)
- **`CGPSController` routing math** (path → minimap POI) not traced; only its reflected-type registration
  (`FUN_14080caf0`) and the map-waypoint invoke names are confirmed. (open)
- **`CUIInputManager` per-frame injection loop** — the event-class *cache* is proven (FUN_141933e90); the
  exact dispatch of a constructed event into the movie each frame is only partially reachable. (open)

---

## Appendix — decomp anchors (exact strings + FUN_/DAT_)

Reflected-type registrations (all via `FUN_140f27f60("<Name>", len)` or a registrar table):
`"CUIManagerGO"` FUN_140846830 · `"CUIManager"` FUN_148f960c0 (hash 0x41d8e380) · `"CUIController"`
FUN_140e85bf0/FUN_140815410 · `"CUIInputManager"` FUN_140e701c0 (mgr FUN_148f960c0) · `"CGPSController"`
FUN_14080caf0 · `"COutline"` FUN_1407db240 · `"CInspectableComponent"` FUN_14080d250 · `"CHealthbarComponent"`
FUN_148f99a90 · `"CNotificationManager"`/`"CButtonHintManager"`/`"CTutorialManager"`/`"CVideoRecordingManager"`
FUN_146cd0bea + FUN_148f960c0.

Scaleform / GFx (engine middleware, proven present): `Scaleform::Render::D3D1x::HAL::CreateRenderTarget`
(FUN_2881xxx), `Scaleform::Render::HAL::PopMask`, `Scaleform::Render::DrawableImage`,
`Scaleform::Render::ShaderHAL<…D3D1x…>::Push/PopBlendMode`, `GFx_DefineBitsJpeg{2,3,4}Loader`,
`GFx_ButtonLoader` (AS2/AS3), `gfxfontlib.swf`, `"\" - GFX file format expected"`, `"ActionScript version
mismatch"` — the full GFx SWF/GFX loader + D3D11 render HAL is compiled in.

Movies: `ui/shared_lib.swf` (0x684f18a3), `ui/overlay.swf` (0xf7eaa450), `ui\title.gfx`, `text/master`,
`avatar_fallback.avatar` — all in FUN_140e8e920 / FUN_1408572c0.

AS3 event classes (input bridge, FUN_141933e90 / FUN_141a20060): `scaleform.gfx.GamePadAnalogEvent`,
`scaleform.gfx.MouseEventEx`, `scaleform.gfx.KeyboardEventEx`, `scaleform.gfx.FocusEventEx`,
`scaleform.gfx.TextEventEx`, `scaleform.gfx.MouseCursorEvent`; external-interface tags `invoke` /
`browserInvoke` (FUN_141a2d480).

Game→AS invoke method names (proven string constants; caller FUN_ in §5): health/status —
`UpdateHealth`, `UpdateOxygen`, `DisplayHudState`, `Show/HideEmptyIndicator`; reticle — `UpdateWeaponReticle`,
`SetReticlePosition`, `UpdateGrappleReticle`, `ShowFixedAimReticles`; map/compass — `UpdateCompass`,
`Show/HideUserWaypoint`, `Push/Update/RemovePOI…` family; hints — `Show/Hide/UpdateButtonHints`; tutorials —
`Request/FreeTutorialBox`, `SetTutorialBox{Input,Size,Text}`, `SetTutorial{Images,Texts}`,
`Request/FreeTutorialArrow`, `UpdateSkipCutsceneHint`; retooler — `Show/HideRetoolerWidget`,
`SetRetoolerWidgetButtons`; options — `Display{Mode,VSync,Gamma,Brightness,WindowType,FullscreenWidth,
FullscreenHeight,RefreshRateNumerator,RefreshRateDenominator,HDREnabled,HDRBrightness,HDRMaxBrightness}`.

GPU/profiler markers (UI render, FUN_140ea71d0 + zone switch): `CUIManager_RenderDynUI`,
`CUIManager_RenderToTarget_Render`, `SCALEFORM_OFFSCREEN`, `CHUDUI`, and render-pass zones `UI`, `UI_Texture`,
`UI_ActionScript`, `UI_Video`.

Tutorial/objective data classes: `CTutorialArrow`, `CTutorialBox`, `CTutorialImage`, `CTutorialObject`,
`CTutorialPhase`, `CConditionalTutorialText`, `CBigButtonHint`, `CSmallButtonHint`, `HUDObjectiveUI`,
`CObjectiveParam_HealthBar`, `CObjectiveParam_TornadoWidget`; UI conditions `CConditional_IsMenuIconHovered`,
`CConditional_IsHoveringOverMapIcon`, `CConditional_IsTutorialPhaseActive`.

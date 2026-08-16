# The Demon — Los Demonios possession antagonist, wave/dome endgame threat

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

> Scope note: "The Demon" is the flying possession antagonist of the **Los Demonios / "Dangerous Demons"
> DLC** (`content_pack_demonios`, `dlc_demons_and_danger`). It sets up hostile **areas / domes**, spawns
> enemies in **waves**, and **possesses** characters — turning them into demon-thralls (a literal
> `demon_cow_skin` swap). This doc reconstructs the *mechanism* from the decomp. Per the project
> methodology caveat, this is a **functions-only** export: class **registration skeletons**, the call
> graph, and several reachable CPU-logic bodies are recoverable; most per-component **tick/evaluate
> bodies** and the **tunable magnitudes** live behind data-section factory vtables (`PTR_LAB_*`),
> ADF/RTPC config, or omitted ctor ranges and are tagged *open* with a resolve route. See
> `behavior_system.md` for the condition/action VM this rides on and `faction_frontline_chaos.md` for the
> dome↔chaos link.

## Overview

The Demon system is a **DLC content pack** (`content_pack_demonios`, registered via the standard name
factory `FUN_140f27f60("content_pack_demonios",0x15)` at several DLC-init sites, e.g. `FUN_143074fc0`
region). Its runtime is organized around one singleton **`CDemonAreaManager`** plus a family of reflected
gameplay **components** (`CDemonArea`, `CDemonTODArea`, `CDemonChaosCluster`, `CDemonPositioning`,
`CDemonStateObject`, `CDemonCoreStateObject`, `CDemonMovementParameter`, `CDemonWaveSpawner`,
`CDemoniosOutroState(Controller)`), a set of behavior **conditions** that gate the demon's flight/aim/
possession logic, two custom GPU **render blocks** (`CRenderBlockDemonDome`, `CRenderBlockDemonOrganic`),
and three ADF-reflected **state structs** that persist area/chaos progress (`SDemonAreaState`,
`SDemonChaosObjectState`, `EDemonicActiveState`). (proven — all names read as string constants; see Appendix.)

Gameplay loop, as reconstructed: a **DemonArea** (optionally a **TOD** = time-of-day-scoped area) is
established with a **dome** render effect; **wave spawners** emit enemies (from egg-cluster destructibles);
the demon acquires a **possession target** within a distance threshold, applies a pull impulse and the
`MOD_DEMON_ATTACHED` modifier, plays a `be_possessed_*` struggle animation, and on success fires
`on.demon.polymorphed` and swaps the victim's skin. Clearing the area advances an `EDemonicActiveState`
and eventually triggers the **Demonios outro** (`post_process_effect_demon_completion`). (inferred from the
string/registration set + the reachable bodies below; the orchestration ctor/tick is walled.)

## Key classes & functions

| Name string | Registered by (FUN_ / DAT_) | Role |
|---|---|---|
| `CDemonAreaManager` | manager list `FUN_146cd0bea` / `FUN_148f960c0` via `thunk_FUN_148f6e780(...,"CDemonAreaManager",...)`, vtable `PTR_LAB_141d90f48`; **ctor `FUN_14088fbc0`** | Singleton owner of all demon areas; installs debug commands, holds tunable defaults (proven) |
| `CDemonArea` | type-handle getter `FUN_140845870` → `DAT_142cb932c` | Reflected component: one hostile demon area (proven reg; body walled) |
| `CDemonTODArea` | `FUN_140845a20` → `DAT_142cb9df0` | Time-of-day-scoped demon area variant (proven reg; body walled) |
| `CDemonChaosCluster` | `FUN_14080a1c0` → `DAT_142cb9df8`, vtable `PTR_LAB_141d9ea18` | Chaos-object cluster inside an area (links to chaos/destruction) (proven reg) |
| `CDemonPositioning` | `FUN_14080a320` → `DAT_142cb9db4`, vtable `PTR_LAB_141d9e9c8` | Demon spatial positioning component (proven reg; body walled) |
| `CDemonStateObject` | `FUN_14080a3e0` → `DAT_142cb9de8`, vtable `PTR_LAB_141d9e978` | Generic demon state object (proven reg; body walled) |
| `CDemonCoreStateObject` | `FUN_140845900` → `DAT_142cb9e00` | The demon **core** state object (the central "boss" node) (proven reg) |
| `CDemonMovementParameter` | `FUN_140845990` → `DAT_142cb9e10` | Demon flight/movement tuning component (proven reg; values data-side) |
| `CDemonWaveSpawner` | `FUN_14080a4f0`/`FUN_14041a550` → `DAT_142cb23b4`, vtable `PTR_LAB_141d9b908` | Wave enemy spawner (proven reg; spawn logic walled) |
| `CDemoniosOutroState` | `FUN_140845ab0` → `DAT_142cb9e18` | Defeat/outro sequence state (proven reg; body walled) |
| `CDemoniosOutroStateController` | `FUN_140845b40` → `DAT_142cb9e20` | Controller driving the outro state (proven reg; body walled) |
| `CConditional_DemonDomePercentage` | `FUN_1408a1770` → `DAT_142cba020` (also registrar `FUN_148f99a90` via `thunk_FUN_147cafcd0`) | Behavior conditional: tests dome completion % (proven reg; evaluate walled) |
| `CRenderBlockDemonDome` | material-constants setup `FUN_14a385dc0` (label `"CRenderBlockDemonDome::SMaterialConstants"`) | GPU render block for the dome sphere (proven) |
| `CRenderBlockDemonOrganic` | `FUN_14a385dc0` (label `"CRenderBlockDemonOrganic::SMaterialConstants"`) | GPU render block for organic demon surfaces (proven) |

**Behavior conditions** (all via `thunk_FUN_14aadec10(<name>)` name→hash, registrar `FUN_1485b9730` /
`FUN_1485e07e0`; each getter listed):

| Condition | Getter FUN_ → DAT_ |
|---|---|
| `CDemonEngagedCharacterHasBlackboardValue` | `FUN_140547090` → `DAT_142cb45c4` |
| `CDemonFlyingBlendState` | `FUN_140547110` → `DAT_142cb407c` |
| `CDemonHasMoveInput` | `FUN_140547190` → `DAT_142cb45a4` |
| `CDemonHasPossessedCharacter` | `FUN_140547210` → `DAT_142cb45bc` |
| `CDemonHasPossessionTarget` | `FUN_140547290` → `DAT_142cb45b4` |
| `CDemonHumanAimBlendState` | `FUN_140547310` → `DAT_142cb4084` |
| `CDemonInputMoveType` | `FUN_140547390` → `DAT_142cb45ac` |
| `CDemonVelocityCondition` | `FUN_140547410` → `DAT_142cb4554` |
| `CDemonWithinPossessionDistanceCondition` | `FUN_140547490` → `DAT_142cb459c` |
| `CEngagedDemonWithinSegment` | `FUN_140547910` → `DAT_142cb454c` |

**Behavior actions / animation acts** (via `FUN_140f27f60`):

| Act | Registered at |
|---|---|
| `ACT_DEMON_HUMAN_POSSESSION_STRUGGLE_INTO` | `FUN_142f0c780` → `_DAT_142cb3310` |
| `ACT_DEMON_PASSIVE` | `FUN_142f0c810` → `_DAT_142cb38cc` |
| `ACT_POSSESSED` | wired in `FUN_14914b640` → handler `LAB_1408b01c0` |
| `be_possessed_long` / `be_possessed_short` | `FUN_1405c7480` → `DAT_142cb52f4` / `DAT_142cb52fc` |
| `demon_kill` (anim) | `FUN_14089d3b0` → `DAT_142cba070` |

**Status modifiers** (via `FUN_140f27f60`, applied in `FUN_1408c3190`):
`MOD_DEMON_ATTACHED` (`_DAT_142cba038`), `MOD_SUICIDE_CHARGE` (`_DAT_142cba030`).

## How it works (from the decomp)

### 1. Area manager & debug hooks — `CDemonAreaManager::ctor` `FUN_14088fbc0` (proven)

The manager constructs itself (`FUN_14088fbc0`, called from `FUN_14083bbb0`), registers into the global
manager list (`DAT_142cb1dd8` vector, lines 935054–935060), installs its vtable chain
(`PTR_LAB_141da6a10 … 141da6cd8`), and wires two **developer console commands**:

```
thunk_FUN_14762d8a0(param_1 + 0x14f, "demon.trigger_possession", 0xff, 1);   // 935118
thunk_FUN_14762d8a0(param_1 + 0x150, "demon.drop_possession",   0xff, 1);   // 935122
```

These prove the possession mechanic is manager-driven and independently triggerable/droppable. The ctor
also seeds a block of default field values (candidate tunables — meanings *inferred*, existence *proven*):
`+0xa8c/param_1[0x152]=0x3e4ccccd (0.2f)`, `+0xa94=5 (int)`, `param_1[0x153]=1.0f`, `+0xa9c=1.0f`,
`+0xae4=0x447a0000 (1000.0f)`, `+0xaec=0x42c80000 (100.0f)`, `param_1[0x15e]=0x42980000 (76.0f)`,
`0xdeadbeef` sentinels, and `DAT_142c73aa8/DAT_142c73ab0` (a shared default-vector pair). (proven these
literals are stored here; their gameplay semantics are not labeled in the decomp → *open*.)

### 2. Possession target acquisition & pull — struggle reaction `FUN_1405c7480` (proven)

`FUN_1405c7480` (callers=[], i.e. reached via an action/state vtable — inferred to back
`ACT_DEMON_HUMAN_POSSESSION_STRUGGLE_INTO`) is the clearest possession body:

- It fetches the engaged character (dynamic-type-checked against `&DAT_141d008e8` via the object's
  `vtable+0x10` "IsA" call) and computes the **planar distance** between demon and target using position
  fields `+0x144` (x) and `+0x14c` (z): `d² = dx² + dz²` (lines 552516–552523).
- If `d² <= DAT_141ca72c4` (a **squared distance threshold**, data-side) it uses the target's own facing;
  otherwise it **normalizes** the direction (`DAT_141ca6cac / sqrt(d²)`, `DAT_141ca6cac` ≈ unit magnitude)
  and applies a directional **pull impulse** through a named force channel:
  ```
  FUN_14036c4f0(uVar5, 0xfe357b8c, &local_38, DAT_141d1c474, 0xffffffff, 0);   // 552540
  ```
  `0xfe357b8c` is a **unique** force/event hash (single occurrence in the whole dump) → the "possession
  pull" channel. (proven)
- It then plays a struggle animation, **randomly** choosing long/short with a **1-in-3 short** bias using
  the classic MSVC LCG (`DAT_142c73a98 = DAT_142c73a98*0x343fd + 0x269ec3; r = (r>>16)&0x7fff`;
  `r%3==0 → be_possessed_short`), played via `FUN_140325d60(plVar7 + 0x41e, act, 0)` — `+0x41e` is the
  character's animation controller. (proven)
- Finally it adjusts a `short` health/segment field at `+0x3aa` (halved, propagated to a linked object via
  `FUN_14067eda0`). (proven mechanism; the field's exact meaning is inferred.)

### 3. Polymorph on success — `FUN_149149b20` (proven)

When the possession completes (a status bit `param_2+0x64 >> 0xf`), `FUN_149149b20`:

```
if bit set and not already-fired(param_1+0x8d1 & 8):
    thunk_FUN_147bfeb20("on.demon.polymorphed");         // 4167265  – broadcast event
    set bit 8
if bit8 set and not skinned(bit 0x10):
    uVar1 = FUN_140f27f60("demon_cow_skin", 0xe);         // 4167272
    thunk_FUN_148d4f690(param_1 + 0x10, uVar1);           // apply skin/material swap
    set bit 0x10
FUN_14089d3b0(param_1, param_2);                          // hand off to possessed-character update
```

So a fully-possessed human is **transformed** — the event `on.demon.polymorphed` is raised once, then the
character's model is skinned with `demon_cow_skin`. (proven — this is the literal "possess → thrall"
transform.)

### 4. Demon-core / possessed-character update — `FUN_1408c3190` (proven, partial)

`FUN_1408c3190` (size 4204, callers=[] → vtable-driven tick) iterates an engaged-entity container
(`param_1+0x308`) and stamps status modifiers on each member:

- If flag `param_1+0x8d0 & 1`: apply `MOD_SUICIDE_CHARGE` (`_DAT_142cba030`) to every engaged entity
  (loop 967556–967563).
- If the secondary list `param_1+0xa68` is populated: apply `MOD_DEMON_ATTACHED` (`_DAT_142cba038`) to
  every engaged entity (loop 967576–967584).
- Raises force/GPU channels `0xe40ca69d` and `0x4e29d94b` (each **unique** in the dump →
  demon-specific effect events) via `FUN_14036c5e0` when the demon becomes/stops being attached
  (967589, 967592).

This is the demon core's per-frame "who is attached to me" bookkeeping. (proven for the modifier/force
application; the surrounding movement integration is walled.)

### 5. `demon_kill` fallback — `FUN_14089d3b0` (proven)

`FUN_14089d3b0` (size 3005), reached from the polymorph handler, plays the `demon_kill` animation
(`FUN_140325d60(plVar18 + 0x41e, DAT_142cba070, 0)`, line 943892) on a struggle target whose segment
health `+0x3ac > 0` — i.e. the demon **executes** a victim that did not break free. (proven anim wiring;
full body is large and partly walled.)

### 6. Rendering — dome & organic (proven, GPU-side)

Two bespoke render blocks exist. `FUN_14a385dc0` populates their shader constant buffers (labelled
`CRenderBlockDemonDome::SMaterialConstants` / `CRenderBlockDemonOrganic::SMaterialConstants`). The shader
**pass** string inventory shows the dome is a multi-layer translucent sphere with reveal/dissolve passes,
and organic surfaces come in skinned/instanced/cloak/refraction variants:

- Dome passes: `DemonDomeInside`, `DemonDomeOutsideOpaque`, `DemonDomeOutsideTransparent`,
  `DemonDomeOutsideReveal`, `DemonDomeOutsideLandmark`, `DemonDomeDebug`.
- Organic passes: `DemonOrganic`, `_Skinned`, `_Instanced`, `_Cloak`, `_Refraction_*`, plus Z-only /
  Z-velocity depth variants.

`CConditional_DemonDomePercentage` is inferred to feed the dome's reveal/dissolve percentage (a completion
meter). The dome also **disables vehicles** — UI string `sfx_gui_dlc_demonios_vehicle_disabled_by_dome`.
(proven strings; the % → shader plumbing is inferred.)

## Data & config integration

- **Persisted state is ADF-reflected.** Three demon types register via `ArGetTypeId<…>` (Avalanche
  reflection) in `FUN_14087b510` / `FUN_14088bad0`:
  - `SDemonAreaState` — per-area state, stored as `std::unordered_map<uint64, SDemonAreaState>`
    (**keyed by a 64-bit hash id** → areas are addressed by name/entity hash, our cracked lookup3 identity).
  - `SDemonChaosObjectState` — `std::vector<SDemonChaosObjectState>` (per-area chaos-object progress).
  - `EDemonicActiveState` — an enum for an area's active/cleared phase.
  These are the save/serialization surface; **resolve values via `jc4_adf`** against demonios ADF blobs.
- **Components are RTPC/ADF-configured.** `CDemonMovementParameter`, `CDemonWaveSpawner`, `CDemonArea`,
  `CDemonPositioning` register as reflected components (the `FUN_14085fd00` component family) but carry no
  in-code magnitudes — flight speed, wave counts/cadence, area radius, possession range live in the
  entity `.epe` (RTPC v3) / ADF data. Map each class name → its entity-component hash per
  `[[rtpc-entity-assembly]]`; **resolve via the demonios `.epe` files**.
- **Assets:** 38 distinct `dlc/demonios/...` asset paths appear. Environment model families:
  `demoncore/` (the central core — `dlc2_demon_core_core_center*`, `..._eye_base`), and destructibles
  `clam/`, `egg_cluster/`, **`egg_demon_spawner/`** (ties directly to `CDemonWaveSpawner` — waves hatch
  from eggs), `medusae_tree/`, `mushroom/`, `pustule/`, `substrate/`. UI/audio: `map_panel_demonios_*`,
  `sfx_gui_dlc_demonios_*`, `post_process_effect_demon_completion`. (proven paths; see Appendix.)

## Notable constants / tunables (present in code)

| Value | Where (FUN_) | Meaning |
|---|---|---|
| `0xfe357b8c` | `FUN_1405c7480` @552540 | possession **pull** force/event channel (unique) (proven id; inferred role) |
| `0xe40ca69d`, `0x4e29d94b` | `FUN_1408c3190` @967589/967592 | demon-attach effect channels (unique) (proven id; inferred role) |
| `DAT_141ca72c4` | `FUN_1405c7480` @552524 | squared possession-distance gate (data-side) (proven ref) |
| `DAT_141ca6cac` | `FUN_1405c7480` @552533 | pull-direction unit magnitude (proven ref) |
| LCG `*0x343fd + 0x269ec3`, `%3` | `FUN_1405c7480` @552560 | 1-in-3 short vs long struggle-anim pick (proven) |
| `0.2f, 5, 1.0f, 1000.0f, 100.0f, 76.0f` | `CDemonAreaManager` ctor `FUN_14088fbc0` @935125-935132 | ctor default field seeds (proven literals; **semantics open**) |

> Everything else demon-related (flight speed, wave size/interval, dome radius, possession dwell time,
> segment counts) is **not** a code literal — it is component/ADF data. Do not invent it.

## Call-graph highlights

- DLC init → `content_pack_demonios` factory (`FUN_143074fc0` & siblings) registers the pack; manager
  registrar `FUN_148f960c0` / `FUN_146cd0bea` installs `CDemonAreaManager`.
- `CDemonAreaManager::ctor FUN_14088fbc0` ← `FUN_14083bbb0`; installs `demon.trigger_possession` /
  `demon.drop_possession`.
- Possession struggle `FUN_1405c7480` → `FUN_14036c4f0` (force `0xfe357b8c`) → `FUN_140325d60` (play
  `be_possessed_*`) → `FUN_14067eda0` (segment health).
- Polymorph `FUN_149149b20` (← `0x1408adc70`) → `thunk_FUN_147bfeb20("on.demon.polymorphed")` →
  `FUN_140f27f60("demon_cow_skin")` + skin swap → `FUN_14089d3b0` (`demon_kill`).
- Core tick `FUN_1408c3190` → applies `MOD_DEMON_ATTACHED` / `MOD_SUICIDE_CHARGE` over engaged entity list
  → `FUN_14036c5e0` (effect channels).
- `ACT_POSSESSED` wiring `FUN_14914b640` → registers handler `LAB_1408b01c0` (→ `FUN_14914bce0`).
- Render: `FUN_14a385dc0` (← `0x140d223b0`) fills Dome/Organic `SMaterialConstants`.

## Open questions / lower-confidence

- **Wave spawning cadence & counts** — `CDemonWaveSpawner` body is walled (factory vtable
  `PTR_LAB_141d9b908`); the `egg_demon_spawner` models confirm the *source* but not the numbers. *Resolve:*
  demonios `.epe`/ADF via `jc4_adf`, or x64dbg breakpoint on the spawner vtable tick. (open)
- **Demon flight/movement** — `CDemonMovementParameter`, `CDemonPositioning`, `CDemonInputMoveType`,
  `CDemonFlyingBlendState`, `CDemonVelocityCondition` register but carry no in-code magnitudes. *Resolve:*
  RTPC component data. (open)
- **State machine / segments** — `CDemonStateObject`, `CDemonCoreStateObject`, `EDemonicActiveState`,
  `CEngagedDemonWithinSegment` describe a segmented core-health state machine; only the registration
  skeleton and the `+0x3aa/+0x3ac` segment-health touches (in `FUN_1405c7480`/`FUN_14089d3b0`) are visible.
  Full transition graph is data/vtable-driven. (open)
- **Outro/defeat** — `CDemoniosOutroState(Controller)` + `post_process_effect_demon_completion` prove a
  scripted defeat sequence exists; the controller body is walled. *Resolve:* mission/RTPC + x64dbg. (open)
- **Dome %→shader** — `CConditional_DemonDomePercentage` evaluate and the `DemonDomeOutsideReveal`
  parameter feed are inferred, not read. (open)
- **`FUN_1405c7480` ↔ `ACT_DEMON_HUMAN_POSSESSION_STRUGGLE_INTO` binding** — the function is the struggle
  reaction by content, but the action→handler edge was not directly observed (callers=[]). (inferred)

## Appendix — decomp anchors

**Registration name→hash factory:** `FUN_140f27f60` (2nd arg = string length). **Condition name factory:**
`thunk_FUN_14aadec10`. **Component registrar:** `FUN_14085fd00`. **Manager registrar:** `FUN_148f960c0` /
`FUN_146cd0bea` (`thunk_FUN_148f6e780`). **Condition registrars:** `FUN_1485b9730`, `FUN_1485e07e0`.

**Component type-handle getters:** `CDemonArea` `FUN_140845870`; `CDemonTODArea` `FUN_140845a20`;
`CDemonCoreStateObject` `FUN_140845900`; `CDemonMovementParameter` `FUN_140845990`; `CDemoniosOutroState`
`FUN_140845ab0`; `CDemoniosOutroStateController` `FUN_140845b40`; `CDemonChaosCluster` `FUN_14080a1c0`
(vtable `PTR_LAB_141d9ea18`); `CDemonPositioning` `FUN_14080a320` (`PTR_LAB_141d9e9c8`); `CDemonStateObject`
`FUN_14080a3e0` (`PTR_LAB_141d9e978`); `CDemonWaveSpawner` `FUN_14080a4f0`/`FUN_14041a550`
(`PTR_LAB_141d9b908`).

**Manager:** `CDemonAreaManager` reg `FUN_146cd0bea`@0x3209745 & `FUN_148f960c0`@0x4139227
(vtable `PTR_LAB_141d90f48`); **ctor** `FUN_14088fbc0`.

**Conditions:** `FUN_140547090/110/190/210/290/310/390/410/490/910` (see table above).

**Actions/acts:** `ACT_DEMON_HUMAN_POSSESSION_STRUGGLE_INTO` `FUN_142f0c780`; `ACT_DEMON_PASSIVE`
`FUN_142f0c810`; `ACT_POSSESSED` `FUN_14914b640`→`LAB_1408b01c0`; `be_possessed_long/short` `FUN_1405c7480`;
`demon_kill` `FUN_14089d3b0`.

**Modifiers:** `MOD_DEMON_ATTACHED`/`MOD_SUICIDE_CHARGE` in `FUN_1408c3190`.

**Logic bodies:** possession struggle `FUN_1405c7480`; polymorph `FUN_149149b20`; core tick
`FUN_1408c3190`; kill `FUN_14089d3b0`.

**ADF state types:** `SDemonAreaState`, `std::unordered_map<uint64,SDemonAreaState>`,
`SDemonChaosObjectState`, `std::vector<SDemonChaosObjectState>`, `EDemonicActiveState` — reg
`FUN_14087b510` (@0x924455), `FUN_14088bad0` (@0x932390).

**Render:** `CRenderBlockDemonDome`/`CRenderBlockDemonOrganic` `SMaterialConstants` in `FUN_14a385dc0`;
dome passes `DemonDome{Inside,OutsideOpaque,OutsideTransparent,OutsideReveal,OutsideLandmark,Debug}`;
organic passes `DemonOrganic[_Skinned|_Instanced|_Cloak|_Refraction_*|ZOnly*|ZVelocity*]`.

**Events/commands/UI:** `demon.trigger_possession`, `demon.drop_possession` (`FUN_14088fbc0`);
`on.demon.polymorphed` (`FUN_149149b20`, `FUN_140a9abb0`); `demon_cow_skin` (`FUN_149149b20`);
`post_process_effect_demon_completion` (`FUN_143074fc0` region @0x3112911);
`sfx_gui_dlc_demonios_vehicle_disabled_by_dome` (`FUN_1408ae570`); `content_pack_demonios`,
`dlc_demons_and_danger`, `LOS DEMONIOS`, `LOS DEMONIOS TEAM`.

**Assets (families):** `dlc/demonios/models/environments/{demoncore, substrate, destructibles/{clam,
egg_cluster, egg_demon_spawner, medusae_tree, mushroom, pustule}}`; `dlc/demonios/textures/ui/zoom%d/%d.ddsc`.

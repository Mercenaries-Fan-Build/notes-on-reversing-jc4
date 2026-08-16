# Weapon-Gadget Component Catalog — JC4's modular weapon-behavior toybox

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative). "proven" =
read directly from the decomp; addresses cite `output/_ghidra_jc4/jc4_all_functions_decomp.txt`.

> Builds on [weapons.md](weapons.md) (the core framework, equip model, firing/recoil math). **That doc owns
> the firing pipeline; this one owns the component catalog** — the interchangeable behavior modules that make
> each weapon/gadget unique. No duplication: firing math (`FUN_140730540`), damage routing, and the
> aim/reload action vocabulary live in weapons.md; here we enumerate the `C*WeaponComponent` family, how a
> weapon composes from them, and the `CRetoolerManager` weapon-modification/upgrade layer.

## Overview

A JC4 weapon is an **RTPC entity** whose behavior is assembled from a bag of reflected **components**, exactly
like every other Apex gameplay object in this repo (`[[rtpc-entity-assembly]]`, `[[composite-assets]]`). The
core stack (`CWeaponConfig`, `CWeaponSetter/Remover`, `CBulletSpawner`, `CDamageController*`) is in weapons.md;
on top of it the data can bolt on any of **26 optional `C…WeaponComponent` modules** — a turret mount, a
tether/magnet, a laser or lightning beam, a scope, a spotlight, a barrel-spin cosmetic, a camera swap, a
physics-ragdoll caster, a vehicle-part remover, and so on. Each module is registered by name into the same
reflection type registry the rest of the engine uses, then instantiated per-weapon from the entity's ADF
component list and walked by per-type manager update passes. Weapon **modification/progression** (unlocking
and swapping these behaviors as loadout options) is the separate `CRetooler*` family — the in-game "retooler"
upgrade screen — gated by a set of `CConditional_IsRetooler*` behavior conditions. (proven: registration
strings + two reachable driver functions below; per-component tick bodies are **walled**, see Open questions.)

## How a weapon composes from components (the registration spine)

Every component class registers itself through a **TLS-guarded static "reflected type-id" accessor** — a
~130-byte function that on first call does `DAT_<typeid> = FUN_140f27f60("<ClassName>", <len>, 0, …)` and
thereafter returns `&DAT_<typeid>`. `FUN_140f27f60` is the **same RTPC type registrar** used by `CWeaponConfig`
/`CWeaponSetter`/etc. in weapons.md (`FUN_140f27f60(name,len,…)` → `FUN_14aadeee0`, keyed by the cracked
lookup3 name hash). The 26 component accessors sit in **two contiguous, alphabetically-ordered reflection
blocks**:

- `FUN_1406b6e10 … FUN_1406b7290` — `CAimPOI…` through `CCylinder…` (9 modules), interleaved with unrelated
  reflected classes (e.g. `CDialogueChain` at `FUN_1406b7320`). (proven)
- `FUN_140743380 … FUN_140744cd0` — `CDoAct…` through `CWallPenetration…` (17 modules). (proven)

The `len` argument is the exact class-name length in hex — a built-in sanity oracle that the string is the
real class name (e.g. `"CTurretWeaponComponent",0x16` = 22, `"CVehiclePartRemoverWeaponComponent",0x22` = 34).
(proven)

**Composition model (inferred, consistent with weapons.md + `[[rtpc-entity-assembly]]`):** the weapon entity's
ADF declares which of these components it carries; at load the reflection factory (keyed on the `DAT_<typeid>`
from each accessor) instantiates them and attaches them to the entity's component list. Per-type **manager
update passes then walk the entity's component array** and drive each module — directly visible in the two
recoverable drivers, which both iterate a component list on their owner and act per element:
`FUN_148be9610` walks `*(param+0x200)…*(param+0x208)` (the Light/Spotlight set) and `FUN_148a8f530` walks
`*(param+0x1f0)…*(param+0x1f8)` (the ContentSwapper set). (proven that the walk exists; that the list is the
weapon's attached-component array is inferred.)

**Walled by construction:** each `DAT_<typeid>` is referenced *only* by its own accessor's `return &DAT_…`
(verified for the Turret/Magnet ids) — the factory and per-frame tick that consume the type-id live behind
vtable/reflection dispatch that the functions-only export did not capture. So for 24 of 26 modules the
recoverable handle is **the registrar + type-id + class name**; behavior is named-but-walled. (proven that the
ids have no other textual consumer; this is the methodology caveat in action.)

## Component catalog

Grouped by theme. Every row is **proven** (the class string + its accessor `FUN_` + type-desc `DAT_` are read
directly from the registration block). The **Role** column is **inferred from the class name** unless a
reachable-logic note follows. `Len` = the hex name-length arg passed to `FUN_140f27f60`.

### Targeting / aiming
| Class string | Accessor `FUN_` | Type-desc `DAT_` | Len | Role (inferred) |
|---|---|---|---|---|
| `CTurretWeaponComponent` | `FUN_140744bb0` | `142cb83d4` | 0x16 | Turret mount — yaw/pitch traverse + fire (cf. "MountedWeapon" `FUN_1407b0e40`, weapons.md) |
| `CLockonWeaponComponent` | `FUN_140744730` | `142cb8358` | 0x16 | Target lock-on (homing/painting); pairs with `CConditional_IsTargetingObject/Shape` |
| `CAimPOIWeaponComponent` | `FUN_1406b6e10` | `142cb8264` | 0x16 | Aim point-of-interest bias (auto-aim / soft-lock POI) |
| `CScopeWeaponComponent` | `FUN_140744a00` | `142cb83bc` | 0x15 | Scope/ADS optic (cf. `CWeaponHasPrecisionScopeCondition`, weapons.md) |
| `CFOWControllerWeaponComponent` | `FUN_140743410` | `142cb8368` | 0x1d | Fog-of-war / reveal controller driven by the weapon |
| `CMountedComponent` | `FUN_1407448e0` | `142cb8360` | 0x11 | Mounted-weapon attach point (note: `CMountedComponent`, not `…WeaponComponent`) |

### Projectile / beam / firing
| Class string | Accessor `FUN_` | Type-desc `DAT_` | Len | Role (inferred) |
|---|---|---|---|---|
| `CLaserBeamWeaponComponent` | `FUN_140744580` | `142cb8370` | 0x19 | Continuous laser beam (raycast + beam FX) |
| `CLightningBeamWeaponComponent` | `FUN_1407446a0` | `142cb839c` | 0x1d | Chaining lightning beam (cf. `CLightningManager`, wind_and_weather.md) |
| `CWallPenetrationWeaponComponent` | `FUN_140744cd0` | `142cb83fc` | 0x1f | Ray-cast wall-penetration firing — **has reachable firing logic** `FUN_14077cac0` (see weapons.md; dev-error string `"…WallPenetrationWeapomComponent without a valid start ray position"`) |
| `CAmmoRegenerationWeaponComponent` | `FUN_1406b6f30` | `142cb8274` | 0x20 | Regenerating-ammo weapons (no reloads); also listed in weapons.md core table |
| `CCylinderWeaponComponent` | `FUN_1406b7290` | `142cb82a4` | 0x18 | Cylinder/revolver-style per-chamber ammo model (inferred) |

### Physics manipulation (the "toy" gadgets)
| Class string | Accessor `FUN_` | Type-desc `DAT_` | Len | Role (inferred) |
|---|---|---|---|---|
| `CMagnetWeaponComponent` | `FUN_1407447c0` | `142cb83a4` | 0x16 | Magnet/tether gun — attracts/pins objects; ties to the tether force channel (`tether.force.detach`) and `CConditional_IsTetheredToObject`/`CConditional_TetherState*` |
| `CPhysicalizationWeaponComponent` | `FUN_140744970` | `142cb83b4` | 0x1f | Converts a hit target to a physicalized/ragdoll body (dynamic Havok) |
| `CVehiclePartRemoverWeaponComponent` | `FUN_140744c40` | `142cb83f4` | 0x22 | Shears parts off vehicles on hit (tie to destruction.md) |
| `CHitReactionCastWeaponComponent` | `FUN_1407444f0` | `142cb8388` | 0x1f | Casts a hit-reaction/impulse onto struck characters (stagger/knockback) |
| `CAirplaneFlybyWeaponComponent` | `FUN_1406b6ea0` | `142cb826c` | 0x1d | Calls in an airplane strafing flyby (support-gadget style) |

### Camera
| Class string | Accessor `FUN_` | Type-desc `DAT_` | Len | Role (inferred) |
|---|---|---|---|---|
| `CCameraSwapWeaponComponent` | `FUN_1406b7170` | `142cb8294` | 0x1a | Swaps the active camera when the weapon is used (e.g. missile-cam) |
| `CCameraFollowWeaponComponent` | `FUN_1406b70e0` | `142cb828c` | 0x1c | Camera follows the projectile/target while firing |

### Cosmetic / attachment / audio
| Class string | Accessor `FUN_` | Type-desc `DAT_` | Len | Role (inferred) |
|---|---|---|---|---|
| `CBarrelSpinWeaponComponent` | `FUN_1406b7050` | `142cb8284` | 0x1a | Spins a barrel (minigun spin-up cosmetic/gate) |
| `CBarrelRecoilWeaponComponent` | `FUN_1406b6fc0` | `142cb827c` | 0x1c | Barrel/slide recoil animation offset (cosmetic; cf. "WeaponRecoil" `FUN_147759a20`) |
| `CModelAttachementWeaponComponent` | `FUN_140744850` | `142cb83ac` | 0x20 | Attaches an extra model to the weapon (sic: "Attachement") |
| `CLightWeaponComponent` | `FUN_140744610` | `142cb8378` | 0x15 | Point/omni scene light on the weapon — **reachable render logic** (see below) |
| `CSpotlightWeaponComponent` | `FUN_140744a90` | `142cb83c4` | 0x19 | Spot (cone) scene light — same render driver as Light (see below) |
| `CContentSwapperWeaponComponent` | `FUN_1406b7200` | `142cb829c` | 0x1e | Swaps model content/parts by visibility flags — **reachable driver** (see below) |
| `CDoActWeaponComponent` | `FUN_140743380` | `142cb8350` | 0x15 | Fires an `ACT_*` behavior action when used (bridges into behavior VM, behavior_system.md) |
| `CTriggerBarkWeaponComponent` | `FUN_140744b20` | `142cb83cc` | 0x1b | Triggers a VO "bark"/dialogue line on fire (cf. `CVocalsManager`, audio_dialogue.md) |

(All 26 rows proven from the two registration blocks. Roles inferred from the class name except where a
reachable-logic note is given.)

## Reachable component logic (the two that decompiled)

### `CLightWeaponComponent` / `CSpotlightWeaponComponent` — real scene-light attach (`FUN_148be9610`, proven)
A 902-byte update pass (`FUN_148be9610` @L4055655) walks the weapon's light-component list
(`*(param_1+0x200)…+0x208`, stride 2) and drives the **engine light manager `_DAT_142cada38`** using the debug
tag string `"lightweaponcomponent"`:
- **Destroy** (when the enable byte `param_2=='\0'`): `thunk_FUN_14a04a9b0(_DAT_142cada38, handle, "lightweaponcomponent")` releases the light and nulls the slot. (proven)
- **Create**: if the light flag byte at `+0x45` is 0 → `thunk_FUN_14a03c7d0(mgr, id, "lightweaponcomponent")` (point/omni light); else → `thunk_FUN_14a041ee0(mgr, id, "lightweaponcomponent",0,0)` (spot light). The id is `(int)*(float*)(desc+0x30) & 0xffff`. (proven — the point-vs-spot branch is exactly what separates `CLightWeaponComponent` from `CSpotlightWeaponComponent`.)
- **Parameters**: reads a radius/intensity float from `desc+0x38`, clamps against a floor at `+0x218`, and derives a cone term via `FUN_141afc000((fVar22+fVar22)/ *(desc+0x48))` (a trig call — half-angle → cone), using module constants `DAT_141ca6ca0/…cac/…cb4`. (proven arithmetic; field names inferred.)

The same `"lightweaponcomponent"` release path also appears in weapon-teardown functions `FUN_140770960` and
`FUN_140722755`/`FUN_140752426`/`FUN_140781668`, confirming these components own live render-light handles that
must be released on unequip. (proven: string refs.)

### `CContentSwapperWeaponComponent` — model-part visibility swap (`FUN_148a8f530`, proven)
`FUN_148a8f530` (@L4015151, `callers=[0x1406c1170]`) walks the swapper list (`*(param_1+0x1f0)…+0x1f8`) and for
each element resolves **five boolean visibility states** from hashed ids via
`thunk_FUN_148ba6d80(resolver, *(elem+0x38/0x3c/0x40/0x44/0x48))`, storing the results at `elem+0x30…0x34`,
then binds the element to the model system with `thunk_FUN_149748ab0(elem+0x18, "ContentSwapperWeaponComponent")`.
So the component maps up to five named model-content ids to on/off visibility — the mechanism behind weapons
whose model changes (e.g. retooler skins / mod visuals). (proven; the "five states" and the resolver identity
are read directly.)

## Weapon modification / upgrade — the Retooler (progression tie-in)

The **retooler** is JC4's in-game weapon/gadget modification screen (each weapon has upgrade "mods" and
loadout options unlocked through progression). It is a manager + object family plus a set of behavior
conditions the UI/behavior graph query.

### Managers (registered in the singleton manager registry `FUN_146cd0bea` @L3209030, via `thunk_FUN_148f6e780`)
| Manager | vtable (`PTR_LAB_`) | Role (inferred) |
|---|---|---|
| `CRetoolerManager` | `141d907a8` | Weapon-mod / retool progression + loadout system (proven: `@L3209445`) |
| `CAmmunitionManager` | `141d90928` | Ammo pools / regeneration accounting (proven: `@L3209505`) |
| `CWeaponManager` | `141d90c68` | Weapon registry / equip authority (proven: `@L3209635`; also in weapons.md) |
| `CWeaponPlacementRulePersistentManager` | `141d90948` | World weapon-pickup placement (proven: `@L3209510`) |

`CRetoolerManager` registers **adjacent to the progression managers** (`CRewardSystem`, `CNewGamePlusManager`,
`CFrontlineManager`, `CMissionManager`, `CQuestManager` are its neighbors in the same table) — structural
confirmation it is a progression system, not a runtime combat one. (proven: table ordering.)

### Retooler objects & settings (reflected classes)
`CRetoolerObject`, `CRetoolerSettings`, `CRetoolerLoadoutObject`, `CRetoolerLoadoutEditable`,
`CRetoolerLoadoutVisibility` — the per-weapon retool state, the settings block, and the editable loadout model
(the UI lets you build/edit loadouts, hence "Editable"/"Visibility"). UI/audio layer: `RetoolerScreenLayerManager`
and a large `sfx_gui_retooler_*` string set (navigate, unlock module, swap loadout, upgrade screen, cannot_unlock),
plus localization keys `retooler_mods_completemsg`, `retooler.group.unlocked`, `retooler_powerlevel_{off,low,medium}`,
and achievement `ach_retooler_all_unlocked`. (proven: string literals.)

### Retooler / tether behavior conditions (the query surface, registered via `FUN_140f27f60`)
| Condition | Accessor `FUN_` | Type-desc `DAT_` | Gates (inferred) |
|---|---|---|---|
| `CConditional_IsRetoolerFeatureUnlocked` | `FUN_14065f400` | `142cb5ee0` | A retooler feature/mod is unlocked |
| `CConditional_IsRetoolerGroupSelected` | `FUN_14065f4a0` | `142cb5ee8` | A retooler group is selected in UI |
| `CConditional_IsRetoolerLoadoutSelected` | `FUN_14065f540` | `142cb5ef8` | A loadout is selected |
| `CConditional_IsRetoolerOptionEquipped` | `FUN_14065f5e0` | `142cb5ef0` | A specific mod option is equipped |
| `CConditional_IsTetheredToObject` | (reg `@L627753`) | `142cb5f28` | Object is tethered (magnet/grapple) |
| `CConditional_TetherState` / `…StateBase` / `…_Simple` | (reg `@L628253/628278/628303`) | `142cb5fc0/…fb8/…fc8` | Tether state machine query |
| `CConditional_TetherConnection` | (reg `@L628228`) | `142cb5fb0` | Tether connection present |
| `CConditional_DeployedTetherCount` | `FUN_14063d3c0` | `142cb5a20` | Count of deployed tethers |

These register in the **behavior-condition registry** alongside every other `CConditional_*` (behavior_system.md);
each is a size-145 name-registration stub. The `CMagnetWeaponComponent` gadget produces the tether state these
conditions read — the magnet-gun ↔ grappling-tether shared surface (`grappling_hook.md`, `CTetherTensionCondition`,
force channel `tether.force.detach`). (proven: registrations; the component→condition data flow is inferred.)

## Notable constants / tunables

- Light/Spotlight render driver `FUN_148be9610`: light-id mask `& 0xffff` on `*(desc+0x30)`; cone-angle trig
  via `FUN_141afc000`; module float constants `DAT_141ca6ca0`, `DAT_141ca6cac`, `DAT_141ca6cb4`; intensity
  floor at `desc+0x218`. (proven arithmetic; field names inferred.)
- ContentSwapper `FUN_148a8f530`: **exactly 5** visibility ids at `elem+0x38…+0x48` → 5 bools at `elem+0x30…+0x34`. (proven.)
- Every component name-length constant (the `Len` column) is a proven exact-match oracle for the class name.
- **Per-component magnitudes** (turret traverse speed, beam range/DPS, magnet force, physicalization impulse,
  light radius/intensity values, flyby speed) are **data-side** — they live in the `CWeaponConfig`/ADF/RTPC
  component records, not in the executable (consistent with the repo-wide "mechanism in code, magnitudes in
  data" finding). (inferred.)

## Call-graph highlights

- Component type registry: 26 accessors `FUN_1406b6e10…1406b7290` + `FUN_140743380…140744cd0`, each → `FUN_140f27f60(name,len,…)` → `FUN_14aadeee0` (the lookup3-keyed RTPC type registrar). (proven)
- Light/Spotlight update `FUN_148be9610` → light manager `_DAT_142cada38` create/add/release thunks (`FUN_14a03c7d0`, `FUN_14a041ee0`, `FUN_14a04a9b0`), tag `"lightweaponcomponent"`; teardown mirror in `FUN_140770960`. (proven)
- ContentSwapper update `FUN_148a8f530` (caller `FUN_1406c1170`) → visibility resolver `FUN_148ba6d80` + model bind `FUN_149748ab0`. (proven)
- Manager registry `FUN_146cd0bea` → `CRetoolerManager`/`CAmmunitionManager`/`CWeaponManager`/`CWeaponPlacementRulePersistentManager` (vtables `141d907a8`/`…90928`/`…90c68`/`…90948`). (proven)
- Retooler/Tether conditions register in the behavior-condition registry with the rest of `CConditional_*` (behavior_system.md). (proven)

## Open questions / lower-confidence

- **24 of 26 component tick bodies are walled.** Only `CLight`/`CSpotlight` (`FUN_148be9610`) and
  `CContentSwapper` (`FUN_148a8f530`) decompiled to reachable logic; the rest are recoverable only as
  registrar + type-id + class name (their `DAT_<typeid>` has no other textual consumer — the factory/update is
  behind vtable/reflection dispatch the functions-only export dropped). **Resolve route:** the ADF/RTPC
  component records that instantiate them (`[[rtpc-entity-assembly]]`, `[[composite-assets]]`) will name the
  properties each component reads — decode a weapon entity that carries, e.g., `CMagnetWeaponComponent` and
  read its component payload; and/or set a read-only breakpoint on `FUN_140f27f60`'s returned type-id in
  x64dbg to catch the factory that consumes it (per the debugger mandate: paused inspection only).
- **Component roles are name-inferred** except the two above and `CWallPenetration…` (whose firing logic
  `FUN_14077cac0` is documented in weapons.md). Turret/Magnet/Beam/Physicalization behavior and magnitudes are
  not yet read.
- **`CMountedComponent`** (0x11) is named `CMountedComponent`, not `…WeaponComponent`, and is distinct from the
  "MountedWeapon" logic `FUN_1407b0e40` in weapons.md — relationship (attach-point vs. weapon logic) not yet
  confirmed. **No `CCannonWeaponComponent`** string exists in the dump (the prompt's "Cannon(?)" is unfounded);
  cannon behavior is presumably a `CWeaponConfig` archetype, not its own component. (proven negative.)
- **Retooler mechanics** (unlock gating, mod → component mapping, loadout persistence) are only seen as
  manager/object/condition registrations + UI strings; the actual unlock/apply logic in `CRetoolerManager`'s
  vtable `141d907a8` is not yet decoded. Tie each `CConditional_IsRetooler*` to its progression data source
  next.

## Appendix — decomp anchors

```
Component registrar (all → FUN_140f27f60(name,len,0,…) → FUN_14aadeee0):
  Block A FUN_1406b6e10..1406b7290 :  CAimPOI 1406b6e10/DAT_142cb8264 · CAirplaneFlyby 1406b6ea0/826c ·
    CAmmoRegeneration 1406b6f30/8274 · CBarrelRecoil 1406b6fc0/827c · CBarrelSpin 1406b7050/8284 ·
    CCameraFollow 1406b70e0/828c · CCameraSwap 1406b7170/8294 · CContentSwapper 1406b7200/829c ·
    CCylinder 1406b7290/82a4
  Block B FUN_140743380..140744cd0 :  CDoAct 140743380/DAT_142cb8350 · CFOWController 140743410/8368 ·
    CHitReactionCast 1407444f0/8388 · CLaserBeam 140744580/8370 · CLight 140744610/8378 ·
    CLightningBeam 1407446a0/839c · CLockon 140744730/8358 · CMagnet 1407447c0/83a4 ·
    CModelAttachement 140744850/83ac · CMountedComponent 1407448e0/8360 · CPhysicalization 140744970/83b4 ·
    CScope 140744a00/83bc · CSpotlight 140744a90/83c4 · CTriggerBark 140744b20/83cc · CTurret 140744bb0/83d4 ·
    CVehiclePartRemover 140744c40/83f4 · CWallPenetration 140744cd0/83fc
Reachable logic:
  Light/Spotlight scene-light driver   FUN_148be9610 (tag "lightweaponcomponent", mgr _DAT_142cada38;
    create FUN_14a03c7d0 / spot FUN_14a041ee0 / release FUN_14a04a9b0); teardown FUN_140770960
  ContentSwapper visibility driver     FUN_148a8f530 (caller FUN_1406c1170; resolver FUN_148ba6d80,
    bind FUN_149748ab0, 5 ids elem+0x38..0x48 → 5 bools elem+0x30..0x34)
  WallPenetration fire (see weapons.md) FUN_14077cac0
Managers (registry FUN_146cd0bea @L3209030, thunk_FUN_148f6e780):
  CRetoolerManager PTR_LAB_141d907a8 @L3209445 · CAmmunitionManager 141d90928 @L3209505 ·
  CWeaponManager 141d90c68 @L3209635 · CWeaponPlacementRulePersistentManager 141d90948 @L3209510
Retooler objects/UI: CRetoolerObject · CRetoolerSettings · CRetoolerLoadoutObject ·
  CRetoolerLoadoutEditable · CRetoolerLoadoutVisibility · RetoolerScreenLayerManager · sfx_gui_retooler_* ·
  retooler.group.unlocked · retooler_powerlevel_{off,low,medium} · ach_retooler_all_unlocked
Retooler/Tether conditions (behavior-condition registry):
  CConditional_IsRetoolerFeatureUnlocked 14065f400/DAT_142cb5ee0 ·
  CConditional_IsRetoolerGroupSelected 14065f4a0/5ee8 · CConditional_IsRetoolerLoadoutSelected 14065f540/5ef8 ·
  CConditional_IsRetoolerOptionEquipped 14065f5e0/5ef0 · CConditional_IsTetheredToObject @L627753/5f28 ·
  CConditional_TetherConnection @L628228/5fb0 · CConditional_TetherState @L628253/5fc0 ·
  CConditional_TetherStateBase @L628278/5fb8 · CConditional_TetherState_Simple @L628303/5fc8 ·
  CConditional_DeployedTetherCount 14063d3c0/5a20
Force channel string: "tether.force.detach"
```

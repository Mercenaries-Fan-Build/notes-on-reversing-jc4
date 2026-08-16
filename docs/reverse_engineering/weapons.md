# Weapons & Combat — how JC4 arms, aims, fires, and damages

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative). "proven" = read
directly from the decomp; addresses cite `output/_ghidra_jc4/jc4_all_functions_decomp.txt`.

## Overview

Just Cause 4's weapon system is a data-driven **RTPC entity-component** stack sitting on the Apex behavior
VM, exactly like the other gameplay systems in this repo. A wielded weapon is not a monolithic object; it is
a set of engine **components** attached to an entity — `CWeaponConfig` (tuning), `CWeaponSetter`/
`CWeaponRemover` (equip/unequip drivers), `CWeaponUIInfo` (HUD data), `CBulletSpawner`, `CDamageController*`
(hit analyze + apply), `CAmmoRegenerationWeaponComponent`, and specialised firing components such as the wall-
penetration ray-caster (proven: registrations in `FUN_14085fd00`'s table; component factories below). Player
intent flows through the **behavior VM**: a large family of `CWeapon*Condition` / aiming conditions gate
animation-graph transitions, and `ACT_*` / `UB_ACT_*` actions drive draw/aim/fire/reload/holster. The actual
per-shot mechanism — spread, recoil accumulation, muzzle effect — lives in `FUN_140730540` ("WeaponFire"),
which uses an MSVC linear-congruential PRNG for bullet spread and accumulates recoil onto the aim yaw/pitch.
Source-path strings confirm the C++ layout: `…\game\equipment\item\weapon\weapon.cpp` and
`…\game\character\statetasks\inputweapontask.cpp` (proven: string literals in the dump).

## Key classes & functions

### Component factories (registered in the RTPC component table `FUN_14085fd00`)
Each factory allocates a 0x10-byte component object with a class vtable and registers the class name+length
via `FUN_140f27f60(name, len, …)` (the RTPC type registrar → `FUN_14aadeee0`). (proven)

| Class string | Factory `FUN_` | vtable (`PTR_LAB_`) | Type-desc `DAT_` | Role (inferred from name) |
|---|---|---|---|---|
| `CWeaponConfig` | `FUN_140815f80` | `141d9ce20` | `142cb985c` | Weapon tuning/data block |
| `CWeaponSetter` | `FUN_1408161c0` | `141d9cdd0` | `142cb984c` | Equips/sets the wielded weapon |
| `CWeaponRemover` | `FUN_140816100` | `141d9cdf8` | `142cb9854` | Unequips/removes weapon |
| `CWeaponUIInfo` | `FUN_140846b90` | — | `142cb988c` | HUD/reticle/ammo UI data |
| `CWeaponPlacementRule` | `FUN_140816040` | `141d9bbb0` | `142cb9448` | Pickup/spawn placement rule |
| `CDamageController` | `FUN_140809c00` | `141d9dac8` | `142cb91c0` | Damage routing per entity |
| `CDamageControllerHitAnalyze` | `FUN_140809cc0` | — | — | Resolves what a hit struck |
| `CDamageControllerHitApply` | `FUN_140809d80` | — | — | Applies resolved damage |
| `CBulletSpawner` | `FUN_1407432f0` | — | `142cb83e4` | Spawns projectile/bullet instances |
| `CAmmoRegenerationWeaponComponent` | `FUN_1406b6f30` | — | `142cb8274` | Regenerating-ammo weapons |

Note the string length passed to the registrar is the literal name length in hex — e.g. `"CWeaponSetter",0xd`
(13), `"CWeaponRemover",0xe` (14), `"CWeaponPlacementRule",0x14` (20), `"CAmmoRegenerationWeaponComponent",0x20`
(32). (proven — sanity oracle that the string is the real class name.)

### Manager singletons (registered in the manager registry region `FUN_146cd0bea`, via `thunk_FUN_148f6e780`)
- `CWeaponManager` (proven: `FUN_146cd0bea` @L3209635)
- `CWeaponPlacementRulePersistentManager` (proven: `FUN_146cd0bea` @L3209510; also `FUN_146cd0bea`/`FUN_...`)
- `CDamageCoordinator` (proven: `FUN_146cd0bea` @L3209755)

### Firing / recoil / effects
| Name | `FUN_` | Role |
|---|---|---|
| "WeaponFire" per-shot tick | `FUN_140730540` | Spread PRNG + recoil accumulation + muzzle effect (proven) |
| Wall-penetration firing component | `FUN_14077cac0` | Ray-cast firing, start-ray + penetration (proven) |
| `OnOutOfAmmoFire` handler | `FUN_140e9d1d0` | Dry-fire / out-of-ammo event (proven) |
| `UpdateWeaponReticle` | `FUN_140ebb120` | Reticle/HUD update (proven, large) |
| "WeaponRecoil" | `FUN_147759a20` | Recoil (anim/pose) driver (proven; `callers=[]`) |
| "MountedWeapon" | `FUN_1407b0e40` | Mounted / turret weapon logic (proven; large) |
| `NStateTask_DeployedWeaponTask` | `FUN_14059f370` | Deployed-weapon character state task (proven) |
| `ai/weapons.aisystunec` loader ref | `FUN_1403c89a0` | AI weapon system config (proven) |

## How it works (from the decomp)

### Equip / wield model
A weapon becomes "wielded" by attaching the `CWeaponSetter` component (and the inverse `CWeaponRemover`
tears it down). The wielded/hand state is exposed to the animation + behavior layer through class-name tokens
and behavior conditions rather than a single flag:
- Hand/mode animation modifiers: `MOD_WEAPON_IN_HAND`, `MOD_WEAPON_LEFT` / `MOD_WEAPON_LEFT_HAND`,
  `MOD_WEAPON_RIGHT` / `MOD_WEAPON_RIGHT_HAND`, `MOD_WEAPON_MODE1/2/3`, and the grapple-interplay modifier
  `MOD_GRAPPLE_TETHER_AND_NO_WEAPON_AIM` (proven: string literals; each lives in a size-34 interned-string
  accessor, e.g. `MOD_WEAPON_IN_HAND` → `FUN_142f81300`).
- The dedicated left-hand-wield class `CWeaponWieldedInLeftHand` registers via `thunk_FUN_14aadec10(...)` in
  `FUN_14054e610`; a string getter also returns it (`FUN_1483e5c80`) (proven).
- `WEAPON_MODIFIER_EQUIPPED_ANY` / `WEAPON_MODIFIER_EQUIPPED_RPG` gate behavior on whether *any* weapon /
  specifically an RPG is equipped (proven: `FUN_1430027e0` etc.).

### Behavior-VM conditions (the gate layer)
All of the following register as behavior conditions in the two condition-registry init functions
`FUN_1485e07e0` and `FUN_1485b9730` (each condition is a size-121 name-registration stub calling
`thunk_FUN_14aadec10("<Name>")`; the third caller is the real construction site). This is the queryable
weapon state surface the animation graph / AI use (proven — every row is a distinct registered class string):

| Condition | Reg stub `FUN_` | Checks (inferred from name) |
|---|---|---|
| `CWeaponWieldedCondition` | `FUN_14054e590` | A weapon is wielded |
| `CWeaponEquippedCondition` | `FUN_14054e190` | A weapon is equipped |
| `CWeaponSelectedCondition` | `FUN_14054e410` | Current selected weapon |
| `CWeaponNextSelectedCondition` | `FUN_14054e390` | Next-in-cycle weapon |
| `CWeaponTypeCondition` | `FUN_14054e490` | Weapon type/class match |
| `CWeaponMagazineEmptyCondition` | `FUN_14054e290` | Magazine empty |
| `CWeaponMagazineFullCondition` | `FUN_14054e310` | Magazine full |
| `CWeaponHasPrecisionScopeCondition` | `FUN_14054e210` | Has precision scope |
| `CWieldingWeaponIsInStateCondition` | `FUN_14054e690` | Wielded weapon in a given state |
| `CWeapoIsTryingToFireCondition` | `FUN_14054e110` | Fire input held (sic — original typo "Weapo") |
| `CIsReadyToFireCondition` | `FUN_14054a310` | Ready to fire (cooldown/state) |
| `CCurrentWeaponsHasAmmunitionCondition` | `FUN_140546f10` | Current weapon has ammo |
| `CInventoryHasWeaponAmmunitionCondition` | `FUN_140549990` | Inventory has ammo for weapon |
| `CAimingCondition` | `FUN_140546290` | Player is aiming |
| `CPlayerIsAutoAimingCondition` | `FUN_14054b690` | Auto-aim active |
| `CAiAimTargetDirectionCondition` | `FUN_140546110` | AI aim-target direction |
| `CRecoilState` (blend state) | `FUN_14054bd90` | Recoil blend-state token |
| `CWeaponVelocityBlendState` | (string only) | Weapon-velocity blend state |

### Firing tick — spread + recoil (`FUN_140730540`, "WeaponFire") (proven mechanism, offsets inferred)
This is the core per-shot function. Its `callers=[…]` is a large family of per-weapon-type dispatchers
(`FUN_14072daf0`, `FUN_14072ec20`, `FUN_140730ca0`, `FUN_140732020`, `FUN_140733cb0`, `FUN_140734de0`,
`FUN_140735e50`, `FUN_140736a80`, `FUN_1407376b0`, `FUN_140738020`, `FUN_1407389c0`, `FUN_140739940`, …) —
one wrapper per weapon archetype (proven: caller list).

Mechanism, in order (proven from the code; struct-offset semantics inferred):
1. Fetches the weapon config record via a vtable call `(**(code**)(*param_1 + 0x200))(param_1)` → `lVar10`.
2. **Bullet spread** is drawn from an MSVC-style linear-congruential PRNG seeded by the global
   `DAT_142cb852c`: `seed = seed*0x343fd + 0x269ec3` (the classic MSVC `rand` constants), normalized to a
   float via `((seed>>0x10 | 0x3f8000) << 8)` with normalization constants `DAT_141ca6cac` / `DAT_141ca6c98`.
   Two axes are drawn; each is `((max - min) * rand01 + min) * scale`, where min/max come from config offsets
   `+0x18/+0x1c` and `+0x20/+0x24`. (proven: the arithmetic and the LCG constants are literally in the code.)
3. **Aim modifiers** scale the spread: config `+0x28` when a flag at `param_1+0x31d` is set (inferred: ADS /
   focus-aim), config `+0x2c` when a secondary-fire flag `*(lVar16+0x36c)==1` is set (inferred: secondary
   firing mode). Entering secondary mode emits the metric `metrics_on_secondary_firing_mode`
   (`PTR_s_metrics_on_secondary_firing_mode_142ae15e8`) and the sound event `"ply.firingmodule.start"`
   (proven: both string refs present).
4. **Recoil accumulation:** pitch kick `param_1+0x53c += spreadX*config+0x30`; a running accumulator
   `param_1+0xa9`; and a yaw kick `param_1+0x544 ±= spreadY` with the sign chosen by a fresh PRNG bit
   (`(seed>>0x10)&1`). A recoil clamp uses config `+0x40`/`+0x44` against accumulator `param_1+0xa7`
   (`min` of the two). (proven arithmetic; names inferred.)
5. **Muzzle effect:** when the fire flag `*(param_1[0x62]+0x608)` is set and config `+0x88 > 0`, spawns an
   effect through `FUN_14028e530(DAT_142cafd58, 1, …, config+0x84, "WeaponFire", 0, 0)` — i.e. the "WeaponFire"
   named effect, sized by config `+0x88` and keyed by `+0x84` (proven: literal `"WeaponFire"` arg).

### Wall-penetration ray firing (`FUN_14077cac0`) (proven)
A specialised firing component that ray-casts through geometry. It reads a start-ray position and a direction
component (`thunk_FUN_14838bd40`, `thunk_FUN_14845b720`), performs a scene query
(`thunk_FUN_1476b4130(DAT_142ce3f30 + 0xa0, …)`), and iterates hits. If the component lacks a valid start ray
it logs the developer error `"Weapon (%s) has a WallPenetrationWeapomComponent without a valid start ray
position"` through the **"Weapon"** log category (`FUN_140675120(DAT_142cb6058,"Weapon",…,2)`) — the misspelt
"Weapom" is in the retail binary (proven). A gate `DAT_141d143d4 <= fVar26` plus a per-entity flag at
`param_1+0x385` decides whether a fired/penetration branch runs (proven; semantics inferred).

### Out-of-ammo / dry fire (`FUN_140e9d1d0`)
Dispatches a behavior event named `"OnOutOfAmmoFire"` via a vtable call
`(**(code**)(*param_1 + 0x20))(0,"OnOutOfAmmoFire",0,0)`, then compares an interned id against fields on the
weapon object to select the response (proven).

## Data & config integration

- Weapons are configured as **RTPC entity components**; the class strings above map 1:1 to component-class
  hashes on the data side (`[[rtpc-entity-assembly]]`, `[[composite-assets]]`). `CWeaponConfig` is the tuning
  block; `CWeaponUIInfo` supplies HUD; `CBulletSpawner`/`CDamageController*` wire projectile→damage.
  (inferred from the registrar pattern shared with the other documented systems.)
- AI weapon selection/behavior loads from `ai/weapons.aisystunec` (an ADF/`aisystunec` blob) via
  `FUN_1403c89a0` (proven: path string).
- Weapon-pickup placement in the world is driven by `CWeaponPlacementRule` (per-pickup) governed by the
  singleton `CWeaponPlacementRulePersistentManager` (proven: registrations).
- Source files (proven, from embedded paths): `…\game\equipment\item\weapon\weapon.cpp` (the weapon item
  class) and `…\game\character\statetasks\inputweapontask.cpp` (the character state-task that turns fire/aim
  input into weapon actions — cf. `NStateTask_DeployedWeaponTask` `FUN_14059f370`).

### Animation-graph action / segment vocabulary (proven — all are string literals)
These are the animation-graph tokens the behavior VM drives; each is a size-34 interned-string accessor.

- **Aim:** `ACT_AIM_WEAPON_OR_GRAPPLE` (`FUN_142f060e0` — shared weapon/grapple aim, see grappling_hook.md),
  `ACT_ON_STARTING_AIM`, `ACT_ON_LEAVING_AIM`, `ACT_TO_AIM_FOCUS_WEAPON`, `ACT_FROM_AIM_FOCUS_WEAPON`,
  `ACT_AIM_STRUGGLE`, `ACT_AIM_TARGET_90L/90R_PARTIAL`, `ACT_AIM_FOCUS_ROTATE_90L/90R`,
  `ACT_MOVE_AIM_FOCUS_{0,45L,45R,90L,90R,135L,135R,180}`, `ACT_MOVE_AIM_STRAFE`, `ACT_MOVE_WALK_AIM_FOCUS_*`,
  `ACT_FLICK_*_AIM`.
- **Fire:** `ACT_FIRE` (`FUN_142f15410`), `ACT_FIRE_{0,90L,90R,180L,180R}`, `ACT_GRENADE_FIRE_{0..180}`,
  `ACT_RPG_FIRE_{0,90L,90R}_ADDITIVE`, `ACT_MORTAR_FIRE`, `ACT_REELED_IN_GRENADE_FIRE`,
  cover blind-fire `ACT_STAND_BLINDFIRE_{LEFT,RIGHT}_COVER`, `ACT_CROUCH_BLINDFIRE_{OVER,RIGHT}_COVER`.
- **Reload / holster:** `ACT_RELOAD` (`FUN_142f5c480`), and upper-body variants
  `UB_ACT_RELOAD` (`FUN_143000790`), `UB_ACT_RELOAD_BULLET`, `UB_ACT_RELOAD_SECONDARY`,
  `UB_ACT_CANCEL_RELOAD`, `UB_ACT_HOLSTER` (`FUN_1430000b0`), `ACT_ON_UNEQUIP_WEAPON_SLOW`.
- **Pickup / mounted:** `ACT_PICKUP_WEAPON` (`FUN_142f59160`), `ACT_PICKUP_WEAPON_PARTIAL`,
  `ACT_PICKUP_MOUNTED_WEAPON`, `ACT_ENTER_MOUNTED_WEAPON` / `ACT_ENTER_MOUNTED_GUN` /
  `ACT_ENTER_VEHICLE_MOUNTED_WEAPON`, `ACT_EXIT_MOUNTED_WEAPON`, `ACT_MGUN_STOP_AIMING`,
  `ACT_{PACK,UNPACK}_SHIELDED_MACHINEGUN`.
- **Weapon settings / melee:** `ACT_WEAPON_SETTING1/2/3`, `ACT_MELEE`, `ACT_URGENT_MELEE`, `ACT_STUNT_MELEE`,
  `ACT_HITREACT_STUMBLE_MELEE`.
- **Recoil animation segments:** `RECOIL_ADDITIVE`, `RECOIL_CHARGE_LOOPBACK_SEG`, `RECOIL_LEAN_LOOPBACK_SEG`,
  `RECOIL_PERMISSION_SEG`, `RECOIL_PUMP_SEG`.
- **"No-weapon" segment set** (used to strip weapon anim when disarmed): `SEG_NO_WEAPON_{AIM,DRAW,FIRE,
  HOLSTER,RELOAD,SWITCH}`, `SEG_NO_WEAPON_AIM_NOAIM_TRANSITIONS`.

## Notable constants / tunables (all in `FUN_140730540`) (proven)

- LCG spread/recoil PRNG: `seed = seed*0x343fd + 0x269ec3`, global seed `DAT_142cb852c` (MSVC `rand`
  constants — a *cheap* per-shot RNG, not crypto).
- Float normalization constants for the LCG output: `DAT_141ca6cac`, `DAT_141ca6c98`.
- Per-weapon config offsets consumed per shot (config record from vtable slot `+0x200`):
  `+0x18/+0x1c` and `+0x20/+0x24` = spread min/max (two axes); `+0x28` = aim/ADS spread scale; `+0x2c` =
  secondary-mode spread scale; `+0x30` = recoil pitch gain; `+0x40/+0x44` = recoil clamp pair; `+0x84`/`+0x88`
  = muzzle-effect key/size. (offsets inferred from usage; the arithmetic around them is proven.)
- Wall-penetration gate threshold: `DAT_141d143d4` (`FUN_14077cac0`).

## Call-graph highlights

- RTPC component registry init `FUN_14085fd00` → registers `CWeaponConfig/Setter/Remover/UIInfo/
  PlacementRule` + `CDamageController{,HitAnalyze,HitApply}` (proven: all callers point back to it).
- Behavior-condition registry init `FUN_1485e07e0` **and** `FUN_1485b9730` → register the full `CWeapon*`/
  aiming condition set (proven: shared caller of every size-121 stub above).
- Per-weapon-type fire wrappers (`FUN_14072daf0` … `FUN_140739940`) → `FUN_140730540` ("WeaponFire" tick).
- `FUN_140725d90` → `FUN_140e9d1d0` (out-of-ammo fire handler) (proven: caller edge).
- Manager registry `FUN_146cd0bea` (reached from `FUN_149b2f323`) → `CWeaponManager`,
  `CWeaponPlacementRulePersistentManager`, `CDamageCoordinator`.
- Component/name registrar: every factory calls `FUN_140f27f60(name,len,…)` → `FUN_14aadeee0`; conditions call
  `thunk_FUN_14aadec10(name)`.

## Open questions / lower-confidence

- **Struct offsets are inferred.** The `+0x18…+0x88` config offsets and the player-state flags
  (`+0x31d`, `+0x53c`, `+0x544`, `+0x608`, `+0x385`) in `FUN_140730540`/`FUN_14077cac0` are read from usage,
  not from a typed struct — the *arithmetic* is proven but the field *names* are guesses.
- **Damage numbers not yet located.** `CDamageController*`/`CBulletSpawner` are only seen as registrations;
  the actual damage formula, projectile velocity, and per-weapon damage constants live in the component
  update methods (vtables `141d9dac8` etc.) and/or the `CWeaponConfig`/ADF data — not yet decoded here.
- **Ammo counts / magazine sizes** are data-driven (`CWeaponConfig`, ADF); the magazine-empty/full conditions
  only test state, they don't hold the numbers.
- The `CWeapoIsTryingToFireCondition` typo and `WallPenetrationWeapomComponent` typo are in the shipped binary
  (useful as exact-match anchors, but a reminder the strings are ground truth, not our normalization).
- `MountedWeapon` (`FUN_1407b0e40`, 5052 bytes) and `WeaponRecoil` (`FUN_147759a20`, 2491 bytes) are large and
  only skimmed; a follow-up pass should decode their update loops.

## Appendix — decomp anchors

Strings (exact) and their functions, so any claim is re-verifiable in
`output/_ghidra_jc4/jc4_all_functions_decomp.txt`:

```
"CWeaponConfig"                 -> FUN_140815f80   (registrar arg 0xd)
"CWeaponSetter"                 -> FUN_1408161c0   (0xd)   vtable PTR_LAB_141d9cdd0
"CWeaponRemover"                -> FUN_140816100   (0xe)   vtable PTR_LAB_141d9cdf8
"CWeaponUIInfo"                 -> FUN_140846b90   (0xd)
"CWeaponPlacementRule"          -> FUN_140816040   (0x14)  vtable PTR_LAB_141d9bbb0
"CWeaponWieldedInLeftHand"      -> FUN_14054e610 / string getter FUN_1483e5c80
"CWeaponManager"                -> FUN_146cd0bea @L3209635
"CWeaponPlacementRulePersistentManager" -> FUN_146cd0bea @L3209510
"CDamageController"             -> FUN_140809c00   (0x11)  vtable PTR_LAB_141d9dac8
"CDamageControllerHitAnalyze"   -> FUN_140809cc0
"CDamageControllerHitApply"     -> FUN_140809d80
"CDamageCoordinator"            -> FUN_146cd0bea @L3209755
"CBulletSpawner"                -> FUN_1407432f0   (0xe)
"CAmmoRegenerationWeaponComponent" -> FUN_1406b6f30 (0x20)
"CRecoilState"                  -> FUN_14054bd90
Conditions (registry FUN_1485e07e0 / FUN_1485b9730):
  CWeaponWieldedCondition FUN_14054e590 · CWeaponEquippedCondition FUN_14054e190
  CWeaponSelectedCondition FUN_14054e410 · CWeaponNextSelectedCondition FUN_14054e390
  CWeaponTypeCondition FUN_14054e490 · CWeaponMagazineEmptyCondition FUN_14054e290
  CWeaponMagazineFullCondition FUN_14054e310 · CWeaponHasPrecisionScopeCondition FUN_14054e210
  CWieldingWeaponIsInStateCondition FUN_14054e690 · CWeapoIsTryingToFireCondition FUN_14054e110
  CIsReadyToFireCondition FUN_14054a310 · CCurrentWeaponsHasAmmunitionCondition FUN_140546f10
  CInventoryHasWeaponAmmunitionCondition FUN_140549990 · CAimingCondition FUN_140546290
  CPlayerIsAutoAimingCondition FUN_14054b690 · CAiAimTargetDirectionCondition FUN_140546110
Firing / effects:
  "WeaponFire" tick        FUN_140730540 (LCG 0x343fd/0x269ec3, seed DAT_142cb852c)
  wall-penetration fire    FUN_14077cac0 ("...WallPenetrationWeapomComponent...")
  "OnOutOfAmmoFire"        FUN_140e9d1d0   ("UpdateWeaponReticle" FUN_140ebb120)
  "WeaponRecoil"           FUN_147759a20   "MountedWeapon" FUN_1407b0e40
  NStateTask_DeployedWeaponTask FUN_14059f370
  ai/weapons.aisystunec loader FUN_1403c89a0
Registrars: FUN_140f27f60 -> FUN_14aadeee0 (name,len) ; thunk_FUN_14aadec10 (name)
Log category "Weapon" via FUN_140675120(DAT_142cb6058, "Weapon", msg, 2)
Sound/metric strings: "ply.firingmodule.start", metrics_on_secondary_firing_mode (PTR_..142ae15e8)
Source paths: ...\equipment\item\weapon\weapon.cpp ; ...\character\statetasks\inputweapontask.cpp
```

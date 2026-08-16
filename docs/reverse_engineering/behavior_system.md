# Behavior / condition VM — the gameplay-logic layer driving the animation graph & ability gating

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

## Overview

JC4's behavior layer is not one VM but **two parallel, name-hash–keyed type registries** that share the
same identity mechanism (the cracked lookup3 name hash) and the same "register a factory by class-name
string" idiom:

1. **The gameplay conditional VM** — the `CConditional_*` family (135 classes). These are the boolean
   predicates that gate *game-state* logic: mission/quest/objective status, faction & territory control,
   DLC/progression unlocks, UI/menu state, player traversal state, tether state. They are the operands of
   objective scripts and RTPC-driven gates. Composites (`CConditional_AND/OR/NOT`, plus counted/timed
   variants) let designers build boolean trees. (proven — enumerated from `FUN_148f99a90`.)
2. **The animation-graph condition set** — the `C*Condition` / `CGraph*Condition` family (174 classes).
   These are the *transition guards* on the character animation state graph: speed, airborne, ragdoll,
   cover, vehicle, weapon-wield, grapple/tether, anim-layer overrides, graph-node occupancy. They gate
   which **`ACT_*` action** the graph may play next. (proven — enumerated from `FUN_1485b9730`.)

The action vocabulary — 315 `ACT_*` constants in C++ namespace **`NCharacter`** — is the *output* side:
the game queues an action onto a character (`character->QueueAct(NCharacter::ACT_...)`) and the animation
state graph, guarded by the `C*Condition` set, transitions to play it. (proven — embedded trigger-script
string + the per-action hash-init functions.)

Both registries key on the **lookup3 `hashlittle`** name hash we already cracked
(memory `[[name-hash-cracked]]`); the same hash function `FUN_14aadeee0` that hashes asset paths hashes
these class/action name strings. (proven.)

## Key classes & functions

| Name string / role | FUN_ | One-line role |
|---|---|---|
| **lookup3 name hash** (`hashlittle`) | `FUN_14aadeee0` (via thunk `FUN_140f27f60`) | Hashes any `(ptr,len)` — the shared identity for classes, actions, asset paths. (proven) |
| strlen+hash convenience wrapper | `FUN_14aadec10` | Computes strlen then calls the hash; used by the graph-condition name getters. (proven) |
| **CConditional master registry** | `FUN_148f99a90` (size 7930) | Registers all 184 `CConditional_*`/`CObjective*`/… prototypes into a name-hash→prototype map. (proven) |
| Registry insert (hashmap) | `FUN_147cafcd0` (`thunk_FUN_147cafcd0`) | `key = hash(name)`; open-hash bucket by `key % tablesize`, chained; inserts `{key → prototype*}`. (proven) |
| Prototype allocator | `thunk_FUN_1496a12b0(8)` | Allocates the 8-byte (vtable-pointer-only) prototype object for each conditional class. (proven) |
| Example conditional prototype ctor | `FUN_148f99a90` body: `*puVar1 = &PTR_LAB_141d91f18` for `CConditional_AND` | Prototype = a single vtable ptr; cloned/typed on deserialize. (proven) |
| Magic-static name-hash cache (per class) | e.g. `FUN_14065ec80` → `DAT_142cb5ff8 = FUN_140f27f60("CConditional_AND",0x10)` | Thread-safe one-time cache of a class's name hash. (proven) |
| **Graph-condition factory registry** | `FUN_1485b9730` (size 6979) | Registers 198 `C*Condition` entries as `{size(0x10/0x18), ctor LAB_, name-hash}`. (proven) |
| Graph-condition registry insert | `thunk_FUN_1478b1d60(hash,&{ctor,size})` | Binds name-hash → `{constructor, object size}`. (proven) |
| Graph-condition **unregister** pass | `FUN_1485e07e0` (size 2396) | Paired teardown: same name getters → `thunk_FUN_1478c1ad0(hash)` removes each. (proven) |
| Per-class graph name getter | e.g. `FUN_14054b310` → `thunk_FUN_14aadec10("COverrideAdditiveAnimLayerCondition")` | Magic-static that yields a graph condition's name hash. (proven) |
| **Action hash init** (per `ACT_`) | e.g. `FUN_142f07090` → `_DAT_142cb38dc = FUN_140f27f60("ACT_CLOSE_PARACHUTE",0x13)` | One per action; caches the action's name hash in a global, called from the `0x140008xxx` init table. (proven) |

## How it works (from the decomp)

### Identity: everything is a lookup3 name hash

`FUN_140f27f60(name, len[, 0])` is a thin thunk to `FUN_14aadeee0`, which is the byte-exact **Jenkins
lookup3 `hashlittle`** (the same 12-byte-block mixing with the `0x21524111` seed constant and the final
`(a^c)-(rot)` avalanche visible in the body). This is the identical function used for the TAB/ADF name
hash (`[[name-hash-cracked]]`). Every condition class, every action, is reduced to a 32-bit key by this
function. (proven — `FUN_14aadeee0`; matches the cracked hash spec.)

Each class/action gets a **magic-static** getter that hashes its literal once and caches it in a `DAT_`
global (`_Init_thread_header`/`_footer` guard). Example: `FUN_14065ec80` caches
`hash("CConditional_AND")` (len `0x10`) in `DAT_142cb5ff8`; `FUN_142f07090` caches
`hash("ACT_CLOSE_PARACHUTE")` (len `0x13`) in `_DAT_142cb38dc`. The `len` argument passed alongside each
literal is exactly its `strlen`, confirming these are name-string hashes, not opaque IDs. (proven.)

### Registry A — the gameplay conditional VM (`CConditional_*`)

`FUN_148f99a90` is a single ~8 KB function that is the **class table** for the conditional/objective
scripting system. Its body is 184 repetitions of the same three-instruction idiom:

```c
puVar1 = thunk_FUN_1496a12b0(8);          // alloc an 8-byte prototype
*puVar1 = &PTR_LAB_141d91f18;             // install this class's vtable  (here: CConditional_AND)
thunk_FUN_147cafcd0(param_1, "CConditional_AND", puVar1);   // register name-hash -> prototype
```

`FUN_147cafcd0` is a textbook open-addressed hashmap insert: it strlens the name, computes
`key = FUN_140f27f60(name,len,0)`, indexes `bucket = key % *(ushort*)(table+0x18)`, walks the chain
(`next` at `+4` of each 0x10-byte slot), and if absent inserts `{key, prototype}` via
`thunk_FUN_14768f3e0`. So the registry is `HashMap<lookup3(name) → prototype*>`. (proven.)

Every prototype is **8 bytes = one vtable pointer** (all 184 allocations are `thunk_FUN_1496a12b0(8)`).
This is the **prototype/factory pattern**: at data-load time an RTPC/ADF blob names a conditional class,
the loader hashes the name, finds the prototype, and clones/constructs a typed instance whose vtable
carries the class's `Deserialize`/`Evaluate` behavior. The registered set is not only conditionals — it
also registers the whole objective-script class family (`CObjectiveGoal_*`, `CObjectiveParam_*`,
`CDaredevil*`, `CSpawnParam_*`, `CDialogueLine`, …), which is why this one function is the hub of the
mission/objective scripting layer. (proven for the registry mechanism; the vtable slot *names*
Deserialize/Evaluate are **inferred** — the data-section vtables are not in the decomp text.)

The **composite operators** are themselves registered classes, so boolean trees are built by nesting
prototype instances: `CConditional_AND`, `CConditional_OR`, `CConditional_NOT`, plus the stateful
variants `CConditional_AND_Counter` (size hint `0x18` in its hash getter) and `CConditional_AND_Duration`
(`0x19`), and `CConditionalLock`, `CConditional_Manual`, `CConditional_Misc`. A composite holds child
conditionals and its `Evaluate` folds them (`AND` = all children true; `_Duration`/`_Counter` add a
time/hit threshold before latching). (proven that these are registered composites; the fold semantics are
**inferred** from the names + the counted/timed size hints.)

### Registry B — the animation-graph condition set (`C*Condition`)

`FUN_1485b9730` is the parallel registry for the **animation state graph's transition guards**. Its body
is 198 repetitions of:

```c
local_10 = 0x10;                          // sizeof(this condition class)  (some are 0x18)
local_18 = (code*)&LAB_140528f10;         // the class constructor
uVar1 = FUN_14054b310();                  // this class's cached name hash (e.g. COverrideBaseAnimLayerCondition)
thunk_FUN_1478b1d60(uVar1, &local_18);    // register: name-hash -> {ctor, size}
```

Unlike Registry A (which stores a pre-built prototype), Registry B stores a **`{constructor, size}`
descriptor** so the graph loader can allocate and placement-construct a transition-condition of the right
type when it parses a graph edge. The paired teardown `FUN_1485e07e0` walks the *same* name getters and
calls `thunk_FUN_1478c1ad0(hash)` to unregister each — confirming these are lifetime-managed registrations
into a global graph-condition type table. (proven.)

These guards are what a graph transition asks before firing: a transition from an idle node to
`ACT_CLOSE_PARACHUTE` is gated by conditions like `CInAirCondition`, `CIsGrappleActiveCondition`, etc.
The base type of the family is `CGraphTransitionCondition` (hash cached in `DAT_142cb45dc` by
`FUN_140548c90`); `CGraphTransitionClearCondition`, `CGraphEntryCondition`, `CGraphExitCondition` are the
structural transition/entry/exit guards. (proven that these are registered; the exact per-edge evaluation
call is **inferred** — evaluation happens through the constructed object's vtable.)

### The action side (`ACT_*`) and dispatch

Actions are **not** objects — they are 315 named 32-bit hash IDs in namespace `NCharacter`. Each has a
tiny init function (e.g. `FUN_142f060e0`, `FUN_142f07090`, `FUN_142f06a70`) that hashes its literal and
stores the result in a dedicated global (`_DAT_142cb38dc` etc.); these inits are invoked from the
`0x140008xxx` startup dispatch table. At runtime, gameplay code queues an action by its cached ID:

```
character->QueueAct(NCharacter::ACT_CLOSE_PARACHUTE);
```

This exact call survives verbatim in an embedded anti-tamper trigger-script string (a mini C++ DSL the
engine compiles/interprets from XML `<TriggerAction>` blocks). Other captured scripts show the same object
model: `character->m_LifeTime`, `character->GetUnitHealth()`, `CancelReelIn(character, hook)`,
`reel_params->m_Timer`. So the behavior layer's *authoring* surface is: **conditions (the `C*` predicates)
guard transitions; a transition queues an `NCharacter::ACT_*`; the animation graph plays it.** (proven —
the `QueueAct(NCharacter::ACT_CLOSE_PARACHUTE)` and reel-cancel strings are literal in the binary.)

Dispatch is therefore **hash-matched**, not string-matched at runtime: an `ACT_` global holds a hash, a
graph transition record carries a target-action hash, and the graph plays the animation clip bound to that
hash once its `C*Condition` guard evaluates true. (proven that IDs are hashes; the graph's hash→clip match
step is **inferred**.)

## Data & config integration

- **Conditionals are RTPC/objective-script operands.** The prototype registry `FUN_148f99a90` co-registers
  `CConditional_*` with `CObjectiveGoal_*`/`CObjectiveParam_*`/`CDaredevil*` — i.e. the conditional VM is
  the predicate sublanguage of the mission/objective system (see `missions_progression.md`). A designer's
  objective goal `CObjectiveGoal_Conditional` embeds a `CConditional_*` tree. (proven co-registration;
  the goal→conditional embed is **inferred** from the class name pairing.)
- **Class name ↔ entity/RTPC hash.** Because identity is the same lookup3 hash used for RTPC property/class
  keys (`[[rtpc-entity-assembly]]`, `[[name-hash-cracked]]`), a `CConditional_*` string here hashes to the
  same 32-bit value an RTPC blob uses to name that conditional. This is the bridge to the data side.
  (inferred — same hash function, not yet round-tripped against a specific RTPC blob.)
- **Trigger scripts.** The `<ANTITAMPER_TRIGGER_DATABLOCK>` / `<TriggerAction>` XML embeds show gameplay
  logic authored as a C++-like DSL over `character->…` and `ACT_*`; these datablocks are compiled behavior
  hooks, distinct from the two registries but consuming the same action vocabulary. (proven strings.)

## Notable constants / tunables

- lookup3 seed `0x21524111` (i.e. `-0xdeadbeef` mixing constant) and the block stride `0xc` (12 bytes) in
  `FUN_14aadeee0` — the signature of Jenkins `hashlittle`. (proven.)
- Conditional prototype size = **8 bytes** (vtable-only), all 184 via `thunk_FUN_1496a12b0(8)`. (proven.)
- Graph-condition object sizes = **0x10 or 0x18 bytes** (the `local_10` immediates in `FUN_1485b9730`);
  stateful conditions (counters/timers) take the larger 0x18. (proven.)
- Anti-tamper example thresholds: parachute force-close after `m_LifeTime > 3600.f` s; reel auto-cancel
  after `m_Timer > Maxf(1.f, 4.f * GetUnitHealth())`. (proven — literal in the trigger-script strings.)

## Call-graph highlights

- `FUN_140f27f60` (hash thunk) → `FUN_14aadeee0` (lookup3). Callers: hundreds of per-class/per-action
  magic-static getters (`FUN_14065ec80`, `FUN_142f07090`, …). (proven.)
- `FUN_148f99a90` → `thunk_FUN_1496a12b0` (alloc 8) + `thunk_FUN_147cafcd0` (insert) ×184. `FUN_147cafcd0`
  → `FUN_140f27f60` (key) + `thunk_FUN_14768f3e0` (bucket insert). (proven.)
- `FUN_1485b9730` (register ×198) and `FUN_1485e07e0` (unregister) both fan out to the ~198 graph-condition
  name getters (`FUN_14054b310`, `FUN_140548c90`, …); register → `thunk_FUN_1478b1d60`, unregister →
  `thunk_FUN_1478c1ad0`. (proven.)
- Action init table at `0x140008xxx` → per-action hash-init funcs (`FUN_142f060e0`, `FUN_142f06710`, …).
  (proven.)

## Open questions / lower-confidence

- **Exact vtable layout** of a conditional prototype (which slot is `Evaluate`, which is `Deserialize`) is
  not recoverable from the text dump — the vtables (`PTR_LAB_141d91*`) live in the data section. Confirming
  the `bool Evaluate(context)` signature needs a live x64dbg vtable read. (inferred.)
- **How a graph transition selects its `ACT_`** (hash→clip binding table) is inferred, not located. The
  graph edge records that carry `{condition, target action}` were not read. (inferred.)
- Whether `CConditional_AND` evaluation short-circuits, and how `_AND_Counter`/`_AND_Duration` latch, is
  inferred from names + size hints only. (inferred.)
- The two registries are assumed independent (gameplay vs animation); no cross-registration was observed,
  but a bridge (e.g. a graph condition wrapping a gameplay conditional) has not been ruled out.
  (speculative.)

## Appendix A — decomp anchors (re-verifiable)

| Anchor | FUN_/DAT_ |
|---|---|
| lookup3 hash | `FUN_14aadeee0`; thunk `FUN_140f27f60`; strlen wrapper `FUN_14aadec10` |
| CConditional master registry | `FUN_148f99a90` |
| hashmap insert | `FUN_147cafcd0`; bucket-add `thunk_FUN_14768f3e0`; alloc `thunk_FUN_1496a12b0` |
| CConditional_AND hash cache | `FUN_14065ec80` → `DAT_142cb5ff8`; `_Counter` `FUN_14065ed20`; `_Duration` `FUN_14065edc0` |
| Graph-condition register / unregister | `FUN_1485b9730` / `FUN_1485e07e0`; insert `thunk_FUN_1478b1d60`, remove `thunk_FUN_1478c1ad0` |
| CGraphTransitionCondition hash | `FUN_140548c90` → `DAT_142cb45dc` |
| Action hash inits | `ACT_AIM_WEAPON_OR_GRAPPLE` `FUN_142f060e0`→`_DAT_142cb398c`; `ACT_CLOSE_PARACHUTE` `FUN_142f07090`→`_DAT_142cb38dc`; `ACT_CLOSE_HOVERBOARD` `FUN_142f06a70`; `ACT_CLIMB_OBSTACLE` `FUN_142f06a10` |
| Runtime API string | `character->QueueAct(NCharacter::ACT_CLOSE_PARACHUTE)` (in a `<TriggerAction>` datablock) |

## Appendix B — vocabulary by category (counts)

Totals (unique strings, ripgrep `sort -u`): **135** `CConditional_*` · **174** `C*Condition` (animation-graph
+ misc) · **315** `ACT_*`. Registry sizes: **184** entries in `FUN_148f99a90` (conditionals + objective
classes), **198** in `FUN_1485b9730` (graph conditions). (proven counts.)

### B.1 — `CConditional_*` gameplay conditional VM (135)

| Category | ~Count | Representative members |
|---|---|---|
| Logic / composite / meta | 10 | `AND`, `OR`, `NOT`, `AND_Counter`, `AND_Duration`, `CConditionalLock`, `Manual`, `Misc`, `TimestampAge`, `Playtime` |
| Missions / quests / objectives | ~26 | `QuestStatus`, `IsQuest{Active,Available,Completed}`, `MissionStatus`, `IsMission{Active,Completed}`, `ObjectiveGoalStatus`, `IsObjective{Active,Completed,Failed,Hidden,Tracked,CurrentInSequence,ContentAvailable}`, `IsAnyHardFailObjectiveActive`, `EncounterStatus`, `EncounterResultCount`, `EncounterPreviousResult`, `IsTutorialPhase{Active,Completed}`, `ContentIntroductionStatus`, `AvailableContentStatus` |
| Faction / territory / nodes (region-control metagame) | ~24 | `FactionStatus`, `IsNode{Secured,Cleared,Tracked,Discovered,Unsafe}`, `IsNodeNeighbor{Secured,Cleared,Unsafe}`, `IsInClearedNode`, `IsInSecuredTerritory`, `IsLocationInSecuredTerritory`, `IsLocation{Complete,Discovered}`, `LocationTypeProgression`, `IsNearActiveFrontline`, `IsPlayerInsideNode(DiscoveryVolume)`, `IsPlayerInside{Biome,InfestationZone}`, `DemonDomePercentage`, `TacticalConnectionStatus`, `IsPOILocked` |
| Progression / unlocks / DLC / economy | ~24 | `IsDLCUnlocked`, `HasPilotUnlocked`, `IsPilotReady`, `AreSupplyDropsUnlocked`, `HasUnlockedSupplyItems`, `IsCowGunUnlocked`, `IsCauldronUnlocked`, `BackerXp`, `BackerLevel`, `IsMediaRevolutionMilestoneApplied`, `CanUnlockNextMediaRevolutionMilestone`, `IsMonthlyChallenge{Complete,Available,RewardClaimed}`, `IsDaredevilRaceTierReached`, `HasBeatenDaredevilRacesByTier`, `IsAgencyRingCourse{Active,Complete}`, `IsRetooler{FeatureUnlocked,GroupSelected,LoadoutSelected,OptionEquipped}`, `HasEnoughChaos`, `HeatUnitCount`, `IsPlayerInHeatAOO` |
| Player state / traversal | ~18 | `IsPlayerInWind`, `IsPlayerInWingsuit`, `IsPlayerInParachute`, `IsPlayerSwimming`, `IsPlayerOnHoverBoard`, `IsPlayerOn{Magrail,AnyMagrail}`, `IsPlayerPolymorphed`, `IsPlayerInState`, `IsPlayerInWeather`, `VehicleState`, `VehicleState2`, `IsPlayerInVehicle{,Type}`, `IsPlayerInGivenVehicle`, `HasPassenger`, `HasPassengers` |
| Grapple / tether (signature mechanic) | ~8 | `TetherState`, `TetherState_Simple`, `TetherStateBase`, `TetherConnection`, `IsTetheredToObject`, `IsReeledOnObject`, `DeployedTetherCount`, `IsObjectConnectedToNumWinches`, `ConnectedByWire` |
| Weapons / combat / targeting | ~5 | `IsWeaponEquipped`, `IsWeaponInInventory`, `IsWeaponTypeInInventory`, `IsFiringWeapon`, `IsTargeting{Object,Shape}` |
| UI / menu / HUD / tracking | ~20 | `IsMenuActive`, `IsMenuIconHovered`, `IsSupplyLoadoutMenuActive`, `IsPopupModuleActive`, `IsPilotSelectedIn{UI,FastTravelUI}`, `IsSupplyCategorySelectedInUI`, `IsHoveringOver{MapIcon,DiscoveryLocation}`, `ArModeState`, `IsRadioStationActive`, `IsLaughTrackEnabled`, `IsFastTravelAvailable`, `ObjectTrackerStatus`, `IsObjectTracked`, `IsAnythingTracked`, `IsInputActionTaken`, `SpawnTags`, `IsGroupActive`, `IsTimeOfDay`, `IsAnyCutsceneActive`, `CollectibleStatus`, `CollectibleCompletion` |

### B.2 — `C*Condition` animation-graph transition guards (174)

| Category | ~Count | Representative members |
|---|---|---|
| Graph structure / transition / entry-exit | ~30 | `CGraphTransitionCondition`(base), `CGraphTransition{Clear,NodeDistance,WarpDistance}Condition`, `CGraphEntry{,Clear,Occupation,NodeDistance,NodeRelativeOrientation,NodeRelativePosition,WarpDistance,IsFacingNode,AngleToUp,LineNodeAlignment,OccupiedByAuthority,ParentVehicleSpeed}Condition`, `CGraphNode{Type,Identifier,Occupied,RelativeOrientation,RelativePosition}Condition`, `CGraph{AngleToUp,Exit,OccupiedByAuthority,ParentVehicleSpeed,ParentVehicleDirToFaceAngle}Condition`, `CWithinSegmentCondition`, `CNoLoopbackCondition` |
| Logic / blackboard / events | ~12 | `CAndCondition`, `COrCondition`, `CCompareCondition`, `CRandomCompareCondition`, `CStateBitCondition`, `CContextBitIsSetCondition`, `CBlackboardValueCompareCondition`, `CDoesBlackBoardValueExistCondition`, `CWaitForEventCondition`, `CTrackEventCondition`, `CLinkTargetEventCondition`, `COneOffCondition` |
| Locomotion / movement / spatial | ~16 | `CSpeedCondition`, `CHorizontalSpeedCondition`, `CVerticalSpeedCondition`, `CInAirCondition`, `CGroundContactCondition`, `CHeightOverGroundCondition`, `CFall{Time,Distance,Transition}Condition`, `CSurfaceNormalCondition`, `CUpVectorAngleCondition`, `CViewPitchCondition`, `CMovementInputCondition`, `CInputMagnitudeCondition`, `CCheckMovementStyleCondition`, `CTargetOnFootMovementSpeedCondition` |
| Ragdoll / recovery | ~9 | `CRagdoll{Velocity,Contact,Blending,IsSettled}Condition`, `CRagdollingTimerCondition`, `CAirborneHookedRagdollCondition`, `CBlockedGetUpCondition`, `CIsHighImpactImpulseCondition`, `CHitDirectionCondition` |
| Water / swim | 3 | `CInWaterCondition`, `CInWaterfallCondition`, `CWaterDepthCondition` |
| Weapons / aim / combat / damage | ~22 | `CWeapon{Wielded,Type,Selected,NextSelected,MagazineFull,MagazineEmpty,HasPrecisionScope,Equipped}Condition`, `CWieldingWeaponIsInStateCondition`, `CWeapoIsTryingToFireCondition`, `CIsReadyToFireCondition`, `CAimingCondition`, `CPlayerIsAutoAimingCondition`, `CIsCrossbowModeCondition`, `CIsHoldingPrimedGrenadeCondition`, `CInventory{,HasWeaponAmmunition}Condition`, `CCurrentWeaponsHasAmmunitionCondition`, `CHitZoneCondition`, `CHealthCondition`, `CLinkHealthCondition`, `CIsTakingDamageCondition` |
| Cover / AI / NPC | ~12 | `CFacingCoverCondition`, `CInfrontOfCoverCondition`, `CDistanceToCoverCondition`, `CIs{Left,Right}CoverCondition`, `CAiUsingCoverCondition`, `CAiAimTargetDirectionCondition`, `CNpcIsRunningCondition`, `CAcquireCloseCombatTargetCondition`, `CCheckMentalStateCondition`, `CInteractionBehaviorTriggerCondition`, `CCharacterIsAliveCondition` |
| Vehicle | ~24 | `CVehicle{Float,Bool,Motorbike,Helicopter,Boat,Cabbed,Below,CrashImpulse,JerkAcceleration,IsUpsideDown,LockedForPlayer,LockedForHijack}Condition`, `CVehicleDoor{IsClosed,Exist}Condition`, `CEntryVehicle{LockedForPlayer,LockedForHijack,DoorExist,DoorIsClosed}Condition`, `CVehicleLockedFor{Player,Hijack}Condition`, `CIsInCargoBayCondition`, `CUprightMotorbikeCondition`, `CIsAttachedToVehicleCondition`, `CIsTargetStuntingOnMyVehicleCondition`, `CIsInAAgunCondition` |
| Grapple / tether / magrail | ~13 | `CTetherTensionCondition`, `CIsTetheredCondition`, `CReel{Land,Distance,Direction}Condition`, `COriginalReelVerticalDirCondition`, `CDualTetheringInputCondition`, `CHasActiveWireCondition`, `CIsGrappleActiveCondition`, `CCombatantNotTetherableCondition`, `CMagrail{,Attached}Condition`, `CIsAttachedToGraphCondition`, `CAttachedCondition` |
| Anim-layer override | 5 | `COverride{Base,Additive,Upper,GlobalPartial,GlobalAdditive}AnimLayerCondition` |
| Wind / weather (signature) | 3 | `CVerticalWindInAir{Height,Speed}Condition`, `CIsInWeatherCondition` |
| Demon / possession (DLC) & misc state | ~14 | `CDemonVelocityCondition`, `CDemonWithinPossessionDistanceCondition`, `CGhostModeCondition`, `CIsPlayerCondition`, `CIsAbilityEnabledCondition`, `CDashEnabledCondition`, `CHoverboardInputCondition`, `CIsModifierSetCondition`, `CIsProgressionVitalCondition`, `CAnimation{Finish,TimeInterval,RuleFinish}Condition`, `CDistanceTo{Target,Camera}Condition`, `CIsAnimationPreviewerModeCondition`, `CRelative{Target,NextMoveAndTarget,FaceAndLook}Condition`, `CRayCastFanCondition`, `CForwardRayCastFanCondition` |

### B.3 — `ACT_*` action vocabulary (315), namespace `NCharacter`

| Category | ~Count | Representative members |
|---|---|---|
| Locomotion (turn/rotate/move/idle) | ~90 | `MOVE_*`, `TURN_*`, `ROTATE_*` (incl. `CW/CCW`), `STOP_TURN_*`, `SHUFFLE_ROTATE_*`, `IDLE_TO_{WALK,RUN}_START_*`, `IDLE_TO_ACCEL_CURVE`, `FLICK_*`, `STEP_OVER`, `STEP_BACKWARD`, `NO_{TURN,MOVE_INPUT,ACC}`, `ACC_{FORWARD,BACKWARD}`, `FRONT_TO_BACK_MOVE_TRANSITION_*`, `MOVE_{RELAXED,PRECISION_MODE,CINEMATIC_MODE,NO_AIM}` |
| Traversal (parachute/wingsuit/hoverboard/vault/dash) | ~24 | `TRIGGER_{PARACHUTE,WINGSUIT,HOVERBOARD}`, `CLOSE_{PARACHUTE,WINGSUIT,HOVERBOARD}`, `FORCE_{FREEFALL,FASTFALL,PARACHUTE_IDLE}`, `SEQUENCE_TRIGGER_{WINGSUIT,FREEFALL}`, `{START,END}_VAULT`, `LEDGE_{CLIMB,VAULT}`, `CLIMB_OBSTACLE`, `JETPACK_START`, `REGULAR_DASH`, `HOMING_DASH`, `DASH_ABORT`, `FAST_TRAVEL_FREEFALL_{DROP,ZOMBIE}`, `PULLCHORD`, `RAPPELL_DROP`, `FALL` |
| Grapple / tether / reel (signature) | ~40 | `GRAPPLE_*` (`FIRE`, `ATTACH`, `LAND*`, `{FWD,BWD}_{LEFT,RIGHT}_YANK*`, `HANG`, `RAGDOLL`, `TETHER_FIRE`, `VEHICLE_YANK`, `STUNT_*`, `UPSIDEDOWN`), `GRPL_{JUMP_UP,JUMP_AWAY*,CLIMB,RELEASE,FORCE_RELEASE}`, `SIMPLE_GRAPPLE_*` (`FIRE`, `CANCEL`, `REEL`, `REEL_LAND_{FLOOR,CEILING,WALL,LEDGE,STUNT_*}`, `INSTANT_VEHICLE_STUNT_*`), `PRE_REEL`, `REEL_KICKED`, `SLINGSUIT_TO_REEL`, `{PUSH,PULL}_GRAPPLE` |
| Vehicle (enter/exit/hijack/turret/mounted) | ~30 | `ENTER_VEHICLE*`, `EXIT_VEHICLE*`, `EJECT_FROM_VEHICLE*`, `ARMORED_HIJACK_EJECT`, `GET_HIJACK`, `PANIC_EXIT_VEHICLE`, `{CLOSE,OPEN}_{LEFT,RIGHT}_DOOR_FROM_INSIDE`, `SWITCH_SEAT`, `SWITCH_DRIVING_DIRECTION`, `ENTER/EXIT_MOUNTED_{WEAPON,GUN}`, `TURRET_{IDLE,TETHERED,PRE_DEATH,DEAD}`, `STUNT_VEHICLE*`, `{REVERSE,UPRIGHT}_MOTORBIKE`, `IDLE_VEHICLE`, `{HAVE,NO}_VELOCITY_VEHICLE` |
| Combat / weapon / aim | ~40 | `FIRE`, `RPG_FIRE_*`, `GRENADE_FIRE_*`, `MORTAR_{START_AIMING,STOP_AIMING,FIRE}`, `MGUN_STOP_AIMING`, `RELOAD`, `AIM*`, `MOVE_AIM_FOCUS_*`, `MOVE_{WALK_AIM_FOCUS,RELATIVE_FOCUS}_*`, `AIM_{TARGET,FOCUS_ROTATE,STRUGGLE}_*`, `TO/FROM_AIM_FOCUS_WEAPON`, `ON_{STARTING,LEAVING}_AIM`, `MELEE`, `URGENT_MELEE`, `STUNT_MELEE`, `PUNCH_{L,R}_JAB`, `PICKUP{,_WEAPON,_MOUNTED_WEAPON}`, `WEAPON_SETTING`, `GRAVITY_GUN_{ATTRACT,THROW,SEARCH_ENTER,ABORT}`, `WIND_{GUST,CANNON_ENGAGE,CANNON_DISENGAGE}`, `NPC_THROW_GRENADE`, `{PACK,UNPACK}_SHIELDED_MACHINEGUN`, `ON_UNEQUIP_WEAPON_SLOW` |
| Cover | ~24 | `STAND_*_COVER`, `CROUCH_*_COVER` (`LOOK_{LEFT,RIGHT,OVER}`, `HIDE`, `GRENADETHROW_*`, `BLINDFIRE_*`, `ATTACK_*`), `STOP_AND_ENTER_COVER`, `COVER_TO_IDLE`, `EXCEPTION_COVER_TO_IDLE`, `INVALIDATE_COVER` |
| Reactions / damage / death / ragdoll | ~35 | `HITREACT_*` (`BULLET`, `MELEE`, `GRAPPLE`, `FLY`, `RAGDOLL`, `STUMBLE*`, `REEL_KICK_*`, `VEHICLE_IMPACT_*`), `REACT_EXPLOSION_*`, `DIE_EXPLOSION_*`, `RAGDOLL_{IMPACT,INAIR_STABILIZATION_*}`, `{FULL,FORCE_DEATH}_RAGDOLL`, `LOCOMOTION_REACTION_*`, `DAMAGE`, `EVADE` |
| Swim | ~10 | `SURFACE_SWIM_{START,IDLE,CRAWL,TO_UW}_*`, `SURFACE_IDLE_QUICKTURN_*`, `UW_SWIM_{IDLE,FROG,IDLE_TURN}_*`, `UW_TO_SURFACE_SWIM` |
| Demon / possession (DLC) | ~5 | `DEMON_PASSIVE`, `DEMON_HUMAN_POSSESSION_STRUGGLE_INTO`, `POSSESSED`, `AIM_STRUGGLE` |
| Meta / control / graph-internal | ~20 | `NONE`, `NULL_GLOBAL_PARTIAL`, `UPDATE`, `TRANSIT`, `TELEPORT_EXIT`, `{INTERRUPT,EXIT}_CONTEXT_ACTION`, `ABORT_CUSTOM_ANIMATION`, `ANIMATION_FINISHED`, `INVALIDATE_{L,R,COVER}`, `CUSTOM_DLC`, `TO_IDLE{,_ONE_OFF}`, `DETACH_ONE_OFF`, `KEEP_HANGING`, `IDLED_FOR_A_LONG_LONG_TIME`, `{RELAXED_TO_URGENT,URGENT_TO_{RELAXED,EXTREME},EXTREME_TO_URGENT}` |

*(Category counts are approximate groupings of the 315/174/135 unique strings by theme; exact per-string
membership is in the raw ripgrep dumps. The three totals and the two registry entry-counts are proven.)*

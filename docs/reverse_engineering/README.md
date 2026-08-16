# JC4 Reverse Engineering — Systems & Mechanics (decomp-grounded)

This directory documents **how Just Cause 4's game systems actually work**, reconstructed from the Ghidra
decompilation of `JustCause4.exe`. Every claim is grounded in the decompiled code (function addresses,
control flow, constants) — not guessed from playing the game.

## The decompilation

- **Source:** `output/_ghidra_jc4/jc4_all_functions_decomp.txt` — **162,115 functions**, ~219 MB, 7.2M lines.
  Produced by `tools/ghidra/DecompileJC4.java` (headless Ghidra 12.1; see `docs/binary_recon.md`).
- **Format:** each function is a block delimited by a header line:
  ```
  ==== FUN_140abc123 @0x140abc123  size=NNN  callers=[FUN_..., ...] ====
  <decompiled C body>
  ```
- **Stripped, but string-rich.** No symbols — everything is `FUN_<addr>` / `DAT_<addr>`. **BUT** the binary
  registers gameplay systems by their real **C++ class names** and **action/condition names** as string
  constants, which survive in the decomp. This is the primary handle for finding a system.

## How to find & document a system (methodology)

1. **Anchor on names.** `rg -i '"CGrapplingHook'` (or the system's class/action strings) over the dump →
   the functions that reference the name are the class factory / registration / vtable setup.
   Useful string families: `"C<ClassName>` (e.g. `CGrapplePoint`), `"ACT_<ACTION>` (behavior actions,
   e.g. `ACT_CLOSE_PARACHUTE`), `"C<...>Condition` (behavior conditions).
2. **Read the function block.** Find the `==== FUN_… ====` header, read its body. Note the `callers=[]`
   list and the `FUN_`/`DAT_` it calls/touches.
3. **Walk the call graph.** Follow callees (what it does) and callers (who drives it). Record the addresses.
4. **Tie to the data side.** Systems are configured by RTPC entity components + ADF; cross-reference the
   property/class hashes we've already cracked (`docs/formats/`, memory `[[rtpc-entity-assembly]]`,
   `[[composite-assets]]`). A class name here often maps to an entity component class hash there.
5. **Grade every claim** `proven | inferred | speculative` (project mandate). "Proven" = read directly from
   the decomp. Cite the `FUN_<addr>` so a reader can verify.

## System map (the survey)

Mined from the class-name string inventory. Each row → its own deep-dive doc.

| Doc | System | Primary anchors |
|---|---|---|
| [grappling_hook.md](grappling_hook.md) | **Grappling hook / tether / reel / winch** (the signature mechanic) | `CGrapplingHook`, `CGrapplePoint`, `CTetherTensionCondition`, `CReel*BlendState`, `CDualTetheringInputCondition`, `CVehicleWinchController`, `CWinchControllerSystem`, `ACT_AIM_WEAPON_OR_GRAPPLE` |
| [wind_and_weather.md](wind_and_weather.md) | **Wind & extreme weather** (tornadoes, storms — JC4's signature) | `CWindTunnelManager`, `CForcePulse`, `CLocalWindObject`, `CTopographicalWind`, `CPfxWindPhysicsSystem`, `CVerticalWindInAir*Condition`, `CConditional_IsPlayerInWind`, `CLightningManager`, `CWeatherPreset`, `CEnvironmentPresets` |
| [traversal_movement.md](traversal_movement.md) | **Player traversal** — parachute, wingsuit, hoverboard, locomotion | `CParachuteObject(State)`, `CParachuteBlendState`, wingsuit/`ACT_CLOSE_WINGSUIT`, `ACT_CLOSE_HOVERBOARD`, `CMoveInput{Yaw,Pitch,Offset}Modifier`, `CRollModifier`, `CCharacter` |
| [vehicles.md](vehicles.md) | **Vehicles** — driving, entry/exit, winches | `CVehicleData`, `CVehicleWinchController`, `CEntryVehicleLockedForPlayerCondition`, `ACT_*_DOOR_*`, `ACT_ARMORED_HIJACK_EJECT` |
| [weapons.md](weapons.md) | **Weapons & combat** | `CWeaponWieldedInLeftHand`, `CWeaponSetter`, `CWeaponRemover`, `CWeaponUIInfo`, `CWeaponPlacementRulePersistentManager`, `ACT_AIM_*`, `ACT_RELOAD*` |
| [destruction.md](destruction.md) | **Havok destruction / chaos objects** | `CHavokDestructionSystem`, `CHavokDestructionGraphicsManager`, `CHavokDestructionInstancedGraphicsManager`, `CForcePulse` |
| [behavior_system.md](behavior_system.md) | **Behavior/condition VM** — the gameplay-logic layer driving all of the above | `CConditional_*` (AND/IsDLCUnlocked/IsMissionCompleted/…), the `ACT_*` action set, `CGraph*Condition` (animation-graph transitions) |
| [missions_progression.md](missions_progression.md) | **Missions, activities, supply, progression, DLC** | `CQuestManager`, `CObjectiveContentManager`, `CActivityManager`, `CSupplyManager`, `CCollectionManager`, `CStatisticManager`, `CDownloadableContentManager`, `CNotificationManager` |

Supporting managers seen in the inventory that individual docs should reference as they intersect:
`CPlayer`/`CPlayerManager`, `CCharacter`/`CCreatureManager`, `CModelInstanceManager`, `CEffectSystem`,
`CSoundSystem`/`CVocalsLayer`, `CDialogueChain`/`CDialogueCoordinator`, `CCutsceneManager`, `CHUDUI`/`COutline`.

## Doc template (each system doc follows this)

```markdown
# <System> — <one-line what-it-is>

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

## Overview
What the system does, in engine terms. 3–6 sentences.

## Key classes & functions
Table: class/name string → the FUN_<addr> that registers/implements it → one-line role.

## How it works (from the decomp)
The actual mechanism: state, data flow, the math/constants, the update loop. Cite FUN_ addresses inline.

## Data & config integration
How it's driven by RTPC entity components / ADF config; map class name → entity component (hash if known).

## Notable constants / tunables
Magic numbers, thresholds, force values read straight from the decomp (with the FUN_ they live in).

## Call-graph highlights
The important caller/callee edges.

## Open questions / lower-confidence
What's inferred or unresolved, so the next pass knows where to dig.

## Appendix — decomp anchors
The exact strings + FUN_/DAT_ addresses used, so any claim is re-verifiable.
```

## Cross-cutting architecture (what the survey established)

Documenting the eight systems independently surfaced one **shared engine spine** that all of them ride on —
this is the most important structural finding of the first pass:

- **One name→type registry, keyed by the hash we already cracked.** Gameplay classes, conditions, actions,
  components and content are all instantiated *by string name* through a single global factory
  (`FUN_140f27f60`, a thin thunk over the reflection lookup) with **6,138 call sites**. The behavior-system
  dig proved the key is our own **lookup3 `hashlittle`** (`FUN_14aadeee0`) — the *same* hash used for asset
  paths and TAB/ADF names (`docs/formats/name_hash.md`, memory `[[name-hash-cracked]]`). So the gameplay-code
  namespace and the asset namespace share one hashing identity. (proven)
- **Three registrars install the world.** `FUN_14085fd00` registers the reflected **component** family
  (vehicles, weapons, …); `FUN_148f960c0` registers the singleton **managers** (progression, wind/weather, …);
  `FUN_148f99a90` / `FUN_1485b9730` register the two **condition** registries. Every system's classes appear
  as rows in one of these tables.
- **Behavior graph is the universal driver.** Abilities are gated by `CConditional_*` / `C*Condition`
  predicates and dispatched as `ACT_*` **hash IDs** (queued via `QueueAct`, hash-matched not string-matched).
  Grapple, wingsuit, vehicle entry, weapon fire and mission triggers are all edges in this graph — see
  `behavior_system.md` for the machinery the other seven docs reference.
- **Tunables live in data, not code.** A recurring result: the executable holds the *mechanism* (state
  machines, force-channel names, integrators) while the *magnitudes* (forces, damage, ammo, timeouts) live in
  RTPC/ADF/Havok-tagfile data. Force application is consistently done through **named force channels**
  (`GrapplingHook`, `ReelInKick`, `CForcePulse`, the ~120-entry vehicle driving registry) whose values are
  data-side. This is the natural bridge to the next pass.

## Status

Survey + all eight per-system deep dives written 2026-08-16 by parallel decomp-mining agents, every claim
graded and `FUN_`-cited. The corpus MCP server is currently pointed at the Mercs2 project (not indexed for
JC4) — these docs work **directly against the decomp text dump** above.

**Next pass (data-side):** the open questions across the docs converge on the same thing — resolve the
force/damage/tunable *values* by decoding the RTPC entity components and ADF configs the code names (tie each
`CConditional_*` / component class here to its entity component hash in `[[rtpc-entity-assembly]]`).

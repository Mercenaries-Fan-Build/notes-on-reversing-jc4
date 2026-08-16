# Physics & Constraints — the Havok `hknp` rigid-body / constraint layer JC4 rides on

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

> Scope: the general rigid-body / constraint / query layer and the **external-force-generator bus**
> (the common channel grapple tethers, winches, wind and explosions all feed). **Destruction** (`hknd`,
> `CHavokDestructionSystem`, chaos objects, fracture) is a separate system — see
> [destruction.md](destruction.md); this doc references it but does not duplicate it. Grapple/winch
> force *sources* live in [grappling_hook.md](grappling_hook.md); here we document the force *plumbing*
> they plug into.

## Overview

JC4's physics is **Havok Physics "next-gen" (`hknp`)**, middleware version
**`havok_physics-2016.1.0.4.0.4.1614385`**, paired with **`havok_animation-2016.1.0.3.0.0.1470535`**
(the ragdoll/physics bridge) and **Havok Destruction (`hknd`)**. The exact vendor paths survive as string
constants in the decomp, e.g.
`...\havok_physics-2016.1.0.4.0.4.1614385\source\physics\physics\dynamics\world\hknpworld.cpp` and
`...\hknpmultithreadedsimulation.cpp` (proven — string table). This matches memory
[[havok-hct-2018]] (structures = Havok TAG0 2016.1) — the whole physics stack is one 2016.1 Havok drop.

The engine wraps Havok in a small set of **reflected engine systems** — `CPhysicsSystem`,
`CPhysicalRestraintsSystem`, `CConstraintFactory`, `CPfxBodyPropsSystem`, `CPfxWindPhysicsSystem` — plus a
`CTransformComponent` that bridges a Havok body's motion back to the entity's world transform. The world is
stepped by a **multithreaded task graph** (`hknpMultithreadedSimulation`): each stage is a task whose
profiler token survives as a `"Tt<Name>"` string, so the *entire step pipeline is legible by name* even
though the task callbacks themselves show `callers=[]` (they are installed as function pointers in Havok
task descriptors, not called directly — see the methodology caveat). The game injects its own logic through
**Avalanche custom Havok modifier callbacks** (`ava_CPhysicsContactModifier_*`, `ava_CPfxContactClipper_*`,
`ava_CPfxCustomConstraintMassModifier_*`) — the true integration seam. (proven)

**What is walled (route: Havok libs + tagfile data, memory [[havok-hct-2018]]):** the solver internals,
integrator, gravity magnitude, constraint/motor stiffness, contact/friction/restitution coefficients and
constraint-break force thresholds live in the linked Havok libraries and in data (`hknpWorldCinfo`,
material/body-props tagfiles, RTPC). The decomp gives the *mechanism and the wiring*, not most magnitudes.

## Key classes & functions

| Name string | Registered / implemented at | Role |
|---|---|---|
| `CPhysicsSystem` | `FUN_148f958e0` (mgr table; desc `0x141cb1618`, vtable `PTR_LAB_141d90408`) | Top-level physics world manager (owns the `hknpWorld`). |
| `CPhysicalRestraintsSystem` | `FUN_148f958e0` (desc `0x141d8ffe8`, vtable `PTR_LAB_141d903e8`) | Manages physical restraints (attach/pin bodies; ragdoll restraints). |
| `CPfxBodyPropsSystem` | `FUN_148f958e0` (string-keyed entry, vtable `PTR_LAB_141d90428`) | Per-body physics-properties system (mass/material/damping props). |
| `CPfxWindPhysicsSystem` | `FUN_148f958e0` (desc `0x141cb1660`, vtable `PTR_LAB_141d90428`) | Wind→physics coupling (feeds the force bus; see wind_and_weather.md). |
| `CConstraintFactory` | mgr table `FUN_1400...` at line ~3209377 (desc `0x141cbe0c8`, vtable `PTR_LAB_141d906a8`) | Builds Havok constraints (joints, ragdoll chains) from `ConstraintData`. |
| `CTransformComponent` | reflected via `FUN_140f27f60("CTransformComponent",0x13,…)` in `FUN_1407dc170` | Entity component bridging physics motion ↔ entity world transform. |
| `CRigidObject` / `CRagdollObject` / `CRagdollBoneProxyObject` / `CRagdollAttachment` | string table (component classes) | Rigid body wrapper; ragdoll instance; per-bone proxy; ragdoll pin. |
| Force-collect task | `FUN_1416d3380` (`TtCollectExternalForceGenerators`) | Gathers per-body external force generators into the solver. |
| Force-merge task | `FUN_1416d3c50` (`TtMergeForceGenerators`) | Merges/sorts the per-island force-generator lists. |
| Force-apply task | `FUN_1416d43d0` (`TtApplyExternalForces`) | Applies the merged force generators to bodies (`thunk_FUN_14ceb2f60`). |
| Constraint-Jacobian task | `FUN_141193ab0` (`TtBuildConstraintJacobians`) | Builds solver Jacobians for active constraints. |
| Contact modifier | `FUN_1400bc050` (`ava_CPhysicsContactModifier_manifoldProcessCallback`) | Per-manifold game callback into the narrowphase. |
| Contact Jacobian hooks | `FUN_1400bdf60`/`FUN_1400bee70` (`..._postContactJacobianReused/Setup`, `..._preConstraintJacobianSetup`) | Game hooks into contact/constraint Jacobian setup. |
| Pfx contact destruction / clipper | `FUN_1410ea6f0` / `FUN_1410ed650` | Contact-driven destruction + impulse clipping (bridges to destruction.md). |
| Constraint-force event | `FUN_14b665f90` (`ConstraintForceExceededEvent`) | Fires when a constraint's force exceeds its (data-side) break threshold. |
| Reflection factory | `FUN_140f27f60` | Global name→type registry (6,138 call sites; key = lookup3, [[name-hash-cracked]]). |

## How it works (from the decomp)

### The step is a named task graph (`hknpMultithreadedSimulation`)
The physics update is a Havok multithreaded task graph. Every stage brackets itself with a profiler push
of a `"Tt<Name>"` token via `FUN_1411c8670(timeline, "<Name>")` / `FUN_1411c8680(timeline)` (open/close),
timestamped with `rdtsc()`. Harvesting these tokens reconstructs the **entire per-frame physics pipeline**
(proven — string constants). The mainline ordering (Havok-canonical, corroborated by the token set):

1. **External forces** — `TtCollectExternalForceGenerators` → `TtMergeForceGenerators` →
   `TtApplyExternalForces`, alongside `TtApplyActions`, `TtApplyVehicleForces`, `TtSimulateVehicle`.
2. **Broadphase** — `TtBroadPhase` (`hknpWideBroadPhase`), `TtAddActiveBodyPairs`, `TtAppendForcedBodyPairs`.
3. **Narrowphase / collide** — `TtCollide`, `TtCollideTrees`, `TtCompoundShapeManifoldTask`,
   `TtGatherManifoldIds`, `TtCopyContactCaches`, plus the `ava_CPhysicsContactModifier_*` callbacks.
4. **Solver setup** — `TtCalcMassProps`, `TtCreateSolverBodies`, `TtCreateSolverConnections`,
   `TtBuildConstraintJacobians` / `TtCreate Inplace Jacobian` / `TtJacobians`, `TtBuildSolveTasks`.
5. **Solve** — `TtPreSolve` → `TtSolve` / `TtSolveConstraints` → `TtPostSolve`, then integration
   `TtSubIntegrate` / `TtSubIntegrateLast`, `TtCalcVelocities Task`, `TtGatherSolverVelocities`.
6. **Post** — `TtMotionWelding`, `TtMotionExt`, `TtDeactivation` / `TtInactiveBodyMaintenence`,
   `TtDeleteSimulationContext`.

The three **force-generator tasks are the common bus** the prompt calls out:

- **`FUN_1416d3380` (Collect).** Walks the active-body / active-connection sets of the current simulation
  island (bodies at `lVar2+0xf8`, connections at `lVar2+0x368`) and, for each body that carries a
  force-generator reference and is not a special/immovable type (`(flags & 0xf000000)==0`), appends a
  16-byte record `{ bodyIndex, kind(0=body/1=connection), seq }` into the island's generator buffer at
  `+0x4f8` (count at `+0x500`, capacity at `+0x504 & 0x3fffffff`, grown via `thunk_FUN_14b932030`). The
  list is then sorted (`thunk_FUN_14ceb2300(..., &LAB_1416d3c10)`). (proven — `FUN_1416d3380`)
- **`FUN_1416d3c50` (Merge).** Reads a per-generator count (`lVar3+0x278`), allocates a scratch pointer
  array from a frame allocator (`DAT_141c8f550(DAT_142ce7a38)`), fills it with pointers into each island's
  `+0x4f8` generator block (stride `0x540`), and merge-sorts it (`thunk_FUN_14ceb2940(..., &LAB_1416d3c10)`)
  into `lVar3+0x2e0` (merged count at `+0x2e8`). This is the "single global ordered force list" step. (proven)
- **`FUN_1416d43d0` (Apply).** Partitions the merged list across worker lanes
  (`param_1+0x20`=lane index, `+0x24`=lane count), groups runs that share a target, and for each run calls
  `thunk_FUN_14ceb2f60(world, generator, records, count, &accumulator)`. The accumulator `local_48` is
  initialised to a 4-lane broadcast of `DAT_141ca9c6c` (**inferred `0.0f`** — the same constant is used
  throughout as a `max(x,0)` clamp floor and as a zero multiplier, e.g. lines 37496, 43087). This is where
  a generator's contribution is turned into an impulse on the body. (proven — mechanism; inferred — the 0.0f)

Because the task callbacks are installed as pointers in Havok task descriptors, `FUN_1416d3380/…3c50/…43d0`
all show `callers=[]` — expected, not missing (methodology caveat).

### The external-force bus is what grapple / wind / explosion feed
Force *sources* register generators that this bus collects/merges/applies:
- **Grapple / reel / winch:** `ApplyImpulseOverTimeAction`, `ReelInKick` / `ReelInKickTask`,
  `WinchSuckerDamping`, `Vehicle Seat Winch` (strings). These are hkpAction-style generators applied under
  `TtApplyActions` / `TtApplyExternalForces` — see grappling_hook.md for the source side. (proven — strings)
- **Vehicles:** `TtApplyVehicleForces` / `TtSimulateVehicle` (engine/wheel forces; vehicles.md). (proven)
- **Wind:** `CPfxWindPhysicsSystem` (registered in `FUN_148f958e0`) couples wind into the bus
  (wind_and_weather.md). (proven — registration)
- **Explosions / contact destruction:** `ava_CPfxContactDestructionModifier` (`FUN_1410ea6f0`) and the
  impulse clipper `ava_CPfxContactClipper` (`FUN_1410ed650`); destruction.md. (proven)

The engine holds the *mechanism* (named channels + the collect/merge/apply integrator); the *magnitudes*
(reel force, kick impulse, wind strength) are data-side — consistent with the README's cross-cutting finding.

### The contact seam — Avalanche custom modifiers
`FUN_1400bc050` (`ava_CPhysicsContactModifier_manifoldProcessCallback`) is invoked per manifold with the two
bodies. It resolves each body's game object (`thunk_FUN_147677250(*body + 0x70)`) and, if that object opts in
(`flags byte & 8`), dispatches a virtual callback at **vtable offset `0xd0`**
`(**(code**)(*obj + 0xd0))(obj, …, otherObj, isA)` for both bodies. This is how gameplay code intercepts
contacts (material response, damage, one-way collision, trigger logic) without patching Havok. Companion
hooks `FUN_1400bdf60`/`FUN_1400bee70` run at `postContactJacobianReused` / `postContactJacobianSetup` /
`preConstraintJacobianSetup` — i.e. the game can also edit the solver's contact/constraint Jacobians and
constraint mass before the solve. (proven — `FUN_1400bc050`, siblings)

### Constraints, joints & ragdoll
`CConstraintFactory` builds Havok constraints from `ConstraintData`. Constraint building/solving stages:
`SetupConstraints` → `PrepareConstraints` / `PrepareActivatedConstraints` (`StPrepareActivatedConstraints`)
→ `TtBuildConstraintJacobians` (`FUN_141193ab0`) → `TtSolveConstraints` → `enforceConstraints`
(`StenforceConstraints`). (proven — strings)

**Ragdoll** rides `hknpragdollutils.cpp` from the *animation* module (physics↔animation bridge) plus the
constraint chain. The build path validates the joint hierarchy and emits named errors, which document its
rules (all proven — string constants):
- `"Cyclic constraint graph detected in ragdoll. Aborting ragdoll build."`
- `"Invalid parent body id in constraint. Cannot use world-attached body in a ragdoll."`
- `"Invalid child body id in constraint."` / `"Empty constraint hierarchy in physics system."`
- `"Pivot of child rigid body (A) is expected to be aligned with the constraint at setup time."`
- `"Attempted to constrain a body to itself. Skipping…"`, `"…unsupported constraint type…"`,
  `"Cannot convert constraint data to a powered constraint."`, `"This type of constraint does not have motors"`.

Ragdoll gameplay wrappers: `CRagdollObject`, `CRagdollBoneProxyObject` (per-bone collision proxy),
`CRagdollAttachment`, and a family of behavior conditions (`CRagdollContactCondition`,
`CRagdollVelocityCondition`, `CRagdollIsSettledCondition`, `CAirborneHookedRagdollCondition`,
`CRagdollBlendingCondition`) plus `ACT_*` ragdoll actions (`ACT_FULL_RAGDOLL`, `ACT_GRAPPLE_RAGDOLL`,
`ACT_HITREACT_RAGDOLL`, `ACT_RAGDOLL_IMPACT`, `ACT_RAGDOLL_INAIR_STABILIZATION_*`). These are behavior-graph
edges (behavior_system.md) that toggle a character between keyframed and simulated ragdoll. (proven — strings)

**Constraint force events:** `ConstraintForceEvent` / `ConstraintForceExceededEvent` (`FUN_14b665f90`) report
when a constraint's solved force crosses a threshold — the mechanism for breakable joints. The **threshold
value is walled** (data-side, `ConstraintData` / tagfile). (proven — event exists; walled — magnitude)

### Queries (raycast / shapecast)
Queries are their own task family: `TtCastRay` / `TtWorldCastRay` / `TtCompoundCastRay` /
`TtConvexShapeCastRay(GSK)`, `TtStaticTree::castRay`, `TtDynMediator::castRay`, `TtCastShape` /
`TtCompoundCastShape`, `TtCompoundGetClosestPoints`, `TtCollision Query`, `TtBuild query mask`. Ray/shape
tolerance is `hkcdRayCastTriangle::g_tolerance` (a Havok global; the warning string notes it isn't stored in
`hknpWorldCinfo` snapshots). Gameplay query wrappers: `CRaycastObject`, and the fan-cast conditions
`CRayCastFanCondition` / `CForwardRayCastFanCondition`. Which layers a query hits is governed by the
collision-filter table below. (proven — strings)

### Collision filter — the layer matrix
`FUN_1400cf270` (size 5313, called by `FUN_148e825d0`) prints the **full collision-filter group table** — a
30-layer group filter (`hknpConstraintCollisionFilter` / `setBodyCollisionFilterInfo`). The complete layer
set (index → name, proven from the printed matrix rows):

```
 0 FULL_COLLISION            10 CHARACTER_ATTACHED_DYNAMIC_RB  20 RAYCAST_DEFAULT
 1 NO_COLLISION              11 CHARACTER_RAGDOLL_PROXIES      21 RAYCAST_EVERYTHING_PHYSICAL
 2 STATIC                    12 PARTICLE                       22 RAYCAST_WHEELS
 3 TERRAIN                   13 EXPLOSIVE                      23 SHAPEQUERY_DEFAULT
 4 DYNAMIC                   14 PROJECTILE                     24 SHAPEQUERY_TRIGGER
 5 DESTRUCTION               15 WEAPONS                        25 SHAPEQUERY_EXPLOSION
 6 CHARACTER_COLLISION       16 VEHICLE_COLLISION_WHEEL        26 SHAPEQUERY_VEHICLES
 7 CHARACTER_GHOST_MODE      17 VEHICLE_COLLISION_DYNAMIC_ONLY 27 STATIC_AND_DESTRUCTION
 8 CHARACTER_RAGDOLL         18 VEHICLE_COLLISION              28 ALL_DYNAMIC_OBJECTS
 9 CHARACTER_RAGDOLL_KEYFRAMED 19 VEHICLE_RAYCAST              29 LISTENER_EFFECTS
```
The printed matrix also encodes, per layer, which other layers it collides with (the `#`/`.` grid), so the
full collision policy is recoverable from `FUN_1400cf270` (an editor/debug dump; `CCollisionFilterEditor`).
(proven — `FUN_1400cf270`)

### The transform bridge (`CTransformComponent`)
`CTransformComponent` is registered as a reflected entity component
(`FUN_140f27f60("CTransformComponent", 0x13, 0, …)` cached in `FUN_1407dc170`; also referenced at
`FUN_...4140717`). It is the component that carries an entity's world transform; a physicalized entity's
Havok body motion is written back through it each step (the entity↔physics coupling). The precise
write-back function was not isolated in this pass (see open questions). (proven — registration; inferred — role)

## Data & config integration

- **Systems are reflected singletons.** `CPhysicsSystem`, `CPhysicalRestraintsSystem`, `CPfxBodyPropsSystem`,
  `CPfxWindPhysicsSystem` are installed by the **manager registrar `FUN_148f960c0`** via its table builder
  `FUN_148f958e0` (each row: a 16-byte `{descriptorPtr, 1, vtablePtr}` entry). `CConstraintFactory` and
  `CTransformComponent` register in the same family (`FUN_140f27f60` reflection). This is the same
  three-registrar spine the README documents; keys are lookup3 `hashlittle` ([[name-hash-cracked]]). (proven)
- **Bodies/materials are data.** `CPfxBodyPropsSystem` implies per-body physics properties (mass, friction,
  restitution, damping) are authored data attached to models/entities, not code constants — resolve via RTPC
  entity components and the model/`hknpWorldCinfo` tagfiles ([[rtpc-entity-assembly]], [[composite-assets]]).
- **Constraints/ragdoll are data.** Joint chains, limits, motors and break thresholds come from
  `ConstraintData` in Havok tagfiles (structures = TAG0 2016.1, [[havok-hct-2018]]). `CConstraintFactory`
  is only the *builder*. (inferred)
- **Component class ↔ entity hash.** `CRigidObject`, `CRagdollObject`, `CPhysicalizationWeaponComponent`,
  `CTransformComponent` each map to an entity-component class hash on the data side — next-pass linkage.

## Notable constants / tunables (present in code)

| Value | Where | Meaning |
|---|---|---|
| `DAT_141ca9c6c` (**inferred `0.0f`**) | `FUN_1416d43d0` (force accumulator init, 4-lane) + ~dozens of `max(x,0)` clamp sites | Zero constant used to seed the applied-force accumulator and as clamp floor. |
| Generator record = 16 B `{bodyIdx:4, kind:2, pad:2/seq}` | `FUN_1416d3380` | Layout of an external-force-generator entry; buffer `+0x4f8`, count `+0x500`, cap `+0x504 & 0x3fffffff`. |
| Island generator-block stride `0x540` | `FUN_1416d3c50` | Per-island simulation-context stride when gathering generator lists. |
| Contact-modifier vtable slot `0xd0` | `FUN_1400bc050` | Virtual called on a body's game object during manifold processing. |
| Body flag mask `0xf000000` (skip) | `FUN_1416d3380` | Bodies with these type bits are excluded from force generation. |
| Opt-in flag `byte & 8` | `FUN_1400bc050` | Body's game object requests the contact callback. |
| 30 collision-filter layers (table above) | `FUN_1400cf270` | Full layer set + collide matrix. |
| Havok versions `hknp 2016.1.0.4`, `hka 2016.1.0.3`, `hknd` | string table | Middleware lineage (matches [[havok-hct-2018]]). |

> **Walled magnitudes** (route → Havok libs / tagfiles): gravity vector, solver iteration counts, contact
> friction/restitution, constraint stiffness & motor gains, constraint-break force threshold, deactivation
> thresholds, `hkcdRayCastTriangle::g_tolerance`. Present in `hknpWorldCinfo` / material / `ConstraintData`
> data, not in the functions export.

## Call-graph highlights

- **Manager install:** `FUN_148f960c0` → `FUN_148f958e0` (builds the system table incl. `CPhysicsSystem`,
  `CPhysicalRestraintsSystem`, `CPfxBodyPropsSystem`, `CPfxWindPhysicsSystem`). (proven)
- **Reflection:** `FUN_1407dc170` → `FUN_140f27f60("CTransformComponent",0x13,…)` (component type lookup,
  cached in `DAT_142cb8ee0`). (proven)
- **Force bus (per step):** `TtCollectExternalForceGenerators` `FUN_1416d3380` →
  `TtMergeForceGenerators` `FUN_1416d3c50` → `TtApplyExternalForces` `FUN_1416d43d0`
  → `thunk_FUN_14ceb2f60` (apply). Peers: `TtApplyActions`, `TtApplyVehicleForces`. (proven)
- **Contact seam:** narrowphase → `FUN_1400bc050` (`manifoldProcessCallback`) → body vtable `+0xd0`;
  `FUN_1400bdf60`/`FUN_1400bee70` at Jacobian setup. (proven)
- **Constraint solve:** `SetupConstraints`/`PrepareActivatedConstraints` → `FUN_141193ab0`
  (`TtBuildConstraintJacobians`) → `TtSolveConstraints`; break → `FUN_14b665f90`
  (`ConstraintForceExceededEvent`). (proven)
- **Collision filter dump:** `FUN_148e825d0` → `FUN_1400cf270` (layer table). (proven)

## Open questions / lower-confidence

- **World step orchestrator not pinned.** The individual task callbacks are named, but the top-level
  `CPhysicsSystem::Update` / `hknpWorld::step` entry that assembles the graph each frame was not isolated
  (it lives behind the Havok `hknpMultithreadedSimulation` scheduler). Route: find the caller that builds
  the task descriptors pointing at `FUN_1416d3380`/`…43d0` (data-section descriptor tables). (open)
- **Transform write-back function.** `CTransformComponent`'s exact per-step physics→entity motion copy was
  not located; only its registration/role is proven. (inferred)
- **`DAT_141ca9c6c` = 0.0f** is inferred from usage, not read from the data section (functions-only export).
- **All solver magnitudes / break thresholds** are walled (Havok data) — the recurring project result.
- **`CPhysicalRestraintsSystem` vs `CRagdollAttachment` division of labour** is inferred from names; the
  restraint apply path was not read in full. (inferred)

## Appendix — decomp anchors

**Registration**
- `FUN_148f958e0` — system table: `"CPhysicalRestraintsSystem"` (`0x141d8ffe8`/`PTR_LAB_141d903e8`),
  `"CPhysicsSystem"` (`0x141cb1618`/`PTR_LAB_141d90408`), `"CPfxWindPhysicsSystem"` (`0x141cb1660`),
  `"CPfxBodyPropsSystem"` (`PTR_LAB_141d90428`), `"CWinchControllerSystem"`, `"CHavokDestructionDecalManager"`.
- `FUN_148f960c0` — manager registrar (caller of the table builder).
- mgr table @line ~3209377 — `"CConstraintFactory"` (`0x141cbe0c8`/`PTR_LAB_141d906a8`).
- `FUN_1407dc170` — `FUN_140f27f60("CTransformComponent",0x13,0,…)` → `DAT_142cb8ee0`.

**Force bus / step tasks**
- `FUN_1416d3380` `"TtCollectExternalForceGenerators"` · `FUN_1416d3c50` `"TtMergeForceGenerators"` ·
  `FUN_1416d43d0` `"TtApplyExternalForces"` · `FUN_141193ab0` `"TtBuildConstraintJacobians"`.
- Step token family (all `"Tt…"` strings): `ApplyActions`, `ApplyVehicleForces`, `SimulateVehicle`,
  `BroadPhase`, `Collide`, `CollideTrees`, `CalcMassProps`, `CreateSolverBodies`, `CreateSolverConnections`,
  `Solve`, `SolveConstraints`, `PreSolve`/`PostSolve`, `SubIntegrate`/`SubIntegrateLast`, `MotionWelding`,
  `Deactivation`, `CastRay`/`WorldCastRay`/`CompoundCastRay`/`CastShape`, `Character Proxy`, `Cloth`.

**Contact / constraint hooks**
- `FUN_1400bc050` `"ava_CPhysicsContactModifier_manifoldProcessCallback"` (vtable `+0xd0`).
- `FUN_1400bdf60` / `FUN_1400bee70` — `postContactJacobianReused/Setup`, `preConstraintJacobianSetup`.
- `FUN_1410ea6f0` `"ava_CPfxContactDestructionModifier_postContactJacobianSetup"` ·
  `FUN_1410ed650` `"ava_CPfxContactClipper_manifoldProcessCallback"` / `"…_postContactImpulseClipped"`.
- `FUN_14b665f90` `"ConstraintForceExceededEvent"`.

**Havok symbol strings (lineage / subsystems)**
- Physics: `hknpWorldCinfo`, `hknpWorldEx`, `hknpBroadPhase`, `hknpWideBroadPhase`, `hknpBodyManager`,
  `hknpConstraintManager`, `hknpMotionManager`, `hknpModifierManager`, `hknpMultithreadedSimulation`,
  `hknpCharacterRigidBody`, `hknpConstraintCollisionFilter`, `hknpContactImpulseEvent`,
  `hknpConvexConvexManifoldGenerator`, `hknpCompressedMeshShape`, `hknpHeightFieldShape`,
  `hknpVehicleLinearCastWheelCollide`, `hkcdRayCastTriangle`.
- Animation bridge: `hknpragdollutils.cpp`. Destruction: `hkndDestructionSystem`, `hkndDefaultController`,
  `hkndFlexibleJointRuntime`, `hkndExplosionRuntime` (→ destruction.md).
- Version paths: `havok_physics-2016.1.0.4.0.4.1614385`, `havok_animation-2016.1.0.3.0.0.1470535`.

**Ragdoll / query gameplay classes & validators**
- `CRigidObject`, `CRagdollObject`, `CRagdollBoneProxyObject`, `CRagdollAttachment`, `CRaycastObject`,
  `CRayCastFanCondition`, `CForwardRayCastFanCondition`, `CPhysicalizationWeaponComponent`,
  `CPhysicallyRestrainedBehaviour`, ragdoll `CConditional_*` / `ACT_*_RAGDOLL` set.
- Validators: `"Cyclic constraint graph detected in ragdoll…"`, `"Invalid parent body id in constraint. Cannot
  use world-attached body in a ragdoll."`, `"Pivot of child rigid body (A) is expected to be aligned…"`.

**Collision filter**
- `FUN_1400cf270` (called by `FUN_148e825d0`) — 30-layer table + collide matrix; `CCollisionFilterEditor`,
  `setBodyCollisionFilterInfo`.

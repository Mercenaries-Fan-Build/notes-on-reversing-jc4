# Havok Destruction & Chaos Physics — JC4's destructible-everything system

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

## Overview

JC4's destruction is a game-side wrapper (`CHavokDestruction*` classes) around Havok's **NDivision
Destruction runtime** — every runtime symbol carries the `hknd` prefix (`hkndWorld`,
`hkndDestructionSystem`, `hkndExplosionRuntime`, `hkndBreakableBodyResetUtil`, `hkndDebrisFracture`,
`hkndBodyTimeoutRuntime`, …), so the data/library lineage is Havok Destruction SDK 2016.x — the same
generation as the `hknp` physics tagfiles documented in `[[havok-hct-2018]]`. (proven —
`hknd*`/`hkndWorld` string inventory, FUN_141670d10 @0x141670d10, FUN_14769ff10 @0x14769ff10)

The engine-side owner is a single **`CHavokDestructionSystem`** object built at world/level init
(FUN_14009e600 @0x14009e600). It aggregates a set of sub-managers that split responsibilities:
a Havok destruction *world* wrapper (`hkndWorld`), a **graphics manager** and an **instanced graphics
manager** (which swap intact→fractured geometry and draw thousands of debris pieces via GPU
instancing), and a **decal manager** (burn/scorch decals on damaged surfaces). Destruction is driven
by Havok "runtime" listeners registered on the `hkndWorld`: an **explosion runtime** applies radial
break/impulse, a **body-timeout runtime** despawns settled debris, plus reset/decal/joint runtimes.
(proven — constructor FUN_14009e600; runtime ctors FUN_1416b1ef0 @0x1416b1ef0,
FUN_1416af1b0 @0x1416af1b0)

The whole thing is separately exposed to gameplay as RTPC entity components — `CHavokDestructionObject`
(a destructible instance) and force applicators `CForcePulse`/`CForceField`/`CForcePoint` — registered
by name into the reflection type system. (proven — FUN_140a8f7c0 @0x140a8f7c0,
FUN_14080c5b0 @0x14080c5b0)

## Key classes & functions

| Class / name string | FUN_ (role) | One-line role |
|---|---|---|
| `CHavokDestructionSystem` | FUN_14009e600 @0x14009e600 (ctor) | Top-level owner; builds all sub-managers, stores singleton pointers in `DAT_142c84be0`/`DAT_142c84b98` |
| `hkndWorld` wrapper | FUN_141670d10 @0x141670d10 (ctor) | Game-side wrapper around the Havok destruction world; registers 6 world event listeners |
| `CHavokDestructionGraphicsManager` | built at FUN_14009e600+0x50 alloc (vtable `PTR_LAB_141cad500`/`_141cad520`) | Non-instanced graphics: swaps a body's render/collision shape when it fractures |
| `CHavokDestructionInstancedGraphicsManager` | FUN_14168a050 @0x14168a050 (ctor, 0x7ed0 bytes, vtable `PTR_LAB_1423fd5b0`) | Pooled/instanced debris rendering; preallocates 10k+ instance slots |
| `CHavokDestructionDecalManager` | FUN_1416880e0 @0x1416880e0 | Damage/scorch decals (`hkndDecalMapRuntime`) |
| `hkndExplosionRuntime` | FUN_1416b1ef0 @0x1416b1ef0 (ctor), FUN_1416b21e0 @0x1416b21e0 (apply) | Explosion → fracture connectivity walk + piece transforms + break flags |
| `hkndBodyTimeoutRuntime` | FUN_1416af1b0 @0x1416af1b0 (ctor), FUN_1416af390 (tick) | Debris lifetime / despawn |
| `hkndBreakableBodyResetUtil` | FUN_14769ff10 @0x14769ff10 | Tears down / resets breakable bodies (level unload / reset) |
| graphics-mgr per-frame add/remove | FUN_141674640 @0x141674640, FUN_1416743b0 @0x1416743b0 | "UpdateBodyShape" on newly-fractured bodies; release graphics for removed pieces |
| body↔graphics registration | FUN_141672cd0 @0x141672cd0 | Registers a breakable body with the graphics manager (indexed table) |
| `CHavokDestructionObject` | type reg FUN_140a8f7c0 @0x140a8f7c0 (`DAT_142cb91f0`) | RTPC entity component = a destructible object instance |
| `CForcePulse` | component ctor FUN_140a754d0 @0x140a754d0; factory FUN_14080c5b0 @0x14080c5b0 (`DAT_142cb91e0`) | RTPC entity component: radial force/impulse pulse |
| `CForceField` / `CForcePoint` | FUN_140a8f610 / FUN_140a8f6a0 (type regs) | Sibling force applicators |

System name→hash (from the engine's system-dependency registry, FUN_148f958e0 @0x148f958e0 /
FUN_148f960c0 @0x148f960c0): `CHavokDestructionDecalManager` = `0x41cad740`,
`CHavokDestructionInstancedGraphicsManager` = `0x41cad888`. (proven — literal stores in those funcs)

## How it works (from the decomp)

### 1. System construction (FUN_14009e600)

Called once from the world/level init path (its only caller FUN_1400cf270 @0x1400cf270, a 5313-byte
manager-assembly routine). The constructor allocates, in order, into the object at `param_1[3..8]`:
(proven — FUN_14009e600 body)

- `param_1[3]` — the **`hkndWorld` wrapper** (0x390 bytes, vtable `PTR_LAB_141cada00`), via
  FUN_141670d10. Immediately after, it registers seven **callback thunks** tagged
  `"CHavokDestructionSystem"` onto the world (offsets +0x88…+0xc8 and the parent +0x868/+0x928) — these
  are the system's own event handlers (LAB_1400bc240, LAB_1400bc360, FUN_1400bc9b0, LAB_1400bc4d0,
  thunk_FUN_1476be2b0, …). (proven — lines around FUN_14009e600 @+0x928/+0x868/+0x98..+0xc8)
- `param_1[4]` — the **graphics manager** (0x50 bytes, vtables `PTR_LAB_141cad500` / `PTR_LAB_141cad520`),
  with four callbacks tagged `"CHavokDestructionGraphicsManager"` (LAB_1400bc220, LAB_1400bc5a0,
  LAB_1400bc950, LAB_1400bc400) wired to `param_1[3]` (the world). It is also published into the
  singleton holder: `*(DAT_142c84be0+0x18) = world`, `*(DAT_142c84be0+0x28) = graphicsMgr`.
  (proven — FUN_14009e600 +0x50 alloc block)
- decal manager pointer at `lVar8+0x20`, plus a `"CHavokDestructionDecalManager"` callback
  (FUN_1400bc5c0) — via FUN_1416880e0. (proven)
- `param_1[5]` — decal sub-object (0x28 bytes) via FUN_1416880e0; `param_1[6]` — 0x58-byte object via
  FUN_14166bf40. (proven)
- `param_1[7]` — the **instanced graphics manager** (0x7ed0 bytes ≈ 32 KB, vtable `PTR_LAB_141cad768`),
  via FUN_14168a050. This is the big pooled debris renderer. (proven — 0x7ed0 alloc + FUN_14168a050 call)
- `param_1[8]` — an 8-byte helper (vtable `PTR_FUN_141cad9c0`).

Two `int` tunables are written into the world-init parameter block early: `local_40 = 5000`,
`local_3c = 10000` (inferred: initial reserve/pool sizes passed to FUN_141670d10). Finally
FUN_1410e83d0 registers the system into the global per-frame update list (a generic
"register-for-update" vector at `DAT_142ce3ec0`). (proven — FUN_14009e600 tail; FUN_1410e83d0 @0x1410e83d0)

### 2. The `hkndWorld` wrapper and its listeners (FUN_141670d10)

FUN_141670d10 builds the game-side `hkndWorld` (vtable `PTR_LAB_1423fbb68`) and registers **six event
listeners** onto the underlying Havok world object (`param_1[0xd]`), all tagged `"hkndWorld"`, at world
offsets +0x848, +0x868, +0x908, +0x910, +0x920, +0x938 → handlers FUN_1416743b0, LAB_141673e20,
LAB_1416741e0, LAB_141674560, LAB_1416745f0, LAB_141674600. These are the add-body / remove-body /
shape-changed / break hooks that keep the graphics side in sync with physics fracture. (proven —
FUN_141670d10 +0x868…+0x940 block). It also seeds a per-frame command buffer via
FUN_141138ec0(…,0x8000,…) (inferred: 0x8000 = command/queue capacity). (proven — the call; size inferred)

Sub-objects `PTR_LAB_1423fbae0` (0x20 B) and `PTR_LAB_1423fbb10` (0x40 B) are allocated and back-pointed
to the world (`puVar8[3] = param_1`) — inferred: internal iterators/dispatchers. (inferred)

### 3. Breakable-body ↔ graphics registration (FUN_141672cd0)

When a breakable body enters the world, FUN_141672cd0 grows an indexed pointer table at `param_1+0x118`
(count at +0x120, capacity mask at +0x124 using the Apex `& 0x3fffffff` idiom) and slots the body's
graphics record by an index obtained from a virtual call `(*param_2)(+0x18)`. This is the intact-body
registry the graphics managers iterate each frame. (proven — FUN_141672cd0 body)

### 4. Per-frame fracture processing (FUN_141674640 / FUN_1416743b0)

FUN_141674640 is the graphics manager's per-frame worker. It walks a list of bodies whose state changed
(count at +0x50), and for each: (proven — FUN_141674640 body)
- If the body just became active (`+0x54 == -1`), it re-derives collision flags and calls
  FUN_1411136a0 / FUN_141113bb0 to (inferred) rebuild the body's collision filter, opening a profiler
  scope literally named **`"UpdateBodyShape"`** (`TtUpdateBodyShape`). This is the moment the physics
  body's shape is swapped from intact to fractured. (proven — the `"UpdateBodyShape"` scope string)
- It appends the body to per-frame add/remove work lists (`thunk_FUN_14b61e030` on +0x88/+0x90/+0x98),
  and walks a linked chain (`+0x60`) of sub-pieces, re-keying each to the physics world
  (thunk_FUN_14b56c5a0). (proven)

FUN_1416743b0 is the complementary **release** path: for each entry it frees the piece's graphics
(clears the index table at +0x108, resets shape fields, returns the slab via the frame allocator
`thunk_FUN_14b7bf440` → vtbl+0x20 free), and pushes the freed index onto a free-list (`+0x358`).
(proven — FUN_1416743b0 body)

### 5. Explosion → fracture (hkndExplosionRuntime, FUN_1416b1ef0 / FUN_1416b21e0)

FUN_1416b1ef0 constructs the explosion runtime (vtable `PTR_LAB_1423fa888`) and hooks three callbacks
onto the world tagged `"hkndExplosionRuntime"`: FUN_1416b21e0 (+0xa0), FUN_1416b27b0 (+0xc8), and a
world-level listener at +0x860 (LAB_1416b29f0). (proven — FUN_1416b1ef0 body)

FUN_1416b21e0 is the core per-body explosion handler. For a hit body (`param_3` index) whose flags
`+0x2c` have bits `0x1400` set, it: (proven — FUN_1416b21e0 body)
- Walks the **connectivity graph** of fractured sub-pieces via the flexible-joint table
  (`hkndFlexibleJointRuntime`), chaining through `+0x40` links and matching connected-component ids
  (the `local_res20`/`local_res18` id walk). (proven)
- For each connected piece it composes a **world transform** for the freed debris piece via full
  quaternion rotate + translate math (the `fVar22…fVar37` block: quat-conjugate rotate of the local
  offset, `2*(cross + w*v)` Rodrigues form, then matrix-multiply into the parent's 0xb0-byte transform),
  branching on a shape-type byte `cVar1` (0/1/2). (proven — the transform math)
- Sets break bits `0x1000` / `0x400` into the piece's flag word `+0x2c`, which marks it to detach and
  become an independent dynamic body. It also scales an incoming radius/velocity by
  `DAT_141ca6cac / body[+0x44]` (inferred: explosion strength ÷ piece mass or radius). (proven — flag
  writes and the `local_b8 = fVar8 / *(lVar11+0x44)` divide; semantics inferred)
- Newly-freed pieces are appended to the runtime's own body array (`param_1+0x20`, grown by
  thunk_FUN_14b932030 in 0x70-byte strides) and initialized via thunk_FUN_14ce90110. (proven)

### 6. Debris lifetime (hkndBodyTimeoutRuntime, FUN_1416af1b0)

FUN_1416af1b0 builds the timeout runtime (vtable `PTR_LAB_1423fa868`), preallocating a 0xc-stride
tracking array and hooking FUN_1416af390 (+0xa0, per-frame tick), FUN_1416af880 (+0xc8), and
LAB_1416afcf0 (+0x90) tagged `"hkndBodyTimeoutRuntime"`. This is the system that counts down spawned
debris and removes it once it has settled / a timer elapses, bounding the live-body count. (proven —
FUN_1416af1b0 body; exact timer value lives in per-instance data, not decomp constants)

### 7. Instanced debris rendering (FUN_14168a050)

The instanced graphics manager (0x7ed0 bytes) preallocates a set of GPU-instance pools sized for
mass destruction: reserves of **10000** (8-byte stride), **1000** (×several, 4/0xc-byte strides),
and **4000** (0x10-byte stride) — see the `thunk_FUN_14b931b60(..., 10000/1000/4000, stride)` calls.
It also registers a `"hkndDefaultController"` callback (FUN_14168aa60) on the world. The 10k/4k reserves
are the concrete ceiling on simultaneously-rendered fracture instances. (proven — FUN_14168a050 body)

### 8. Teardown / reset (FUN_14769ff10)

FUN_14769ff10 iterates the breakable-body array (`param_1+0x78`, count +0x80) and for each runs
`hkndBreakableBodyResetUtil` callbacks (thunk_FUN_1476570b0 at world +0x98/+0xa0), unregisters graphics
via FUN_141672730, frees the body, and finally flushes with FUN_141674640. Used on level unload /
destruction reset. (proven — FUN_14769ff10 body)

## Data & config integration

Destruction is exposed to the RTPC entity/data layer (see `[[rtpc-entity-assembly]]`,
`[[composite-assets]]`) as reflection-registered classes. Each `C<Name>` string is hashed and cached in
a thread-safe singleton via `FUN_140f27f60(name, len, …)` (the lookup3 name-hash path from
`[[name-hash-cracked]]`): (proven — the type-registration functions)

| RTPC class | Type-descriptor DAT | Registrar FUN_ |
|---|---|---|
| `CHavokDestructionObject` | `DAT_142cb91f0` | FUN_140a8f7c0 @0x140a8f7c0 |
| `CForcePulse` | `DAT_142cb91e0` | FUN_140a8f730 @0x140a8f730 / FUN_14080c5b0 @0x14080c5b0 (factory) |
| `CForceField` | `DAT_142cb959c` | FUN_140a8f610 @0x140a8f610 |
| `CForcePoint` | `DAT_142cb95a4` | FUN_140a8f6a0 @0x140a8f6a0 |

`CForcePulse` is a full RTPC component; its constructor FUN_140a754d0 @0x140a754d0 sets vtables
`PTR_thunk_FUN_14970cbc0_141df84a0` / `PTR_LAB_141df85a0`, names its force channel `"ForcePulse"`
(FUN_1476e7d10), and writes the default tunables listed below. (proven — FUN_140a754d0 body)

Physics force application is bucketed by a global **force-channel registry** (FUN at line 3378770ff,
`thunk_FUN_147728150("<channel>")`) that names every force type the solver accounts for. The
destruction-relevant channels are: `"ForcePulse"`, `"ForceField"`, `"ForcePoint"`, `"Explosion"`,
`"CollisionImpulseOnDebrisSpawn"`, `"ExplosionImpulseOnDebrisSpawn"`, `"PlantedExplosiveImpulse"`,
`"ProjectileExplosive"`, `"OnDamage"`. The two `*ImpulseOnDebrisSpawn` channels confirm that when a
piece spawns as debris the solver applies both a collision-derived and an explosion-derived impulse to
kick it. (proven — the string list)

"Chaos objects" in JC4 marketing map to `CObjectTrackerFilterChaosObject` (type reg FUN at
`DAT_142cb9998`) on the tracking side, and `TtDestruction` is the profiler timeline bucket (case 9 in
FUN around line 5321754) — i.e. destruction has its own CPU-timeline category. (proven — those strings)

## CForcePulse — force-application math (proven, deep dig)

`CForcePulse` is a **one-shot expanding radial shockwave**, reconstructed end-to-end. Object size **0x418**
(proven three ways: alloc `FUN_1496a12b0(0x418)` in FUN_140838720, deleting-dtor `FUN_140c7e180(_,0x418)`
in FUN_14970cbc0, factory name→type `DAT_142cb91e0`). Copy-ctor FUN_140a88bb0 exposes the full layout
(physics handle +0x398, resolved-body +0x3a0, magic `0xF1EF0A7B`).

**Per-frame tick = FUN_140abd680** (proven): gates on active byte +0x224, calls apply with frame `dt`
(`frameCtx+0x20`), then **grows the pulse radius** (min +0x218, cap `DAT_141cae1c0`) and **deactivates**
(+0x224=0) once the expanding radius passes +0x220 and count +0x234 hits 0 — i.e. a shell that sweeps
outward once, not a persistent field.

**Apply = FUN_140ab9cf0(this, dt)** (proven). Center = object translation (+0x134/138/13c); effective
radius `R = max(+0x218, physicsQueryRadius)`; cull at `R²`. For each candidate body at distance `d`:

```
nd      = clamp(d / R, 0, 1)
impulse = (1 - nd)^2                 // QUADRATIC falloff
        * magMax(+0x214 = 100.0)     // primary impulse scale
        * dirFactor                  // |magVec|^2 - (magVec·n)^2, clamped >=0
        * surfFactor                 // material/curve sample, clamped <= R^2
        * bodyFactor(~1.0) * DAT_141ca6f44 /*global*/ * dt
finalMag = min(impulse, invInertia(body) * max(+0x210 = 100.0, dirFactor))   // per-body velocity-change cap
dir      = normalize(center->body), rotated by object matrix, then JITTERED by +0x238/+0x23c
           via the MSVC-LCG RNG DAT_142c73a98 (seed*0x343fd+0x269ec3)  // same LCG family as weapon spread
impulseVec = dir * finalMag  ->  FUN_140a61ca0(body, impulseVec)
```

**Impulse sinks** (proven, both unique to FUN_140ab9cf0): `FUN_140a61ca0` = rigid-body sink — packs
{body handle, vec3} into a command struct and **enqueues a deferred physics command** (FUN_140a6d700 →
FUN_140099320), because Havok steps multithreaded. `FUN_140a622c0` = the **character/ragdoll** path,
carrying extra knockback payload (`FUN_14971e490`). Siblings `CForceField` (size 0x760, an oriented AABB
volume + reserved list) and `CForcePoint` (size 0x990, self-registers into global registry `DAT_142cb4b30`,
polarity float `-1.0` at +0x926 = attract/repel) use *different, unlocated* apply paths — their
ctor↔class binding is **inferred** (factory descriptor vtables `PTR_LAB_141d9b610/638/5e8` are data-section,
not in the dump).

## Notable constants / tunables

| Value (hex) | Decoded | Where | Meaning (grade) |
|---|---|---|---|
| `5000`, `10000` | ints | FUN_14009e600 (`local_40`,`local_3c`) | world-init reserve sizes passed to FUN_141670d10 (inferred) |
| `0x42c80000` | `100.0f` | FUN_140a754d0 +0x210 / +0x214 | CForcePulse magnitude **min / max** — the primary impulse scale (proven, §apply) |
| `0x3dcccccd` | `0.1f` | FUN_140a754d0 +0x238 / +0x23c | CForcePulse **direction jitter** base / scale (proven, §apply) |
| `0x41a00000` | `20.0f` | FUN_140a754d0 +0x240 | CForcePulse curve/tick-setup param — not in core apply (inferred) |
| `0x3f800000` | `1.0f` | FUN_140a754d0 +0x244 | CForcePulse falloff/shape param → curve setup (inferred) |
| curves | 8-pt @+0x300, ramp @+0x340 | FUN_14763a4f0 / FUN_140a749a0 | falloff curve objects; `surfFactor` term is the likely consumer (proven built; sample site inferred) |
| `0xdeadbeef` | sentinel | FUN_140a754d0 +0x2cc | uninitialized-id marker (proven idiom) |
| `10000`, `4000`, `1000` | element reserves | FUN_14168a050 | instanced-debris pool ceilings, strides 8 / 0x10 / 4/0xc (proven) |
| `0x8000` | 32768 | FUN_141670d10 (FUN_141138ec0) | world command-buffer capacity (inferred) |
| `0x1400` / `0x1000` / `0x400` | flag bits at body+0x2c | FUN_1416b21e0 | "was hit" mask / break-piece / break-piece-alt (proven flag writes; names inferred) |
| `DAT_141ca6cac` | **`1.0` (live-read)** | FUN_1416b21e0 | explosion strength scalar ÷ piece field +0x44 (value proven live; role inferred) |
| `DAT_141ca6f44` | **`0.5` (live-read)** | FUN_140ab9cf0 | global impulse scalar in the CForcePulse apply (value proven live; role inferred) |
| `DAT_141cae1c0` | **`50.0` (live-read)** | FUN_140abd680 | CForcePulse radius-growth cap in tick (value proven live) |
| system hashes | `0x41cad740`, `0x41cad888` | FUN_148f958e0/FUN_148f960c0 | `CHavokDestructionDecalManager` / `…InstancedGraphicsManager` (proven) |

Note: `0x80000000` written to `+0x?4` array-capacity fields and `& 0x3fffffff` capacity masks are the
Apex container idiom (top bit = "storage is inline / not heap-owned"), not destruction tunables —
appears throughout these ctors. (proven idiom, seen in every ctor above)

## Call-graph highlights

- FUN_1400cf270 (world/manager init) → **FUN_14009e600** (`CHavokDestructionSystem` ctor)
  → FUN_141670d10 (`hkndWorld`) → { FUN_14168a050 (instanced gfx), FUN_141672cd0 (body registry) };
  FUN_14009e600 also → FUN_14168a050, FUN_1416880e0 (decal), FUN_14166bf40, FUN_1410e83d0 (update reg).
- `hkndWorld` listeners: FUN_1416743b0 (release), LAB_141673e20/…/LAB_141674600 (add/remove/shape).
- Runtimes hooking the world independently: FUN_1416b1ef0 (`hkndExplosionRuntime`) → FUN_1416b21e0 /
  FUN_1416b27b0; FUN_1416af1b0 (`hkndBodyTimeoutRuntime`) → FUN_1416af390 / FUN_1416af880.
- Per-frame graphics sync: FUN_141674640 (`UpdateBodyShape`) and FUN_141672730 (unregister),
  both also called from teardown FUN_14769ff10.
- Singleton access: `DAT_142c84b98` (72 refs — the shared physics/destruction context, holds world at
  +0x18, gfx managers at +0x20/+0x28) and `DAT_142c84be0` (10 refs — set by the ctor).
  **Live-confirmed (x64dbg, 2026-08-16):** `DAT_142c84b98 → 0x0ef94c00`; prototype `PTR_LAB_141cad488`
  (installed by `FUN_14009e600` — identity proven); live fields `+0x18 = hkndWorld (0x0f3c6230)`,
  `+0x20/+0x28 = gfx managers (0x0f3431e0 / 0x0f332a00)` — **layout upgraded inferred → proven.** The
  context's virtual methods resolve through the prototype's code-slot thunks to `FUN_14766a080`,
  `FUN_1400a5150`, `FUN_1476c9e60`, `FUN_1400a51c0`, … (see `live_values.md` §5).

## Open questions / lower-confidence

- ~~Exact semantics of the `CForcePulse` floats~~ — **RESOLVED** (see "CForcePulse — force-application
  math" above): +0x210/+0x214 = magnitude min/max (100.0), +0x238/+0x23c = direction jitter (0.1). Retail
  `.epe` RTPC values still override these defaults, so per-object magnitudes differ.
- `CForceField` / `CForcePoint` per-frame apply functions are **not located** (they don't reuse the Pulse
  sinks); their ctor↔class binding is inferred via factory vtables `PTR_LAB_141d9b610/638/5e8` which live in
  the data section, absent from this dump — route: read those vtable slots in the loaded binary.
- ~~`DAT_141ca6f44` (global impulse scalar in the apply expression) value unresolved~~ — **RESOLVED via live
  read (x64dbg, 2026-08-16): `DAT_141ca6f44 = 0.5`** (`00 00 00 3f`). Value proven; role (global impulse
  scalar) still as-inferred. +0x240 = 20.0 is a proven ctor default.
- The debris despawn timer value lives in per-instance `hkndBodyTimeoutRuntime` data (0xc-stride array),
  not as a decomp literal — needs a live-data / tagfile read to pin down. (see `[[havok-hct-2018]]`)
- ~~`DAT_141ca6cac` value not resolvable from the text dump~~ — **RESOLVED via live read (x64dbg,
  2026-08-16): `DAT_141ca6cac = 1.0`** (`00 00 80 3f`). Heavily-shared engine float (7614 refs); value proven,
  its role here as the explosion scalar is inferred from the divide.
- **Live read (x64dbg, 2026-08-16): `DAT_141cae1c0 = 50.0`** (`00 00 48 42`) — the `CForcePulse` radius-growth
  cap in the tick (`FUN_140abd680`, line ~202). Value proven.
- The break-threshold (how much impulse fractures a given body) is a property of the Havok destruction
  asset (breakable-body definition in the `hknd` tagfile), not of this game-side code — belongs to the
  data side per `[[havok-hct-2018]]`, not re-derived here.
- Listener LAB_* targets (LAB_141673e20, LAB_1416741e0, LAB_141674560, LAB_1416745f0, LAB_141674600)
  were not individually opened; they are the add/remove/shape-changed hooks and would refine step 4.

## Appendix — decomp anchors

Strings: `"CHavokDestructionSystem"` (FUN_14009e600), `"CHavokDestructionGraphicsManager"`
(FUN_14009e600), `"CHavokDestructionInstancedGraphicsManager"` (=hash 0x41cad888, FUN_148f960c0),
`"CHavokDestructionDecalManager"` (=hash 0x41cad740, FUN_148f958e0), `"CHavokDestructionObject"`
(FUN_140a8f7c0), `"CForcePulse"` (FUN_140a8f730/FUN_14080c5b0/FUN_140a754d0), `"CForceField"`
(FUN_140a8f610), `"CForcePoint"` (FUN_140a8f6a0); `hknd` runtimes `"hkndWorld"`,
`"hkndDestructionSystem"`, `"hkndExplosionRuntime"` (FUN_1416b1ef0), `"hkndBodyTimeoutRuntime"`
(FUN_1416af1b0), `"hkndBreakableBodyResetUtil"` (FUN_14769ff10), `"hkndDefaultController"`
(FUN_14168a050), `"hkndDecalMapRuntime"`, `"hkndFlexibleJointRuntime"`, `"hkndDebrisFracture"`,
`"UpdateBodyShape"` (FUN_141674640), `"ForcePulse"` (FUN_140a754d0), `"CollisionImpulseOnDebrisSpawn"`
/ `"ExplosionImpulseOnDebrisSpawn"` (force-channel registry ~line 3378770), `"TtDestruction"` /
`"Physics_Destruction"` (profiler buckets).

Functions: FUN_14009e600 @0x14009e600, FUN_1400cf270 @0x1400cf270, FUN_141670d10 @0x141670d10,
FUN_14168a050 @0x14168a050, FUN_1416880e0 @0x1416880e0, FUN_14166bf40 @0x14166bf40,
FUN_141677d50 @0x141677d50, FUN_141672cd0 @0x141672cd0, FUN_141674640 @0x141674640,
FUN_1416743b0 @0x1416743b0, FUN_141672730 @0x141672730, FUN_14769ff10 @0x14769ff10,
FUN_1416b1ef0 @0x1416b1ef0, FUN_1416b21e0 @0x1416b21e0, FUN_1416b27b0 @0x1416b27b0,
FUN_1416af1b0 @0x1416af1b0, FUN_140a754d0 @0x140a754d0, FUN_14080c5b0 @0x14080c5b0,
FUN_140a8f7c0 @0x140a8f7c0, FUN_140a8f730 @0x140a8f730, FUN_1410e83d0 @0x1410e83d0,
FUN_148f958e0 @0x148f958e0, FUN_148f960c0 @0x148f960c0.

Data: `DAT_142cb91f0` (CHavokDestructionObject type), `DAT_142cb91e0` (CForcePulse type),
`DAT_142c84b98` / `DAT_142c84be0` (destruction/physics singleton holders), `DAT_141ca6cac`
(explosion scalar). Vtables: `PTR_LAB_141cada00` (world), `PTR_LAB_141cad500`/`_141cad520` (graphics
mgr), `PTR_LAB_141cad768` (instanced graphics mgr), `PTR_LAB_1423fbb68` (hkndWorld wrapper),
`PTR_LAB_1423fd5b0` (instanced gfx internal), `PTR_LAB_1423fa888` (explosion runtime),
`PTR_LAB_1423fa868` (body-timeout runtime), `PTR_LAB_141df85a0` (CForcePulse).

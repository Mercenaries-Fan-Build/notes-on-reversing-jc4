# Event bus & frame scheduler — the game-wide message + update backbone

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

## Overview

Just Cause 4 (Apex engine) drives cross-system communication through **one global, string-named,
deferred message bus** and ticks the world through a **service-locator of ~93 manager singletons** pumped
by a small set of per-frame state machines. Gameplay code fires lowercased dotted event names
(`"player.near.tornado"`, `"player.died"`, `"dlc.skystriker.load"`) which are tokenized, lowercased and
hashed into a **refcounted event-name intern table** owned by `CGameplayEventManager`
(singleton `DAT_142c84708`). A fire does **not** dispatch immediately — it enqueues a message into a
lock-free ring buffer that is **drained once per frame** (`FUN_14008fd80`), delivering each message to the
subscribers bound to that interned event slot. Subscribers bind by the *same* name-tokenizer
(`FUN_14762d8a0`). Parallel work (physics, animation, cloth, destruction, raycasts) is farmed to a
**Havok job queue** drained by worker threads (`FUN_14b9b0af0`) with typed task categories. Frame timing is
`QueryPerformanceCounter`-based with a 30 ms catch-up budget; per-thread `rdtsc` profile zones bracket job
execution. Blackboard **phase keys** (`POST_SELECTION`, `POST_UPDATE_CONTEXTS`) name ordering points within a
manager's update.

The **mechanism** (queue structure, tokenize/hash, ring drain, task-type table, manager registration order)
is fully recoverable. The **per-subscriber handler bodies** and **per-manager tick bodies** are data-section
vtable calls (`thunk_FUN_147630500` → subscriber vptr; `(**(code**)(*mgr + off))`), so the *content* of each
reaction is walled — resolve route in [Open questions](#open-questions--lower-confidence).

## Key classes & functions

| Name / string | FUN_ | Role |
|---|---|---|
| `CGameplayEventManager` | singleton `DAT_142c84708`; registered `FUN_148f960c0` / `FUN_146cd0bea` | Owns the event intern table + dispatch ring |
| Fire event by name | `FUN_147bfeb20` @0x147bfeb20 | Tokenize name → resolve token vector → enqueue (312 call sites) |
| Tokenize/lower/hash a name | `FUN_147625370` @0x147625370 | Space-split, `tolower`, hash each token → token-id vector; returns token count |
| Wrap token vector (alloc) | `FUN_14761f540` @0x14761f540 | Copies token array + count into a heap record |
| Post token vector to manager | `FUN_14762ba00` @0x14762ba00 → `FUN_14762b720` @0x14762b720 | Enqueue one message into the ring buffer (deferred) |
| Drain / dispatch pump | `FUN_14008fd80` @0x14008fd80 | Per-frame: pop ring, deliver each message; 3-pass reentrancy |
| Deliver one message | `FUN_147622110` @0x147622110 → `thunk_FUN_147630500` | Resolve event slot(s), notify each subscriber |
| Find-or-create event slot (refcount++) | `FUN_1400937b0` @0x1400937b0 | Hash-lookup/insert interned event; returns 24-byte slot ptr |
| Release event slot (refcount--) | `FUN_1400939a0` @0x1400939a0 | Frees slot back to free list, unlinks from hash map |
| Subscribe by name | `FUN_14762d8a0` @0x14762d8a0 | Tokenize name → resolve slot indices → bind (242 call sites) |
| Subscribe by pre-hashed tokens | `FUN_14762d4e0` @0x14762d4e0 | Same, from an existing token array |
| Bind index list into subscriber | `FUN_14762ba20` @0x14762ba20 | Stores byte-count-prefixed `u16` event-index list |
| Manager service-locator registrars | `FUN_148f960c0` @0x148f960c0 (size 4805), twin `FUN_146cd0bea` @0x146cd0bea | Instantiate + register ~93 manager singletons in order |
| Register one manager into locator | `FUN_148f6e780` @0x148f6e780 | Push `(name, obj)` into the service array |
| Load-screen frame pump (state m/c) | `FUN_148fe9ec0` @0x148fe9ec0 (size 2045) ← `FUN_1485db0a2` | Boot/load pump; drains events; 30 ms budget loop; state `DAT_142ae1868` |
| App/game frame pump (state m/c) | `FUN_14086d1e0` @0x14086d1e0 (size 4024) ← `FUN_148ca3c80` | Top-level app state pump; state `DAT_142ae184c` |
| Havok job worker dispatch | `FUN_14b9b0af0` @0x14b9b0af0 | Drains typed jobs; per-type vtable table; rdtsc profile zones |
| Critical-section enter / leave | `FUN_140f29f00` / `FUN_140f29f20` @0x140f29f0x | Guards the event registry (lock at `mgr+0x17a0f0`) |
| Worker spin-yield | `FUN_140f2a5e0` @0x140f2a5e0 | Busy-wait/yield on worker sync flags in the frame pumps |
| Blackboard phase-key registrars | `FUN_142f01370` / `FUN_142f016b0` | Register `..._POST_SELECTION` / `..._POST_UPDATE_CONTEXTS` HashString ids |
| `CTimestampManager` | reflected type `FUN_140963200` (ArGetTypeId 0x44) | Persistent `std::vector<STimestamp>` (save-state timing records) |
| `CBinaryStateObjectManager` | reflected type @ `FUN_14077fc70` (ArGetTypeId 0x4c) | Event-driven world-state toggles (binary state objects) |
| `CSettingsManager` | reflected `SInputPlayerBindings` (ArGetTypeId 0x5a) etc. | Settings / input bindings container |

## How it works (from the decomp)

### 1. Event-name interning: tokenize → lower → hash

`FUN_147625370(name, out_tokens, cap)` walks the string char-by-char, `tolower`s each byte, and **splits on
spaces** (`puVar4 += (c != ' ')`). At each space or end-of-string it null-terminates the accumulated word,
constructs a `CHashString` (`thunk_FUN_14cfe1310`) and appends its id (`thunk_FUN_14cfddc70`) into the output
vector, returning the token count (proven). A dotted name like `"player.near.tornado"` contains no spaces →
**one token**, so most events resolve to a single hashed id; the space-split exists so multi-word
console/debug commands (`"effect_shake_small"`, `"ply.input.enable"`) can be phrase-hashed. The hash is the
project-wide **lookup3 `hashlittle`** identity (see `docs/formats/name_hash.md`, memory `[[name-hash-cracked]]`)
— the gameplay-name namespace shares the asset-name hash (inferred from the `CHashString` path, consistent
with README's shared-registry finding).

### 2. The event registry (owned by `CGameplayEventManager`, `DAT_142c84708`)

The manager holds an **open-addressed hash map of interned events**, each a **24-byte (0x18) slot** in a base
array at `mgr + 0xe8`, indexed compactly by `idx = (slotPtr - mgr - 0xe8) / 0x18` (proven —
`FUN_14762d8a0`, `FUN_1400937b0`). Observed layout (offsets proven; field meanings inferred):

| Offset | Meaning |
|---|---|
| `+0xc8` | ptr to hash-bucket table (`u16` indices, modulus at `+0xd8`) |
| `+0xd0` | ptr to node array (16-byte nodes: `{u64 hash, u16 next, u16 slotIdx, …}`) |
| `+0xd8` | bucket count (`u16` modulus) |
| `+0xe0` | live event count (capacity guard `< 0xfc00`) |
| `+0xe8` | base of 24-byte event-slot array |
| `+0x17a0e8` | free-list head (slot index) |
| `+0x17a0f0` | critical section (enter `FUN_140f29f00` / leave `FUN_140f29f20`) |
| ring: `+0x10` read cursor · `+0x14` write cursor · `+0x18` capacity mask · `+0x20` ring base (40-byte slots) |

Per event slot (24 B): `+0x0` combined token hash · `+0x8` token count · `+0xc` first token id · `+0x14`
**refcount** (starts 1) · `+0x16` **subscriber-list head** (0 when empty). `FUN_1400937b0` locks the registry,
hashes the token vector, probes the bucket chain; on hit it **bumps the refcount** and returns the existing
slot, on miss it pops a slot from the free list, initializes it and inserts it into the hash map (proven).
`FUN_1400939a0` is the mirror: decrement refcount, and at zero push the slot back onto the free list and
unlink it — so **event names are refcounted and interned only while at least one subscriber holds them**
(proven). This is the intern table both `subscribe` and `fire` resolve against.

### 3. Subscribe

`FUN_14762d8a0(&binding, name, priority, flag)` clears any prior binding at `*param_1`, tokenizes the name
exactly as §1 into a 128-entry token buffer, and for each token calls `FUN_1400937b0` to find-or-create its
event slot, collecting the compact `u16` slot indices; then `FUN_14762ba20(&binding, count, indices)` stores
those indices into the subscriber's **byte-count-prefixed `u16` index list** (grows/reallocates in place)
(proven). `FUN_14762d4e0` is the same from a pre-hashed token array. This is the primitive the whole engine
uses to bind listeners — audio, weather, camera shake, demon, player input, console/debug commands
(`thunk_FUN_14762d8a0(target, "effect_shake_small", 0xff, 1)`), 242 call sites. `priority`/`flag`
(e.g. `0xff`/`1`, `0x106`) order and classify the subscription (inferred).

### 4. Fire (deferred)

`FUN_147bfeb20(name)` zeroes a 512-byte scratch, tokenizes (`FUN_147625370`), wraps the token vector
(`FUN_14761f540`) and calls `FUN_14762ba00(vec, &args)` → `FUN_14762b720(DAT_142c84708, vec, args)` (proven).
`FUN_14762b720` is a **lock-free ring enqueue**: it atomically increments the write cursor `+0x14`, computes
`slot = base + (mask & cursor)*0x28`, and copies the token-vector pointer + 4×`u64` of argument payload into
the 40-byte ring entry (proven). It `int3`-traps on ring overflow (`+0x18+1+read == write`). **Fire never
dispatches synchronously** — it only appends to the queue (proven). 312 fire sites; distinct names include
`player.died`, `player.killed.enemy`, `player.lowhealth`, `player.is.swimming`, `player.using.wingsuit`,
`player.pickup.weapon`, `player.near.tornado`, `player.near.sandstorm`, `on.demon.polymorphed`,
`game_loading_initiated/completed`, `loading_screen_start/end`, `frontend.new.loaded`,
`on_gamestaterun_enter`, `dlc.skystriker.load`, `arscan.triggered`, `ev.chaos_multiplier_changed`.

### 5. Drain / dispatch pump (once per frame)

`FUN_14008fd80` is the queue pump (proven). It reads the ring cursors and, **for up to 3 passes**, delivers
every queued message via `thunk_FUN_147622110(mgr, &ringSlot, …)` and advances the read cursor `+0x10`; then a
final unbounded drain empties whatever is left. The 3-pass-then-drain shape handles **reentrancy** — a
subscriber that fires further events during delivery (inferred). At the end it resets a "dispatching" atomic
flag at `mgr+0xa0`. `FUN_147622110` delivers one message two ways (proven): (A) if the message carries a token
vector, it resolves each token to its event slot (`FUN_140090f60`) and calls `thunk_FUN_147630500(slot, args)`;
(B) if it carries a pre-resolved index list, it calls `thunk_FUN_147630500(mgr + 0xe8 + idx*0x18, args)` for
each. `thunk_FUN_147630500` walks the slot's subscriber list (`slot+0x16`) and invokes each subscriber's
virtual handler — the handler bodies are per-subscriber vtable calls (walled). `FUN_14008fd80` is invoked
from the frame pumps (`FUN_148fe9ec0`, and world-update paths `FUN_14086b6e0`, `FUN_1402c2f60`) — i.e. the
bus is drained inside the tick, not on the firing thread (proven).

### 6. The manager service-locator (tick order)

`FUN_148f960c0` (README's singleton-manager registrar) and its twin `FUN_146cd0bea` build the world's
**service locator**: for each manager they `thunk_FUN_1496a12b0(8)`-allocate an object, set its vtable
(`PTR_LAB_141d90xxx`), and push `(name, obj)` into an array via `FUN_148f6e780` (proven). The registration
**order** (read from `FUN_146cd0bea`, 93 managers) is the construction order and the natural tick order
(inferred — the same array is walked to update):

```
CSoundSystem, CProfileManager, CVideoManager, CUIManager, CModelInstanceManager, CLandscapeManager,
CEffectSystem, CProjectEffectToTerrain, CSpawnSystem, CAiSystem, CHavokDestructionInstancedGraphicsManager,
CTargetSystem, CRiverManager, CWindTunnelManager, CTopographicalWind, CCoverageManager, CRoadManager,
CGameObjectManager, CGameWorld, CConstraintFactory, CConditionalManager, CCutsceneManager, CActivityManager,
CObjectiveManager, CObjectiveContentManager, CTacticalNodeManager, CRewardSystem, CMediaRevolutionManager,
CRetoolerManager, CNewGamePlusManager, CFrontlineManager, CHeatManager, CMissionManager, CQuestManager,
CDialogueCoordinator, CLightningManager, CWorldTime, CCharacterManager, CPlayerManager,
CPlayerSpawnPointManager, CAmmunitionManager, CWeaponPlacementRulePersistentManager, CDialogueManager,
CRadioSystem, COperationManager, COnlineSuiteManager, CDownloadableContentManager, CFriendManager,
CStatisticManager, CNotificationManager, CChallengeManager, CPlayerReportingManager, CVideoRecordingManager,
CAchievementsManager, CSquareEnixMembership, COnlineFeatureManager, CFpsCounter, CStashManager, CSkinManager,
CButtonHintManager, CVocalsManager, CCollectionManager, CSupplyManager, CSupplyFactoryManager,
CStatisticManager, CEncounterManager, CWeaponManager, CUIInputManager, CGameplayEventManager,
CTutorialManager, CTimestampManager, CBinaryStateObjectManager, CObjectiveDebugTextManager,
CSupplyDropManager, CSupplyRewardManager, CDiscoveryManager, CBiomeManager, CLeaderboardManager,
CPilotDataManager, CVehicleDataManager, CCreatureManager, CTurnTaker, CActionTokenManager, COnRoadService,
CEnvironmentGraphicsModifierManager, ChromaManager, CPresenceManager, CDaredevilPointManager,
CDemonAreaManager, CAgencyBaseManager, CDamageCoordinator, CVegetationInteraction, CRicoVocals
```

Each manager carries a 32-bit id set at construction (e.g. `CSoundSystem` `0x41cc6ff0`, `CProfileManager`
`0x41d8e958`, `CVideoManager` `0x41d8e998`) (proven). Managers referenced elsewhere (`CGameplayEventManager`,
`CTimestampManager`, `CBinaryStateObjectManager`, `CSettingsManager`) also register their **ADF-reflected
types** through `FUN_140f27f60("class CHashString __cdecl ArGetTypeId<class …>(void)", …)` — the
reflection-hash registry, so a manager's runtime type-id is the lookup3 hash of its `ArGetTypeId` signature
(proven; ties to `docs/formats/adf.md`).

### 7. The frame pumps + timing

The outer loop is a stack of state-machine pumps that read the frame delta from the **frame context**:
`dt = *(float*)(param_1 + 0x2c)` (seconds), with `+0x20`/`+0x28` further frame ints (proven). `FUN_14086d1e0`
(← `FUN_148ca3c80`) advances the **app state machine** `DAT_142ae184c` (0=boot/init, 1=front-end load,
2=in-game …); `FUN_148fe9ec0` (← `FUN_1485db0a2`) advances the **load state machine** `DAT_142ae1868`
(1→7: stream world, spawn player, `dlc.skystriker.load` fire at 3→4, etc.) while keeping the renderer and the
event bus alive. Both spin-yield on worker sync flags via `FUN_140f2a5e0` (proven).

`FUN_148fe9ec0` shows the **timing model** (proven): it lazily caches `QueryPerformanceFrequency` in
`DAT_142c846a0`, samples `QueryPerformanceCounter`, and runs a catch-up loop guarded by
`(freq * 30000)/1000000` counter ticks — a **30 ms (~33 fps) per-frame budget** — exiting early when the
budget is spent or there is no pending work, then finalizes the render side. Inside the loop it ticks the
world subsystems in a fixed sequence (physics `FUN_140105d90`, world/streaming `FUN_140109cf0`, spawn
`FUN_1400e4700`, weather `FUN_1407fee80`, etc.). Concurrency between update jobs is gated by named sync-point
schedulers created at static-init: `AccessWorldScheduler` (`DAT_142c84a70`) and the double-buffered
`NextGameUpdateAccessWorldScheduler_0` / `_1` (`DAT_142c84af0` / `DAT_142c849f0`) — a classic double-buffered
game-update (frame N reads while N+1 writes) (proven strings; double-buffer interpretation inferred).

### 8. Job / task system (parallel work)

The parallel backbone is **Havok's job queue** (`hkgpJobsQueue`), drained by worker threads in
`FUN_14b9b0af0` (proven). Each worker pulls a job, switches on its **task-type id** and dispatches through a
per-type vtable table at `worker + 0x148 + type*0x20` indexed by a subtype byte, bracketing execution with
`rdtsc` profile-zone markers pushed into a TLS profiler buffer (`TlsGetValue(DAT_142cee698)`). The task-type
enumeration (proven, string-labelled):

| id | Task |
|---|---|
| 0,1,0xd | `TtPhysics 2012` |
| 2 | `TtCollision Query` |
| 3 | `TtRayCast Query` |
| 4 | `TtAnimation Sample and Combine` |
| 5 | `TtAnimation Sample and Blend` |
| 6 | `TtAnimation Mapping` |
| 7 | `TtBehavior` |
| 8 | `TtCloth` |
| 9 | `TtDestruction` |
| 0xb | `TtCharacter Proxy` |
| 0xc | `TtVehicle` |
| 0xe | `TtUserJob` |
| default | `TtOther` |

Supporting labels seen: `TtGetNextJob`, `TtAddJobBatch`, `TtfinishJob`, `TtNoJobAvailable`, `SetupScheduler`
/ `StSetupScheduler`. The `Tt` prefix marks the engine's task-threading profile zones over the Havok queue.
So gameplay-level "jobs/tasks" are **Havok job batches**, distinct from the CPU-side event bus and the
main-thread manager tick (proven for the enumerated types; that no separate general-purpose gameplay
job-graph exists in the string inventory is inferred).

### 9. Update phases & blackboard keys

Ordering points inside a manager's update are named as **blackboard persistence ids** — HashString keys
registered at init and cached in globals: `BLACKBOARD_PERSISTENCE_ID_CAMERA_MANAGER_POST_SELECTION`
(`DAT_142cb3894`, `FUN_142f01370`) and `..._POST_UPDATE_CONTEXTS` (`DAT_142cb3a3c`, `FUN_142f016b0`), both via
`thunk_FUN_14aadec10(name)` = HashString register (proven). A **blackboard** is a keyed value store carried
across update phases; these keys mark camera-manager phase boundaries (POST_SELECTION, POST_UPDATE_CONTEXTS),
matching the camera doc's note. Related blackboard classes register through the same HashString path:
`CBlackboardCursorState`, `CBlackboardValueCompareCondition`, `CDemonEngagedCharacterHasBlackboardValue`
(proven). The full per-phase update ordering table is not present as data in the functions-only export
(vtable-driven) — see Open questions.

### 10. Timestamps & binary-state objects

`CTimestampManager` serializes a persistent `std::vector<STimestamp>` plus a `u64` (its reflected members are
registered at `FUN_140963200`, ArGetTypeId 0x44 / vector 0x7a / `u64` 0x3d) — i.e. a **saved list of named
game-time timestamps**, the backbone for "time since event" / cooldown logic (proven type shape; usage
inferred). `CBinaryStateObjectManager` (reflected at `FUN_14077fc70`, ArGetTypeId 0x4c) manages
**binary state objects** — event-driven on/off world-state toggles — resolved through the same ADF sorted
`(typeId, value)` binary-search property tables (proven type registration; "on/off world toggle" reading
inferred from the name + event-bus coupling).

## Data & config integration

- Event names are **data-authored strings** fired/bound from RTPC entity components and gameplay code; the
  bound token id is the lookup3 hash of the lowercased name, the same hash used for asset paths and ADF/TAB
  keys (`docs/formats/name_hash.md`). A new subscriber only needs the exact lowercased name string.
- Managers are ADF-reflected types; their runtime type-id = lookup3 of `ArGetTypeId<class …>` — the bridge to
  entity component hashes in memory `[[rtpc-entity-assembly]]` / `[[composite-assets]]`.
- `CSettingsManager` holds reflected structs (`SInputPlayerBindings`, ArGetTypeId 0x5a) → settings/config are
  ADF property tables, edited via the same reflection lookup (`FUN_140b63f80` binary-search pattern).
- `CTimestampManager` / binary-state objects are **save-game state** (serialized vectors), so their values are
  data-side, not in the executable.

## Notable constants / tunables

| Constant | Where | Meaning |
|---|---|---|
| `0xfc00` | `FUN_1400937b0` (`mgr+0xe0` guard) | Max live interned events |
| `0x18` (24) | event-slot stride (`mgr+0xe8`) | Event-slot record size |
| `0x28` (40) | ring-entry stride (`mgr+0x20`) | Queued-message record size |
| `0x80` (128) | `FUN_14762d8a0` token buffers | Max tokens per subscribe/name |
| 3 passes | `FUN_14008fd80` | Reentrancy dispatch passes before unbounded drain |
| `(freq*30000)/1000000` | `FUN_148fe9ec0` | 30 ms per-frame catch-up budget |
| `FUN_140f8f950(25000)` | `FUN_148fe9ec0` | ~25 ms sub-budget/watchdog (inferred) |
| Task ids 0–0xe | `FUN_14b9b0af0` | Havok task-type dispatch table |
| ArGetTypeId lens 0x44/0x4c/0x5a | manager type registrations | signature-string lengths for lookup3 |

## Call-graph highlights

- **Fire:** `FUN_147bfeb20` → `FUN_147625370` (tokenize) → `FUN_14761f540` (wrap) → `FUN_14762ba00` →
  `FUN_14762b720` (ring enqueue).
- **Drain:** frame pump `FUN_148fe9ec0` → `FUN_14008fd80` → `FUN_147622110` → `thunk_FUN_147630500`
  (per-subscriber vtable). Also driven from world-update `FUN_14086b6e0`, `FUN_1402c2f60`.
- **Subscribe:** callers (weather `FUN_149843a50`/`FUN_149139c10`, camera, demon, input) → `FUN_14762d8a0`
  → `FUN_1400937b0` (intern) → `FUN_14762ba20` (bind list).
- **Manager install:** `FUN_148f960c0` / `FUN_146cd0bea` → `FUN_148f6e780` (×93).
- **Frame loop:** `FUN_1485db0a2` → `FUN_148fe9ec0`; `FUN_148ca3c80` → `FUN_14086d1e0`; both → `FUN_140f2a5e0`
  (worker spin) and world-subsystem ticks.
- **Jobs:** worker → `FUN_14b9b0af0` → per-type vtable dispatch (Havok `hkgpJobsQueue`).

## Open questions / lower-confidence

- **Per-subscriber handler bodies** (`thunk_FUN_147630500` → subscriber vptr) and **per-manager tick bodies**
  (`(**(code**)(*mgr + off))`) are data-section vtable calls — not recoverable from the functions-only export.
  *Resolve route:* dump the vtables at the `PTR_LAB_141d90xxx` addresses (and subscriber object vptrs) from the
  live binary in x64dbg while PAUSED, or from the ADF/data sections, to map each slot to its handler FUN_.
- **Exact per-frame manager update ordering** vs. registration order is inferred; the concrete update walk (and
  whether managers are grouped into named phases beyond the camera POST_SELECTION/POST_UPDATE_CONTEXTS keys) is
  vtable/data-driven. *Resolve route:* trace the array at the service-locator (`param_1+0x40..0x50`) consumer.
- **`priority`/`flag` args** to `FUN_14762d8a0` (`0xff`, `0x106`, `1`) — ordering vs. classification semantics
  unconfirmed.
- **Double-buffer schedulers** `NextGameUpdateAccessWorldScheduler_0/_1` — the read/write swap discipline is
  inferred from the naming; the actual barrier logic lives in `FUN_140098df0`-built objects.
- Whether a separate general-purpose gameplay job graph exists beyond the Havok task queue — none surfaced in
  the string inventory (absence, not proof).

## Appendix — decomp anchors

- Bus core: fire `FUN_147bfeb20` @0x147bfeb20 · tokenize `FUN_147625370` @0x147625370 · wrap `FUN_14761f540`
  @0x14761f540 · post `FUN_14762ba00` @0x14762ba00 → `FUN_14762b720` @0x14762b720 · drain `FUN_14008fd80`
  @0x14008fd80 · deliver `FUN_147622110` @0x147622110 · notify `thunk_FUN_147630500`.
- Intern table: `FUN_1400937b0` @0x1400937b0 (find/create, refcount++) · `FUN_1400939a0` @0x1400939a0
  (release) · singleton `DAT_142c84708` (slots `+0xe8`/0x18, free-list `+0x17a0e8`, lock `+0x17a0f0`, ring
  `+0x10/+0x14/+0x18/+0x20`).
- Subscribe: `FUN_14762d8a0` @0x14762d8a0 · `FUN_14762d4e0` @0x14762d4e0 · bind `FUN_14762ba20` @0x14762ba20.
- Managers: registrars `FUN_148f960c0` @0x148f960c0, `FUN_146cd0bea` @0x146cd0bea · push `FUN_148f6e780`
  @0x148f6e780 · alloc `thunk_FUN_1496a12b0` · vtables `PTR_LAB_141d90448…`.
- Frame pumps: `FUN_148fe9ec0` @0x148fe9ec0 (← `FUN_1485db0a2` @0x1485db0a2) · `FUN_14086d1e0` @0x14086d1e0
  (← `FUN_148ca3c80` @0x148ca3c80) · spin `FUN_140f2a5e0` @0x140f2a5e0 · lock `FUN_140f29f00`/`FUN_140f29f20`.
- Schedulers (static-init): `AccessWorldScheduler` `FUN_142e7a350`/`DAT_142c84a70`;
  `NextGameUpdateAccessWorldScheduler_0` `FUN_142e7a660`/`DAT_142c84af0`; `_1` `FUN_142e7a890`/`DAT_142c849f0`
  (built by `FUN_140098df0`).
- Jobs: worker `FUN_14b9b0af0` @0x14b9b0af0; labels `TtPhysics 2012`, `TtCollision Query`, `TtRayCast Query`,
  `TtAnimation Sample and Combine/Blend`, `TtAnimation Mapping`, `TtBehavior`, `TtCloth`, `TtDestruction`,
  `TtCharacter Proxy`, `TtVehicle`, `TtUserJob`; `hkgpJobsQueue` @l.5550464.
- Blackboard phase keys: `FUN_142f01370` (`..._POST_SELECTION` → `DAT_142cb3894`), `FUN_142f016b0`
  (`..._POST_UPDATE_CONTEXTS` → `DAT_142cb3a3c`), register thunk `thunk_FUN_14aadec10`.
- Reflected managers: `CTimestampManager` `FUN_140963200` (ArGetTypeId 0x44); `CBinaryStateObjectManager`
  `FUN_14077fc70` (0x4c); `CSettingsManager::SInputPlayerBindings` `FUN_140b63f80` (0x5a); type registrar
  `FUN_140f27f60`.
- Event-name catalog (fired via `FUN_147bfeb20`, 312 sites; sample): `player.died`, `player.killed.enemy`,
  `player.lowhealth`, `player.is.swimming`, `player.using.wingsuit`, `player.using.parachute`,
  `player.parachute.reel.in`, `player.wingsuit.reel.in`, `player.pickup.weapon`, `player.near.tornado`,
  `player.near.sandstorm`, `on.demon.polymorphed`, `game_loading_initiated`, `game_loading_completed`,
  `loading_screen_start/end`, `frontend.new.loaded`, `on_gamestaterun_enter`, `on_gamestatefrontend_enter`,
  `dlc.skystriker.load`, `arscan.triggered`, `ev.chaos_multiplier_changed`, `gsy.object_destroyed`,
  `playerspawnpoint.triggered`, `any_node_secured`, `any_node_cleared`, `pause_game_checked/clear`.

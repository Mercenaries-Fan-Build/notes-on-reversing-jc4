# Narrative, Cutscene & Media — story presentation, cinematics, the Media Revolution & video

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

## Overview

This system covers how JC4 presents its story: **cutscenes/cinematics** (staged, skippable scenes that
lock out gameplay), the **Media Revolution** — JC4's signature in-world propaganda / news-broadcast
progression flavor system — the **"content introduction"** intros ("new content unlocked" splashes),
and **video playback / recording** (fullscreen and in-world video, plus the replay/share recorder).

All of these are engine **manager singletons** registered in one global table, plus a set of ADF/RTPC
**data types** (milestones, thresholds, rewards, cutscene/sequence objects) and behavior-VM
**conditions** (`CConditional_IsAnyCutsceneActive`, …) that let mission/gameplay logic query narrative
state. Cinematic scenes are authored as compiled `cc*` resources (director/sequence/scenario), and
video is composited through **Scaleform** (ActionScript `LoadVideo`/`UnloadVideo`) with **Bink** as the
licensed video codec (YUV planes uploaded as GPU textures). Dialogue is a sibling system documented in
[audio_dialogue.md](audio_dialogue.md); this doc references it but focuses on cinematic/media orchestration.

**Methodology caveat (functions-only export).** Recoverable here: the manager-registration skeletons,
the ADF-type registrars, the behavior-condition table, the Scaleform invoke sites, the cutscene-manager
tick, and string/constant inventory. **Not** recoverable: the per-manager component bodies
(`CVideoManager`, `CMediaRevolutionManager`, `CVideoRecordingManager`), which sit behind data-section
vtables — e.g. `FUN_14306988e` reaches "CVideoManager" only as an obfuscated debug tag, not real logic.
Those are tagged where they wall off.

## Key classes & functions

| Name (string) | FUN_ / token | Role |
|---|---|---|
| `CCutsceneManager` | registry `FUN_146cd0bea` → vtable `PTR_LAB_141d906c8`; tick `FUN_140687e90` | Cutscene queue + active-state manager (proven) |
| `CCutscene` | ADF type, token `DAT_142cb7dac` (reg via `FUN_14085fd00`) | Cutscene data object (proven) |
| `CCutsceneScene` | ADF type, token `DAT_142cb7db4` | A scene within a cutscene (proven) |
| `CCinematicTrigger` | ADF type `FUN_140bd4bd0`, token `DAT_142cb972c` | World trigger volume that starts a cinematic (proven) |
| `CSequenceObject2` | ADF type, token `DAT_142cb9278` (`FUN_140...`, e.g. line 868285) | Timeline/sequence object (proven) |
| `CConditional_IsAnyCutsceneActive` | behavior cond, `FUN_148f99a90` → `PTR_LAB_141d92938` | "is a cutscene playing" gate for gameplay/mission logic (proven) |
| `ACT_MOVE_CINEMATIC_MODE` | action id `FUN_142f50690`, token `_DAT_142cb3020` | Cinematic-camera locomotion/movement mode action (proven) |
| Skip hint | `FUN_140e2dac0` Show / `FUN_140e0aef0` Hide / `FUN_140e37a60` Update | HUD "hold to skip cutscene" prompt (proven) |
| `CMediaRevolutionManager` | registry `FUN_146cd0bea` → `PTR_LAB_141d90788` | Propaganda/news progression manager singleton (proven skeleton; body walled) |
| `CMediaRevolutionMilestone` | ADF type `FUN_14080e9e0`, token `DAT_142cb9d74`, vtable `PTR_LAB_141d9c1f0` | A propaganda milestone (proven) |
| `CMediaRevolutionThreshold` | ADF type `FUN_14080eb60`, token `DAT_142cb9d84`, vtable `PTR_LAB_141d9c240` | Threshold that unlocks a milestone (proven) |
| `CMediaRevolutionReward` | ADF type `FUN_14080eaa0`, token `DAT_142cb9d7c`, vtable `PTR_LAB_141d9c218` | Reward granted at a milestone (proven) |
| `CConditional_IsMediaRevolutionMilestoneApplied` | behavior cond, `FUN_148f99a90` (`thunk_FUN_147cafcd0` @4140522); token `DAT_142cb94ec` | Query: has milestone N been applied (proven) |
| `CConditional_CanUnlockNextMediaRevolutionMilestone` | behavior cond (@4140527) | Query: is next milestone unlockable (proven) |
| `UIMediaRevolutionController` | Scaleform controller pushed by `FUN_140e53e40` | Media Revolution UI screen (proven) |
| `CContentIntroductionManager` | ADF type `FUN_1408093c0`, token `DAT_142cb93c8`, vtable `PTR_LAB_141d9ec98` | "New content unlocked" intro manager (proven) |
| `CContentIntroductionObject` | ADF type `FUN_140809480`, token `DAT_142cb9924`, vtable `PTR_LAB_141d9ec70` | One content-intro entry (proven) |
| `CConditional_ContentIntroductionStatus` | behavior cond, `FUN_148f99a90` (@4140442); token `DAT_142cb5a18` | Query content-intro state (proven) |
| `CVideoManager` | registry `FUN_146cd0bea` → id `0x41d8e998` | Video subsystem singleton (proven skeleton; body walled at `FUN_14306988e`) |
| `CVideoRecordingManager` | registry `FUN_146cd0bea` → vtable `PTR_LAB_141d90aa8` | Replay/recording ("share") manager (proven skeleton; body walled) |
| Scaleform video | `FUN_140dc6770` (Load/UnloadVideo), `FUN_140daa1e0` | AS-side video load/unload on a movie clip (proven) |
| `videos/` loader | `FUN_140e9f2f0` (caller `FUN_140e969f0`) | Builds `videos/<name>` resource path + loads (proven) |
| Fullscreen video player | `FUN_140e8fd50`; state `FULLSCREEN_VIDEO` (`FUN_140d78c40`) | Render-surface video playback + `video.fullscreen.stop` event (proven) |
| In-world video / projector | `FUN_14a8725f0` (Y/Cr/Cb planes), `FUN_140cb4c80` (`video_projector_texture`) | YUV→RGB video-projector material (proven) |
| Bink codec | middleware table `FUN_14a61a6f0` → `"art.middleware.bink"` | RAD Bink Video = licensed video middleware (proven present; playback path walled → inferred) |

## How it works (from the decomp)

### Cutscene manager tick — `FUN_140687e90`
The manager holds a **cutscene queue** as a vector of pointers at `this+0x40 .. this+0x48` with **0x10**
stride; `(this+0x48 − this+0x40) >> 4` is the queue count (line 646751). Each tick it recomputes a state
int (`this+0x14`) and, gated by a global disable flag `DAT_142cb8f39` and another manager's byte
(`*(DAT_142cb7dd8+0x140) != 2`), derives whether a cutscene is **active** (`cVar6`). On the
active↔inactive **edge** (`this+0x21` is the last active bool):

- going **inactive** → `thunk_FUN_14af0c550(this+0x28, "CCutsceneManager")` — releases a named
  input/gameplay-lock context keyed by the manager name — and, if a fade/timer at `this+0x1c > 0`, resets
  it and calls `FUN_140e0aef0(DAT_142cb7dc0, 1)` = **HideSkipCutsceneHint** (proven).
- going **active** → `thunk_FUN_14af0c7a0()` acquires the lock (proven).

So a cutscene **suspends gameplay input** by pushing a named lock context, and the skip-hint HUD is bound
to the same edge. (proven — `FUN_140687e90`, caller `FUN_14086ed50`.)

### Skipping a cutscene — `FUN_140e2dac0` (ShowSkipCutsceneHint)
The Show function invokes the Scaleform HUD method `"ShowSkipCutsceneHint"` via the movie-clip vtable
(`(*(*param_1+0x38))(param_1,"ShowSkipCutsceneHint",args,3)`), passing (a) an input-binding glyph — the
literal `"common_widgets.win64.cancel"` on KB/M, or a resolved gamepad button via `thunk_FUN_14a896cb0`
(line 1645017/1645021) — and (b) a **hold duration** float `param_2`. `FUN_140e37a60` =
`UpdateSkipCutsceneHint` drives the hold progress; `FUN_140e0aef0` = `HideSkipCutsceneHint`. Drivers are
`FUN_1489981c0` and `FUN_140de44d0`. Conclusion: **skipping is a hold-the-cancel-button gesture** with an
on-screen hold meter. (proven.)

### Cinematic content authoring — the `cc*` resource family
A resource-type→extension table (`FUN_...` around lines 12270–12345, hash keyed via `FUN_14008d460`)
registers the compiled cinematic authoring formats: **`ccdirectorc`**, **`ccsequencec`**,
**`ccscenarioc`**, **`ccmapc`**, `ccenemyc` (proven strings). These are the "compiled cutscene" content
graph — a *director* drives *sequences*/*scenarios*. (Exact hash↔extension pairing is offset by one slot
in the table's write pattern, so the specific 32-bit ids are **inferred**, not proven.) `CCinematicTrigger`
(`FUN_140bd4bd0`) is the world-placed volume that fires one of these; `ACT_MOVE_CINEMATIC_MODE`
(`FUN_142f50690`) is the locomotion action for cinematic-camera control.

### Media Revolution — data-driven propaganda progression
The Media Revolution is a **milestone/threshold/reward** progression system (same shape as the other
progression managers). Three ADF reflected types are registered through the class factory `FUN_14085fd00`:
`CMediaRevolutionMilestone` (`FUN_14080e9e0`), `CMediaRevolutionThreshold` (`FUN_14080eb60`),
`CMediaRevolutionReward` (`FUN_14080eaa0`) — each registers its type token via `FUN_140f27f60(name,len)`
and installs a factory vtable (`PTR_LAB_141d9c1f0/240/218`) via `thunk_FUN_14cfef490`. Gameplay logic
queries state through two behavior conditions registered in `FUN_148f99a90`:
`CConditional_IsMediaRevolutionMilestoneApplied` and `CConditional_CanUnlockNextMediaRevolutionMilestone`.

The **UI** side is `UIMediaRevolutionController`: `FUN_140e53e40` opens it (`FUN_140eb2830(DAT_141ee61b4,
"UIMediaRevolutionController")`) and then **iterates the milestone list** at `local_90+0x170 .. +0x178`
(0x10 stride, count `(hi−lo)>>4`, line 1668341), pushing each milestone entry into the Scaleform screen.
So: data defines milestones and their unlock thresholds/rewards → conditions gate mission logic → the
controller renders the propaganda board. (proven skeleton; the manager's scheduling/broadcast body is
behind its data-section vtable — walled.)

### Content Introduction — "new content unlocked" intros
`CContentIntroductionManager` (`FUN_1408093c0`) + `CContentIntroductionObject` (`FUN_140809480`) are ADF
types; `CConditional_ContentIntroductionStatus` (`FUN_148f99a90`, token `DAT_142cb5a18`) exposes intro
state to logic. These drive the splash/intro shown the first time a feature or region is unlocked.
(proven skeleton; body walled.)

### Video playback — Scaleform + Bink
Two paths:

1. **Scaleform-composited video.** `FUN_140dc6770` invokes ActionScript `"LoadVideo"` / `"UnloadVideo"`
   on a movie clip (`(*(*param_1+0x38))(param_1,"LoadVideo",&info,1,…)`), where `info` is a small struct
   built from template `_DAT_141ca6dd0` plus the video handle at `+0x20` (line 1581374-1381379).
   `FUN_140daa1e0` unloads via `PTR_FUN_141ed28d8`. `FUN_140de2b50` handles `"SetPreviewVideo"` (menu
   preview tiles). The **resource path** is built by `FUN_140e9f2f0`: it constructs `"videos/" + <name>`
   (`thunk_FUN_1475b02b0(…,"videos/",7)`, line 1714325) and loads via `thunk_FUN_14a8a2af0`.
2. **Fullscreen video.** `FUN_140e8fd50` allocates a **render surface** (0x200-byte object at `this+0x48`),
   configures its flags, and subscribes to the `"video.fullscreen.stop"` message with priority `0xff`
   (`thunk_FUN_14762d8a0`, line 1704143). `FUN_140ea0260` (large Scaleform fn) references
   `"_root.OnVideoStop"`; the UI state token is `FULLSCREEN_VIDEO` / `FULLSCREEN_VIDEO` (`FUN_140d78c40`).

**Codec.** The middleware-attribution table `FUN_14a61a6f0` registers `"art.middleware.bink"` alongside
havok/nvidia/simplygon/speedtree — so **RAD Bink Video** is the shipped video codec (proven present).
Consistent with that, the in-world video path (`FUN_14a8725f0`) creates **three GPU textures**:
`"video_Y"` at full resolution and `"video_cR"`, `"video_cB"` at **half** resolution
(`uVar1>>1, uVar7>>1`) — a classic **YUV 4:2:0** decode uploaded as separate planes and converted to RGB
in a shader. `FUN_140cb4c80` binds the `"video_projector_texture"` material — the in-world screen/projector
surface. (Plane layout proven; that Bink specifically feeds these planes is inferred — the decode call
itself is behind the middleware vtable.)

### Video recording / replay
`CVideoRecordingManager` is registered in the manager table (vtable `PTR_LAB_141d90aa8`). This is JC4's
capture/replay ("share") manager. Its body is behind the data-section vtable — **walled**; only the
registration and name survive. (proven registration; mechanism not recoverable from this export.)

### Boot & frontend intro sequence — `FUN_1408718a0`
The new-game / boot flow selects a **boot point** by matching a saved id: `"intro.boot.point.start"`
(default) or `"intro.boot.point.slumbridge"` (line 920651/920669), then either triggers the event
`"intro.cutscene.location.load"` (`thunk_FUN_147bfeb20`, line 920696, sets `DAT_142cb8f50 = 1`) or loads
directly into a save location. It also touches main-mission label `"mm110"` (`FUN_140f27f60("mm110",5)`).
Frontend side: `ShowFrontendIntroBG` (`FUN_140e27ba0`), Scaleform screen `"IntroUI"` (`FUN_140e2f740`),
and query methods `IsIntroMovieComplete` (`FUN_140e93530`) / `IsIntroSequenceComplete` (`FUN_140e935b0`,
called from the boot orchestrator `FUN_14086d1e0` and others). Achievement `ach_quest_all_intros` and flag
`m_IntroCompleted` track intro completion. (proven strings/flow; the Scaleform bodies are opaque.)

## Data & config integration

- **ADF/RTPC types.** `CCutscene`, `CCutsceneScene`, `CSequenceObject2`, `CCinematicTrigger`,
  `CMediaRevolution{Milestone,Threshold,Reward}`, `CContentIntroduction{Manager,Object}` are all ADF
  reflected types registered by `FUN_140f27f60(name,len)` and given factory vtables via
  `thunk_FUN_14cfef490` (through the class factory `FUN_14085fd00`). Their instances live in the game's
  entity/component data (`docs/formats/adf.md`, memory `[[composite-assets]]`, `[[rtpc-entity-assembly]]`).
  `CCinematicTrigger` is a world-placed entity component (like other trigger volumes).
- **Cinematic content** is authored as compiled `ccdirectorc` / `ccsequencec` / `ccscenarioc` / `ccmapc`
  resources (see the resource-extension table ~line 12270-12345).
- **Behavior VM.** `CConditional_IsAnyCutsceneActive`, `CConditional_IsMediaRevolutionMilestoneApplied`,
  `CConditional_CanUnlockNextMediaRevolutionMilestone`, `CConditional_ContentIntroductionStatus` are
  registered in the condition table `FUN_148f99a90`; they are how missions/behavior graphs branch on
  narrative state (see [behavior_system.md](behavior_system.md)).
- **UI.** `UIMediaRevolutionController`, `IntroUI`, and the Scaleform video verbs (`LoadVideo`,
  `UnloadVideo`, `SetPreviewVideo`, `ShowSkipCutsceneHint`, `_root.OnVideoStop`, `ShowFrontendIntroBG`)
  are ActionScript methods on `.gfx` movies (see [ui_hud_menus.md](ui_hud_menus.md)); video files resolve
  under `videos/`.

## Global manager-singleton registry (`FUN_146cd0bea`)

The narrative/media managers are registered (in order) into the global manager list, each paired with a
vtable pointer or a 32-bit id (proven — all read directly):

| Manager | vtable / id |
|---|---|
| `CVideoManager` | id `0x41d8e998` |
| `CCutsceneManager` | `PTR_LAB_141d906c8` |
| `CMediaRevolutionManager` | `PTR_LAB_141d90788` |
| `CVideoRecordingManager` | `PTR_LAB_141d90aa8` |

(Neighbors in the same table include `CActivityManager`, `CObjectiveManager`, `CQuestManager`,
`CDialogueCoordinator`, `CDialogueManager`, `CMissionManager` — see
[missions_progression.md](missions_progression.md) and [audio_dialogue.md](audio_dialogue.md).)

## Notable constants / tunables

- **Cutscene queue stride** `0x10`; active flag at `this+0x21`; state int at `this+0x14`; fade timer at
  `this+0x1c` (`FUN_140687e90`).
- **Cutscene disable flag** global `DAT_142cb8f39`; skip-hint global `DAT_142cb7dc0`; peer-manager gate
  `*(DAT_142cb7dd8+0x140)==2`.
- **Skip glyph** `"common_widgets.win64.cancel"` (KB/M) or resolved gamepad glyph (`FUN_140e2dac0`).
- **Fullscreen-video event** `"video.fullscreen.stop"`, priority `0xff`; surface object size `0x200`
  (`FUN_140e8fd50`).
- **Video path prefix** `"videos/"` (`FUN_140e9f2f0`); Scaleform hook `"_root.OnVideoStop"`.
- **YUV planes** `"video_Y"` (full), `"video_cR"`/`"video_cB"` (half res) — 4:2:0 (`FUN_14a8725f0`);
  in-world material `"video_projector_texture"` (`FUN_140cb4c80`).
- **Intro** boot points `"intro.boot.point.start"`, `"intro.boot.point.slumbridge"`; event
  `"intro.cutscene.location.load"` (sets `DAT_142cb8f50`); main-mission `"mm110"`; achievement
  `ach_quest_all_intros`; flag `m_IntroCompleted`.
- **Middleware** `"art.middleware.bink"` (+ havok/nvidia/rev/simplygon/speedtree) — `FUN_14a61a6f0`.

## Call-graph highlights

- `FUN_14086ed50` → `FUN_140687e90` (cutscene tick) → `thunk_FUN_14af0c550`/`14af0c7a0` (lock ctx),
  `FUN_140e0aef0` (HideSkipCutsceneHint), `FUN_140e225d0`.
- `FUN_1489981c0` / `FUN_140de44d0` → `FUN_140e2dac0` (Show), `FUN_140e37a60` (Update), `FUN_140e0aef0`
  (Hide) skip-hint.
- `FUN_140e969f0` → `FUN_140e9f2f0` (build `videos/<name>` + load via `thunk_FUN_14a8a2af0`).
- `FUN_140dc6770` → Scaleform `LoadVideo`/`UnloadVideo` → `FUN_140e8bd40`.
- `FUN_140e53e40` → `FUN_140eb2830("UIMediaRevolutionController")` + milestone-list iteration
  (`thunk_FUN_147c8c700`).
- `FUN_14086d1e0` (boot orchestrator) → `FUN_1408718a0` (intro boot points / `intro.cutscene.location.load`),
  `FUN_140e935b0` (IsIntroSequenceComplete).
- `FUN_146cd0bea` (`FUN_149b2f323` caller) registers all narrative/media manager singletons.

## Open questions / lower-confidence

- **Manager bodies walled.** `CVideoManager`, `CVideoRecordingManager`, `CMediaRevolutionManager`,
  `CContentIntroductionManager` update/schedule logic is behind data-section vtables — not in this
  functions-only export. Only registration skeletons + string tags recovered. (Methodology caveat.)
- **Bink decode call** not directly visible (behind middleware vtable). Bink is *proven present*
  (middleware table) and the YUV-plane upload strongly implies it (inferred), but the actual `.bk2`/Bink
  API call site isn't in the dump. No `.bk2`/`.bik` path literal was found — only the `videos/` prefix.
- **`cc*` type-hash pairing** (director/sequence/scenario ↔ 32-bit id) is offset-ambiguous in the table's
  write pattern → hashes inferred, extension strings proven.
- **Media Revolution scheduling** — how broadcasts are queued/timed in the world (the "news ticker"
  cadence) lives in the walled manager body; only the data model (milestone/threshold/reward) and UI
  push are recovered.
- **Recording feature scope** — whether `CVideoRecordingManager` is a console share-capture shim or a
  full in-game replay recorder can't be determined from the registration alone.

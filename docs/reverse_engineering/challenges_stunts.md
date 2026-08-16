# Challenges, stunts & feats — JC4's optional skill activities and stat-tracked feats

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)
**Decomp:** `output/_ghidra_jc4/jc4_all_functions_decomp.txt` (162,115 funcs)

## Overview

Just Cause 4 layers three related optional-activity systems on top of core gameplay:

1. **Challenges** — the challenge-ring / monthly-chaos activities, run by `CChallengeManager` with a
   gated unlock layer (`CChallengeUnlockController`) and a save-data record per challenge
   (`CChallengeManager::SChallengeSaveData`). Scoring is composed from a family of **score-multiplier**
   components (`CScoreMultiplier*`). (proven — registration strings present)
2. **Daredevil activities** — the *Daredevil* jumps/races: a point-scoring system (`CDaredevilPointManager`
   and a whole `CDaredevilPoint*` component family), a combo system (`CDaredevilCombo*`), tiered
   bronze/silver/gold medals (`CDaredevilMinBronzeStatSetter`, `CConditional_IsDaredevilRaceTierReached`),
   and race objects/settings (`CDaredevilRace*`). (proven — registration strings present)
3. **Just Run** — the traversal / foot-race challenge, run by `CJustRunManager`. (proven that the class
   exists and is instantiated; its scoring body is walled — see below)

Cutting across all three is a **feat/metric system**: a large enum of ~53 named metrics
(`tornado_wingsuit`, `wingsuit_altitude`, `rpg_missile_distance`, `melee_kill`, …) with paired
enum→string and string→enum lookup tables. Gameplay events increment these metrics through a metrics
event-bus, and tracked metrics are submitted to online **feat leaderboards**
(`osdk::jc4v6::FeatLeaderboardDTO`). (proven)

### Methodology caveat (walled boundary)

This is a **functions-only** export. What is recoverable here: the registration skeletons
(`FUN_140f27f60` typeid registrar, `FUN_14085fd00`/`FUN_140f215d0` factory paths), the class-name string
inventory, the metric enum↔string tables (fully inlined CPU logic), the metrics event-bus wiring, and the
leaderboard submission path. What is **NOT** recoverable: the per-component scoring bodies of
`CChallengeManager`, `CDaredevilPointManager`, `CJustRunManager`, and the `CScoreMultiplier*` /
`CDaredevilPoint*` components — these dispatch through **data-section vtables**
(`PTR_LAB_141da6088`, `PTR_LAB_141da60c0`, …) whose targets are not in the functions export. Star/medal
*threshold magnitudes* live in that walled data. Claims below are graded accordingly.

## Key classes & functions

### Challenge system

| Name string | FUN_ (registrar/site) | Role |
|---|---|---|
| `CChallengeManager` | typeid at `FUN_14081e2b0` (`0x874921`); factory name-getter `FUN_140904…` region | Top-level challenge activity manager (proven) |
| `CChallengeManager::SChallengeSaveData` | typeid at `0x858286`, `0x875009` | Per-challenge persisted save record (proven) |
| `std::vector<…SChallengeSaveData>` | typeid at `0x858246`, `0x874831` | The saved set of all challenge records (proven) |
| `CChallengeUnlockController` | `FUN_140845510` (`0x900487`) | Gates which challenges are unlocked/available (proven) |
| `CMonthlyChaosChallenge` | typeid at `FUN_14081e3b0`-neighbor (`0x874965`) | The rotating monthly chaos challenge (proven) |
| `CConditional_IsMonthlyChallengeAvailable` | `0x618685` | Behavior-VM condition: monthly challenge available (proven) |
| `CConditional_IsMonthlyChallengeComplete` | `0x618711` | Condition: monthly challenge complete (proven) |
| `CConditional_IsMonthlyChallengeRewardClaimed` | `0x618737` | Condition: reward claimed (proven) |
| `settings/monthly_challenges.bin` | `0x908411` (`thunk_FUN_14760e610`) | Config blob loaded for monthly challenges (proven) |
| `chaos_challenge_` | `0x906549` (`FUN_141afbf4c`, len 0x10) | String prefix for chaos-challenge ids (proven) |
| `challenge_safety_name` | `0x941974` (`FUN_140f27f60`, len 0x15) | Named challenge parameter (proven) |
| `sfx_gui_challenge_ring_condition_met` / `…_condtition_failed` [sic] | `0x588485`/`0x588519`, `0x904893`+ | Challenge-ring pass/fail UI SFX cues (proven) |

### Score multipliers (challenge/daredevil scoring building blocks)

All registered via `FUN_140f27f60` typeid; bodies walled behind vtables. (proven that they exist)

| Name string | FUN_ site | Role (inferred from name) |
|---|---|---|
| `CScoreMultiplier` | `0x868059` | Base score-multiplier component |
| `CScoreMultiplierDistance` | `0x868091` | Multiplier scaled by distance |
| `CScoreMultiplierEventCounter` | `0x868125` | Multiplier by repeated-event count (combo-like) |
| `CScoreMultiplierObjectHealth` | `0x868157` | Multiplier by target object health |
| `CScoreMultiplierObjectTracker` | `0x868189` | Multiplier tied to a tracked object |
| `CScoreMultiplierTime` | `0x868221` | Multiplier by elapsed/remaining time |

### Daredevil system

All registered via `FUN_140f27f60` typeid (registration cluster around `0x946899`–`0x947147` and
`0x900562`–`0x900612`); component bodies walled. (proven that they exist)

| Name string | FUN_ site | Role (inferred from name) |
|---|---|---|
| `CDaredevilPointManager` | typeid `0x925033`, `0x932229` | Manager for the daredevil point-scoring activity |
| `CDaredevilPointBase` | `0x946974` | Base daredevil-point award component |
| `CDaredevilPointId` | `0x947049` | Point identifier |
| `CDaredevilPointEvent` | `0x947024` | Points awarded on a game event |
| `CDaredevilPointCharacterKilled` | `0x946999` | Points for killing a character |
| `CDaredevilPointModelDestroyed` | `0x947074` | Points for destroying a model/prop |
| `CDaredevilPointVehicleDestroyed` | `0x947099` | Points for destroying a vehicle |
| `CDaredevilComboLevel` | `0x946949` | Current combo tier |
| `CDaredevilComboSettings` | `0x900562` | Combo tuning (decay window, tiers) |
| `CDaredevilGlobalSettings` | `0x900587` | Global daredevil tuning |
| `CDaredevilMinBronzeStatSetter` | `0x900612` | Sets the minimum stat = **bronze medal** threshold |
| `CDaredevilAnnouncement` | `0x946899` | Announcement/notification hook |
| `CDaredevilCheckpoint` | `0x946924` | Race checkpoint |
| `CDaredevilRaceObject` | `0x862489`, `0x947124` | A daredevil race instance |
| `CDaredevilRaceSettings` | `0x862521`, `0x947147` | Race tuning |
| `CObjectiveParam_DaredevilScoreAndTimer` | `0x899735` | Objective param: score + timer pair (the pass condition) |
| `CBookmarkDaredevilRace` | `0x900287` | Map bookmark for a daredevil race |
| `CConditional_HasBeatenDaredevilRacesByTier` | `0x605564` | Condition: N races beaten at a tier |
| `CConditional_IsDaredevilRaceTierReached` | `0x605940` | Condition: a medal tier reached |

`CDaredevilMinBronzeStatSetter` + `CConditional_IsDaredevilRaceTierReached` together prove a **tiered
medal model** (bronze is the floor; silver/gold above). The tier *magnitudes* are in walled config data.
(proven = tier model; inferred = bronze/silver/gold naming; walled = numeric thresholds)

**Daredevil-point object constructor** `FUN_14088f640` (`0x14088f640`) builds a point node: it calls
`FUN_140b6cef0(node, "daredevil_point", 2, …)` (`0x934877`) to register a property named `daredevil_point`,
then installs vtables `PTR_thunk_FUN_149110950_141da6088`, `PTR_LAB_141da60c0`, `PTR_LAB_141da60d8`,
`PTR_LAB_141da6140`. The scoring math is behind those vtables. (proven ctor; walled math)

### Just Run system

| Name string | FUN_ | Role |
|---|---|---|
| `CJustRunManager` | class-name getter `FUN_14a0563c0` (`0x14a0563c0`); callers `0x148359ebe`, `0x140cac730` (vtable slots) | Traversal / foot-race challenge manager (proven exists; logic walled) |
| `"JustRun"` | `0x4607948` (`param_1+0x10 = "JustRun"`) | Resource/label tag (proven) |

Note the getter `FUN_14a0563c0` is a trivial `*param_2 = "CJustRunManager"; return` — a vtable RTTI/name
slot. Its callers are vtable dispatch sites, so the manager's start/score/pass body is **walled**.
(proven boundary)

> **Disambiguation (not the challenge):** `"JUST_RUN"` at `0x1550841` is a *rendering debug-pass* enum
> label (neighbors: `WEATHER`, `ATMOSPHERIC_SCATTERING`, `REFLECTION_*`) — unrelated to the foot-race.
> Do not conflate. (proven)

## The feat / metric enum↔string tables (the core recoverable mechanism)

This is the fully-inlined heart of the feat/leaderboard stat system. A metric is an integer id in an enum;
two mirrored table families convert id↔name.

### Forward: metric id → name (a segmented `switch` chain)

Signature: `undefined8* f(undefined8 *out, int *id)` where `out[0]` = `char* name`, `out[1]` = `strlen`.
The chain falls through segment-to-segment for ids outside each segment's range:

- `FUN_149485cf0` (`0x149485cf0`) handles **0x03–0x0f**, else → `FUN_1494861d0`
- `FUN_1494861d0` (`0x1494861d0`) handles **0x10–0x1c**, else → `FUN_1494866f0`
- `FUN_1494866f0` (`0x1494866f0`) handles **0x1d–0x29**, else → `FUN_149485620`
- `FUN_149485620` (`0x149485620`) handles **0x2a–0x37**, else → `"unknown_value"`

**Complete metric table (proven, read directly from the four functions):**

| id | metric name | id | metric name |
|----|----|----|----|
| 0x03 | `flaming_vehicle` | 0x1e | `parachute_kill` |
| 0x04 | `drifter` | 0x1f | `reeled_in_kill` |
| 0x05 | `evil_knievel` | 0x20 | `kills_in_chopper` |
| 0x06 | `hit_run` | 0x21 | `drone_multikill` |
| 0x07 | `land_ho` | 0x22 | `multi_kill` |
| 0x08 | `land_vehicle_fast` | 0x23 | `melee_kill` |
| 0x09 | `unicyclist` | 0x24 | `no_miss` |
| 0x0a | `safe_driver` | 0x25 | `motorcycle_kill` |
| 0x0b | `any_fall_distance` | 0x26 | `&DAT_141dd5750` (len 4, walled — string not inlined) |
| 0x0c | `parasailing_distance` | 0x27 | `rpg_missile_distance` |
| 0x0d | `no_damage_wingsuit` | 0x28 | `kick_distance` |
| 0x0e | `parachute_climber` | 0x29 | `wind_gun_distance` |
| 0x0f | `longwingsuit` | 0x2a | `boost_distance` |
| 0x10 | `no_move_distance` | 0x2b | `lifter_fall` |
| 0x11 | `tornado_wingsuit` | 0x2c | `supply_drop_kill` |
| 0x12 | `wingsuit_altitude` | 0x2d | `no_weapons` |
| 0x13 | `ragdoll_time` | 0x2e | `kick_count` |
| 0x14 | `reel_distance` | 0x2f | `kill_distance` |
| 0x15 | `low_plane` | 0x30 | `shotgun_headshot` |
| 0x16 | `deep_sea_dive` | 0x31 | `iron_sights` |
| 0x17 | `sea_god` | 0x32 | `one_shot_one_kill` |
| 0x18 | `kills_in_boat` | 0x33 | `single_clip` |
| 0x19 | `sea_vehicle_fast` | 0x34 | `fire_in_the_hole` |
| 0x1a | `land_ho_speed_demon` | 0x35 | `one_missile_destruction` |
| 0x1b | `land_meets_sea` | 0x36 | `one_missile_kill` |
| 0x1c | `jetski_airborne` | 0x37 | `machine_gunner_headshot` |
| 0x1d | `sailboat_speed_demon` | | |

This matches the two feat ids the other digs surfaced exactly: **`tornado_wingsuit` = 0x11**,
**`wind_gun_distance` = 0x29**. (proven)

Ids **0x00–0x02** are not in this chain; the caller that builds leaderboard keys treats id `1` = `invalid`
and id `2` = `most_wanted` before dispatching id≥3 into the chain (see below). (proven at `0x1189429`)

Id 0x26 is a 4-character metric whose literal lives at `DAT_141dd5750` (data section, not inlined) — its
text is walled. (proven walled)

### Reverse: name → metric id (prefiltered hash-of-switches)

The inverse tables (`FUN_14948aa80` `0x14948aa80`, `FUN_14948ade0` `0x14948ade0`, `FUN_14948cb00`
`0x14948cb00`, plus siblings) take a `{char* str, size_t len}` pair and return `{bool found, int id}`.
Each candidate is prefiltered on **(length, first char)** then confirmed with a recursive char-by-char
compare:
- `FUN_14956e250` (`0x14956e250`) — **case-sensitive** recursive string-equality
  (`str1[i]==str2[i] && recurse(i+1)`).
- `FUN_14956e670` (`0x14956e670`) — **case-insensitive** variant, lowercasing each char via
  `thunk_FUN_1496690a0` (tolower) before compare. A second reverse table (near `0x4279xxx`) uses this
  variant, so metric names parse case-insensitively from config/text. (proven)

Example (from `FUN_14948aa80`): `len==0x11 && str[0]=='w'` → compare `"wind_gun_distance"` →
returns id `0x29`; `len==0xe && str[0]=='b'` → `"boost_distance"` → `0x2a`; etc. (proven, `0x14948aa80`)

## How feats reach leaderboards (the increment → submit path)

### Metrics event-bus wiring

A metrics/stats manager registers a large set of event handlers, each binding a gameplay event to a
metric/stat sink. Seen inline in the registration function around `0x1240232`–`0x1240249`:
`PTR_s_metrics_on_tacked_leaderboard_ch_142ae2368` (the **tracked-leaderboard-changed** handler),
alongside `metrics_on_encounter_spawn`, `metrics_on_encounter_experienced`, `metrics_on_quest_start`,
`metrics_on_quest_status_changed`, `stat_chaos_milestone_reached`, `on.character.grappled`,
`on.character.reelkicked`, `metrics_on_supply_object_spawned`. (proven) This is the bus that turns
gameplay events into metric increments and flags a leaderboard as "tracked/changed".

The `metrics_on_tacked_leaderboard_ch` pointer is also referenced at `0x1665139` and `0x4763055`
(`thunk_FUN_148d4e3a0(...)`), i.e. the tracked-leaderboard event is consumed in multiple sites. (proven)

### Daredevil-race pass metric

`FUN_140f2…` region at `0x417095`–`0x417139` shows a concrete increment+emit: it bumps a counter
(`*(param_1+0x10c)++`), and when the counter passes a stored best (`param_1+0x110`) **or** a ~1000-unit
time window elapses against a global timer (`DAT_142c846b0+0x60` vs `param_1+0x114`), it emits the metric
**`daredevils_race_passed_car`** (registered once via `thunk_FUN_147628c00(&DAT_142cb2900, …)` at
`0x417125`) and dispatches it. This is the "daredevil race passed (in a car)" feat firing. (proven
control flow; the distance-gate constant is `DAT_141d083fc`, a squared-distance threshold, walled value)

### Leaderboard key construction & submission

The metric **name** is used to build the online leaderboard identifier. In the key-builder around
`0x1189420`–`0x1189446`: id `1`→`"invalid"`, id `2`→`"most_wanted"`, else the metric enum
(`thunk_FUN_149485cf0`) supplies `{name,len}`, which is concatenated with a per-region string
(`lVar2+0x1890`) to form the submission key. (proven)

Daredevil ranks use a dedicated key: `FUN_141afbf4c(dst, "daredevil_race.rank_", 0x14)` then appends the
tier index (`param_2+1`) — i.e. `daredevil_race.rank_<tier>` leaderboard keys. (proven, `0x940749`)

The online layer is Avalanche's **osdk** ("jc4v6") stack: `osdk::jc4v6::FeatLeaderboardDTO` is the
leaderboard row DTO, parsed from JSON via `osJSONObjectConverter<…FeatLeaderboardDTO>::FromJSON`, keyed by
`osdk_JSONObject_find(obj,"leaderboard",0xb)` (`0x1159364`, `0x1178667`). Related online calls:
`GetLeaderboardsForFriends` (`0x1194475`), `LeaderboardNotification` (`0x1377075`),
`local_leaderboard_data` (`0x1181231`), `reload_leaderboard_config` (`0x1181307`). (proven)
See also `online_platform_social.md` for the osdk transport. (cross-ref)

## Data & config integration

- Challenges/daredevil components are **RTPC/ADF entity components** registered by `ArGetTypeId<Class>`
  through `FUN_140f27f60` (the typeid registrar) and instantiated through the standard factory
  (`FUN_14085fd00` cluster, `FUN_140845…` name-getters). Map each `C*` class name here to its entity
  component hash via `[[rtpc-entity-assembly]]` / `docs/formats/adf.md`. (inferred — same pattern as other
  systems in this dir)
- Monthly challenges load a dedicated config: `settings/monthly_challenges.bin` (`0x908411`). (proven)
- Challenge state persists via `CChallengeManager::SChallengeSaveData` records held in a
  `std::vector<SChallengeSaveData>` (typeid-registered for save serialization, `0x858246`/`0x858286`).
  (proven the save type exists; field layout walled)
- Daredevil medal tiers are set from config by `CDaredevilMinBronzeStatSetter` /
  `CDaredevilGlobalSettings` / `CDaredevilComboSettings`; `CObjectiveParam_DaredevilScoreAndTimer` is the
  score+time pass criterion. (proven classes; numeric thresholds walled)

## Notable constants / tunables

| Constant | Where | Meaning |
|---|---|---|
| metric ids `0x03`–`0x37` | `FUN_149485cf0`/`1494861d0`/`1494866f0`/`149485620` | The full feat/stat metric enum (proven) |
| `0x11` = `tornado_wingsuit`, `0x29` = `wind_gun_distance` | as above | Confirms the two ids from prior digs (proven) |
| `1`=`invalid`, `2`=`most_wanted` | `0x1189429` | Reserved leaderboard-key ids below the metric enum (proven) |
| `1000` (time window), `DAT_141d083fc` (sq-distance gate) | `0x417097`, `0x417112` | Daredevil-race pass emit gating (proven flow; DAT value walled) |
| `daredevil_race.rank_<tier>` | `0x940749` | Per-tier daredevil race leaderboard key (proven) |
| star/medal *magnitudes* | walled (data-section vtables) | NOT in functions export |

## Call-graph highlights

- Metric enum forward chain: `FUN_149485cf0 → FUN_1494861d0 → FUN_1494866f0 → FUN_149485620`
  (fall-through by id range). Callers build leaderboard/metric name strings (`0x1189442`, `0x1189637`,
  `0x1193845`, `0x4274024`+). (proven)
- Metric reverse chain: `FUN_14948aa80`/`FUN_14948ade0`/`FUN_14948cb00` → confirm via `FUN_14956e250`
  (case-sensitive) / `FUN_14956e670` (case-insensitive). (proven)
- Daredevil point node ctor `FUN_14088f640` → `FUN_140b6cef0("daredevil_point",2)` → vtables
  `PTR_LAB_141da6088/60c0/60d8/6140` (walled bodies). (proven edge; walled targets)
- Metrics bus registrar (`~0x1240232`) binds `metrics_on_tacked_leaderboard_ch` + quest/encounter/chaos
  metric handlers. (proven)
- `CJustRunManager` name-getter `FUN_14a0563c0` ← vtable callers `0x148359ebe`, `0x140cac730` (walled
  manager body). (proven boundary)

## Open questions / lower-confidence

- **Star/medal threshold magnitudes** for challenges and the bronze/silver/gold cutoffs for daredevil
  races are in the data-section (vtable-dispatched component config) and are **not** in the functions
  export. Recover from the RTPC/ADF config side (entity component payloads), not from this dump. (walled)
- Metric id `0x26`'s 4-char name (`DAT_141dd5750`) is not inlined — resolve from the data section.
- Exact `CChallengeManager::SChallengeSaveData` field layout (per-challenge best score, stars earned,
  completion flags) is walled; infer from the save-file/ADF schema.
- Whether `CScoreMultiplier*` compose additively or multiplicatively into a final challenge score is not
  visible (bodies walled); the class names imply a stacked-multiplier model. (speculative)
- The `CJustRunManager` scoring/pass logic (distance/time thresholds for the foot-race) is fully walled
  behind its vtable; only the class + resource tag are proven present.

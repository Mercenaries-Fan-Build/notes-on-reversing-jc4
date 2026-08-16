# Audio & dialogue — FMOD sound system, radio/music, dialogue chains, vocals/barks, 3D occlusion

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

> "proven" = read directly from the decomp (`output/_ghidra_jc4/jc4_all_functions_decomp.txt`).
> All `FUN_`/`DAT_`/`PTR_LAB_` addresses below are from that dump. Names in `"quotes"` are the engine's own
> C++ class strings / asset paths / event-cue strings that survive as string constants.
>
> **Methodology caveat (project mandate):** this is a *functions-only* export. Recoverable here: the manager
> registration skeletons, the FMOD bank/asset plumbing, the string-named event bus, the class/component
> inventory, and a large set of literal event/cue/parameter names. **Not** recoverable: the per-frame audio
> tick bodies, the FMOD Studio API call sites (behind data-section function pointers such as `DAT_141c901e8`),
> and the tunable mix magnitudes — these live in FMOD banks, `.vocals_settingsc`/RTPC data, and vtables. Those
> are tagged **walled** below.

## Overview

Just Cause 4's audio is built on **FMOD** middleware (proven — `"Audio_FMOD"` `FUN_147623xxx`; banks
`sound/fmod_banks/*.fmod_bankc` loaded in `FUN_14017ca20`; three custom FMOD DSP plugins compiled from
`...\audiodsps\kg{amplitudemodulator,compressor,reverb}fmod.cpp`). There is **no Wwise/GameSync** presence
(proven — zero `wwise|ak::|audiokinetic|.bnk` matches in the dump). The audio stack decomposes into five
engine-tagged budgets, read straight from the profiler-zone name table `FUN_147623xxx` (proven,
lines 3321668–3321678): **`Audio` / `Audio_FMOD` / `Audio_Vocals` / `Audio_SoundBs` (sound banks) /
`Audio_SoundFiles` / `Audio_Occlusion`**.

Structurally it is the same three-layer pattern as the other JC4 systems (see `README.md`):

1. **World subsystems (managers).** One core `CSoundSystem` plus a set of secondary managers —
   `CDialogueCoordinator`, `CDialogueManager`, `CRadioSystem`, `CVocalsManager` — are installed by the global
   world-manager registrar `FUN_148f960c0` (proven; mirror copy in `FUN_146cd0bea`). `CSoundSystem` is the
   very first manager registered and the only one given a class-tag (`0x41cc6ff0`); the rest are second-tier
   managers installed through `thunk_FUN_148f6e780`.
2. **RTPC entity components (placed audio objects).** Concrete audio is authored as entity components,
   each registered by name-hash through `FUN_140f27f60("<Name>", <len>)`: the six sound-occlusion volume
   shapes, `CAudioCaveOcclusionPlane`, `CSoundLayerEmitter`, `CSoundAcousticsSettings`, `CListenerEffects`,
   `CAmbientEffects`, `CRadioStation`, `CVehicleRadio`, `CMusicTrack`, `CMusicProgressSetter`,
   `CVocalsLayer`, the `CDialogue*` family, `CTriggerBarkWeaponComponent`, `CChopShopVocals` (all proven).
3. **String-named event bus (the glue).** Gameplay talks to audio by **firing lowercased dotted event
   names** — `"player.near.tornado"`, `"on.lightning.strike.target"`, `"vocals.rico.disable"`,
   `"sfx_gui_general_open_radio"` — through `thunk_FUN_147bfeb20(name)` (fire) and binding them with
   `FUN_14762d8a0(&slot, name, prio, flag)` (subscribe). This is a **generic message bus shared with the rest
   of the game**, not an audio-only channel; audio is one subscriber (proven).

## Middleware identification — FMOD (proven)

| Evidence | Where |
|---|---|
| Profiler zone `"Audio_FMOD"` (enum 0x3f) in the budget-name switch | `FUN_147623xxx` line 3321670 |
| FMOD banks loaded: `sound/fmod_banks/master.fmod_bankc`, `…/master_strings.fmod_bankc` | `FUN_14017ca20` @0x14017ca20 (lines 101329/101338) |
| Streaming banks: `…/streaming_zones.fmod_sbankc`, `…/streaming_front_end.fmod_sbankc` | line 137573; `FUN_140857xxx` line 911429 |
| Custom FMOD DSP plugins built in-house (`KGAmplitudeModulator`, `KGCompressor`, `KGReverb`) | source paths `…\audiodsps\kg*fmod.cpp` lines 4794598/4795504/4796913 |
| Asset-type registrations `.fmod_bankc`→id 0x31, `.fmod_sbankc`→0x32, `.wavc`→0x2f, `.vocalsc`/`.vocals_settingsc`→0x2d, `.stringlookup`→0x2c | resource-type table `FUN_147ea7ee0` lines 3648369–3648388 |

The two `master*.fmod_bankc` banks match FMOD Studio's canonical **master bank + master.strings bank** pairing
(inferred from the naming; `master_strings` is FMOD's GUID→event-path lookup bank). The `.stringlookup`
resource type (0x2c) is the engine-side companion to that (inferred).

## Key classes & functions

### World subsystems (managers) — installed by `FUN_148f960c0` @0x148f960c0 (size 4805; mirror `FUN_146cd0bea`)
| Class string | Install mechanism | Descriptor / tag | Role |
|---|---|---|---|
| `CSoundSystem` | first manager; direct install with class-tag | tag `0x41cc6ff0`, vtable `PTR_LAB_141d90448` | Core audio subsystem / FMOD owner (proven reg; role inferred) |
| `CDialogueCoordinator` | `thunk_FUN_148f6e780(world,"CDialogueCoordinator",desc)` line 4138957 | `PTR_LAB_141d90868` | Arbitrates which dialogue plays (proven reg; role inferred) |
| `CDialogueManager` | `thunk_FUN_148f6e780(…,"CDialogueManager",…)` line 4138997 | `PTR_LAB_141d90988` | Owns dialogue chains/lines (proven reg; role inferred) |
| `CRadioSystem` | `thunk_FUN_148f6e780(…,"CRadioSystem",…)` line 4139002 | `PTR_LAB_141d909a8` | Radio station playback (proven reg; role inferred) |
| `CVocalsManager` | `thunk_FUN_148f6e780(…,"CVocalsManager",…)` line 4139087 | `PTR_LAB_141d90bc8` | Character vocal/bark playback (proven reg; role inferred) |

`CSoundSystem` singleton pointer is `DAT_142cb1068`, returned by the 8-byte getter `FUN_147e98980` @0x147e98980
(proven).

### RTPC entity components (placed audio objects) — each `FUN_140f27f60("<Name>",len)` → name-hash slot
| Class string (len) | Name-hash slot (DAT) | Registrar `FUN_` | Role |
|---|---|---|---|
| `CSoundOcclusionBase` (0x13) | `_DAT_142cb0bdc` | `FUN_1402aaf10` @0x1402aaf10 | Base sound-occlusion volume (proven) |
| `CSoundOcclusionBox` (0x12) | `DAT_142cb0bf4` | `FUN_1402aaf10` | Box occlusion volume (proven) |
| `CSoundOcclusionCylinder` (0x17) | `DAT_142cb0c04` | `FUN_1402aaf10` | Cylinder occlusion volume (proven) |
| `CSoundOcclusionDisc` (0x13) | `DAT_142cb0bfc` | `FUN_1402aaf10` | Disc occlusion volume (proven) |
| `CSoundOcclusionPlane` (0x14) | `DAT_142cb0bec` | `FUN_1402aaf10` | Plane occlusion volume (proven) |
| `CSoundOcclusionDynamicScale` (0x1b) | `_DAT_142cb0be4` | `FUN_1402aaf10` | Runtime-scaled occlusion volume (proven) |
| `CAudioCaveOcclusionPlane` (0x18) | `DAT_142cb8888` | `FUN_140794xxx` line 803819 | Cave-mouth occlusion plane (proven) |
| `CSoundLayerEmitter` (0x12) | `DAT_142cb8d00` | line 840679 | Layered ambient sound emitter (proven reg; role inferred) |
| `CSoundAcousticsSettings` (0x17) | `DAT_142cb8e4c` | line 840656 | Per-area acoustics/reverb settings (proven reg; role inferred) |
| `CListenerEffects` (0x10) | `DAT_142cb8d10` | line 840171 | Effects tied to the audio listener (proven reg; role inferred) |
| `CAmbientEffects` (0xf) | `DAT_142cb8870` | line 803773 | Ambient soundscape component (proven reg; role inferred) |
| `CRadioStation` (0xd) | `DAT_142cb0bc8` | `FUN_1402aaf10` line 198747 | A radio station definition (proven) |
| `CVehicleRadio` (0xd) | `DAT_142cb9b48` | line 870141 | Per-vehicle radio component (proven) |
| `CMusicTrack` (0xb) | `DAT_142cb8cf8` | line 840357 | A music track (proven) |
| `CMusicProgressSetter` (0x14) | `DAT_142cb987c` | line 865755 | Drives music-state progression (proven reg; role inferred) |
| `CVocalsLayer` (0xc) | `DAT_142cb8ef8` | `FUN_1407dc330` @0x1407dc330 | Vocal-layer component on a character (proven) |
| `CDialogueChain` (0xe) | `DAT_142cb7e70` | `FUN_1406b7320` @0x1406b7320 | A sequence of dialogue lines (proven) |
| `CDialogueLine` (0xd) | `DAT_142cb7e80` | line 675451 | A single spoken line (proven) |
| `CDialogue` (0x9) | `_DAT_142cb0b88` | line 199183 | Dialogue root component (proven) |
| `CDialogueSettings` (0x11) | `_DAT_142cb0b90` | line 199208 | Dialogue tuning/config (proven) |
| `CUIDialogue` (0xb) | `_DAT_142cb0b98` | line 199658 | UI-driven dialogue (proven) |
| `CTriggerBarkWeaponComponent` (0x1b) | `DAT_142cb83cc` | `FUN_140744b20` @0x140744b20 | Fires a weapon-related bark (proven reg; role inferred) |
| `CChopShopVocals` (0xf) | `DAT_142cb88b0` | line 803911 | Chop-shop vocal set (proven reg; role inferred) |

`CRicoVocals` / `ChopShopVocals` also appear as bare string constants (proven) but were not seen going through
`FUN_140f27f60` in the export — likely referenced from a walled body.

### Behavior/quest condition
| Class string | Name-hash slot | Mapped by | Role |
|---|---|---|---|
| `CConditional_IsRadioStationActive` (0x21) | `DAT_142cb5ec4` (line 627477) | `thunk_FUN_147cafcd0(…)` line 4140317 (conditions registrar) | Gates logic on whether a radio station is active (proven) |

## How it works (from the decomp)

### FMOD bank system init — `FUN_14017ca20` @0x14017ca20 (size 1872)
On audio-system bring-up this function loads the two core banks via the bank loader
`FUN_14017d470` @0x14017d470 (size 502): first `sound/fmod_banks/master.fmod_bankc` (stored at `+0xB0`), then
`sound/fmod_banks/master_strings.fmod_bankc` (stored at `+0xB8`) (proven, lines 101329/101338). The streaming
banks `streaming_zones.fmod_sbankc` and `streaming_front_end.fmod_sbankc` are loaded through the same loader
from other call sites (`FUN_14017d470` callers include `FUN_1401ebda0` and `FUN_1408572c0`; proven). The
actual FMOD Studio `loadBankFile`/`getEvent` calls inside `FUN_14017d470` resolve through data-section
pointers — **walled** (magnitudes/handles not in the export).

### The string-named event bus (how `"player.near.tornado"` reaches audio)
This is the single most important recoverable mechanism, and it is **generic** — audio, weather and gameplay
all ride it (proven):

- **Resolve/tokenize a name** — `FUN_147625370` @0x147625370 (size 213): walks the string, `tolower`s every
  char, splits on spaces, and interns each dotted token via `thunk_FUN_14cfe1310`/`thunk_FUN_14cfddc70`,
  returning a token count. So `"player.near.tornado"` is normalized+hashed to a token vector (proven).
- **Fire an event by name** — `FUN_147bfeb20` @0x147bfeb20 (size 193): zeroes a 512-byte scratch, calls
  `FUN_147625370(name,buf,0x40)`, resolves the event object with `thunk_FUN_14761f540(buf,tokens)`, then
  dispatches with `thunk_FUN_14762ba00(evt,&prio)` where `prio = 0xff` (proven). This is what posts
  `"vocals.rico.disable"` (`FUN_140bb3180` line 4436829) and every `"sfx_gui_*"` UI cue
  (e.g. `"sfx_gui_general_open_radio"` line 1714033).
- **Bind/subscribe to an event by name** — `FUN_14762d8a0` @0x14762d8a0 (size 526): parses the same
  lowercased/tokenized name and stores a binding object into `*param_1`, with a priority/type code in
  `param_3`. Weather entities use it, e.g. `FUN_14762d8a0(this+0xD0,"player.near.tornado",0xff,1)` in
  `FUN_149843a50` @0x149843a50, and `FUN_14762d8a0(this+0x30,"on.lightning.strike.target",0x106,1)` in
  `FUN_149139c10` @0x149139c10 (proven).

So the pipeline is: **gameplay fires a lowercased dotted event → tokenized+hashed → dispatched on the shared
bus → audio (and other) subscribers registered via `FUN_14762d8a0` react.** The audio *response* (which FMOD
event actually plays) is chosen inside walled subscriber bodies / FMOD bank data.

### Dialogue system — coordinator → chains → lines
`CDialogueManager` owns `CDialogueChain`s, each an ordered set of `CDialogueLine`s; `CDialogueCoordinator`
arbitrates which chain plays (proven registrations; role inferred from names + the trigger below).
Chains are **named, gender-variant conversation assets** triggered at gameplay moments and selected by
name-hash. Proven example in `FUN_1408dc…` (line 984033): at fast-travel the code branches on a character
gender field (`*(int*)(actor+0x270)`: `1`→male, `2`→female) and instantiates
`FUN_140f27f60("CDialogueChain - Fast Travel Male",0x21)` or `"... Fast Travel Female"` (0x23), then compares
it against the running chain's type-id (`thunk_FUN_147c9b5c0`) and triggers it (`thunk_FUN_1489c0510`). The
supply-drop path does the same with `"CDialogueChain - Supply Drop Male/Female"` (proven, `FUN_140b57…`
lines 1428336/1428341). Voice-line audio resolves through the path template
**`sound/dialogue/eng/%s.wavc`** (proven, `thunk_FUN_14aadc440(...,"sound/dialogue/eng/%s.wavc",name)` at
lines 3985524/4719109/4794321 — note the hard-coded language folder `eng` and one call carrying the constant
hash `0xd327d9be`). The `.wavc` container is resource type-id `0x2f` (proven).

### Vocals & barks — `CVocalsManager` / `CVocalsLayer`
Character utterances ("vocals") are layered onto characters via `CVocalsLayer` and toggled by the event bus:
`"vocals.rico.enable"` / `"vocals.rico.disable"` are fired through `thunk_FUN_147bfeb20` (proven,
`FUN_140bb3180` line 4436829; string table at `FUN_140bb3180` line 1391659). Player **barks** are a fixed cue
set built in `FUN_140b9e080` @0x140b9e080 (size 656): it lays out a 12-entry table
`bark_rico_jump_010..060` and `bark_rico_stagger_010..060`, then for each name formats a 0x200 buffer
(`thunk_FUN_14a623ea0`, format `&DAT_141ca7710`) and creates a 0x48-byte sound-request object via
`thunk_FUN_14760d680(obj, DAT_142cb1068 /*sound system*/, name, 0)` (proven). The `010..060` suffixes are
variant indices (inferred). `CTriggerBarkWeaponComponent` ties barks to weapon events (proven reg; behavior
walled). Vocals config is the ADF asset `sound/vocals_settings.vocals_settingsc` (type-id `0x2d`), loaded in
`FUN_140317620` (proven, line 3649358); its tunables are **walled** (data-side).

### Radio & music
`CRadioSystem` + `CRadioStation`/`CVehicleRadio` implement radio; `CConditional_IsRadioStationActive` lets
behavior/quest logic gate on radio state (proven). Radio is driven by named cues on the event bus:
`"sfx_gui_general_open_radio"`, `"sfx_gui_general_close_radio"`, `"sfx_gui_general_switch_radio_station"`,
`"forward_time_radio"`, `"on_player_death_radio"`, and the RTPC/parameter `"SoundCarRadioVolume"` (all proven
string constants). **Music** is a small state machine over named states **`"music.frontend"`** and
**`"music.world"`**: `FUN_140b84620` @0x140b84620 builds a table of these via `thunk_FUN_147cad3f0(slot,name)`
alongside the streaming-bank setup (proven). `CMusicTrack`/`CMusicProgressSetter` are the placed music
components; the progression logic itself is walled.

### 3D positioning & occlusion
Occlusion is authored as six volume-shape components (`CSoundOcclusion{Base,Box,Cylinder,Disc,Plane,
DynamicScale}`) plus `CAudioCaveOcclusionPlane` (proven registrations). The runtime uses an audio **listener**
with a terrain-relative height and an occlusion **raycast**: debug/tuning handles `"LISTENER_RAY"`
(`FUN_1407cd630` @0x1407cd630, line 833142) and `"Listener height over terrain"` (`FUN_1430d1610`, line
3118603) are registered as named debug vars via `FUN_1400d07c0`; there is also a `"LISTENER_EFFECTS"` debug
grid overlay (line 38409) and the `CListenerEffects` component (proven). The occlusion geometry test math and
the FMOD occlusion parameter it feeds are **walled** (not in the export; budgeted under `Audio_Occlusion`).

### Settings → mix
The options menu exposes four volume categories as named audio parameters: **`SoundGameplayVolume`**,
**`SoundMusicVolume`**, **`SoundVoiceVolume`**, **`SoundCarRadioVolume`** (proven). They are packed into a
settings record by `FUN_140b4e090`/`FUN_140b4e210` @0x140b4e090/0x140b4e210 (`(name,value)` → record at
`+0x1028` via `FUN_140b78060`) inside the audio-settings builder `FUN_140bb3a00` @0x140bb3a00 (proven, lines
1392104–1392106), and read back / applied through the function-pointer `DAT_141c901e8(ctx,"SoundMusicVolume")`
in `FUN_140b964b0` (proven, lines 1377658–1377678). The final apply to FMOD buses is behind `DAT_141c901e8` —
**walled**.

## Data & config integration

- Audio components are RTPC entity components: each `C*` class here is registered by **`FUN_140f27f60(name,
  len)`**, i.e. the cracked **lookup3 `hashlittle`** name-hash (`docs/formats/name_hash.md`,
  `[[name-hash-cracked]]`). The stored `DAT_*` slot is that class's hash id — the same hash that keys the
  entity-component factory in `[[rtpc-entity-assembly]]`/`[[composite-assets]]`. Tie-out to concrete entity
  component hashes is the natural next-pass task.
- Audio assets are their own ADF/resource types: `.fmod_bankc`(0x31), `.fmod_sbankc`(0x32), `.wavc`(0x2f),
  `.vocalsc`/`.vocals_settingsc`(0x2d), `.stringlookup`(0x2c) — registered in `FUN_147ea7ee0` (proven). These
  map to the archive assets `sound/fmod_banks/*`, `sound/dialogue/eng/*.wavc`, `sound/vocals_settings.*`.
- Tunables (occlusion factors, bark cooldowns, mix curves, music transitions) live in the FMOD banks and the
  `.vocals_settingsc`/`CDialogueSettings`/`CSoundAcousticsSettings` data — **not in code** (consistent with
  the project-wide "mechanism in exe, magnitudes in data" finding).

## Notable constants / event & cue names (verbatim, with FUN_)

| Kind | Values | Where |
|---|---|---|
| FMOD banks | `sound/fmod_banks/master.fmod_bankc`, `…/master_strings.fmod_bankc`, `…/streaming_zones.fmod_sbankc`, `…/streaming_front_end.fmod_sbankc` | `FUN_14017ca20`, lines 137573/911429 |
| Audio budgets | `Audio`, `Audio_FMOD`, `Audio_Vocals`, `Audio_SoundBs`, `Audio_SoundFiles`, `Audio_Occlusion` (enum 0x3e–0x43) | `FUN_147623xxx` lines 3321668–3321678 |
| Custom DSPs | `KGAmplitudeModulator`, `KGCompressor`, `KGReverb` (FMOD plugins) | source paths lines 4794598/4795504/4796913 |
| `CSoundSystem` class tag | `0x41cc6ff0` | `FUN_148f960c0` |
| Weather→audio events | `"player.near.tornado"`, `"player.near.sandstorm"`, `"on.lightning.strike.target"` | fire `FUN_140b344d0` l.1327106; bind `FUN_149843a50` l.4380345 / `FUN_149139c10` l.4165760 |
| Vocals events | `"vocals.rico.enable"`, `"vocals.rico.disable"` | `FUN_140bb3180` l.1391659/4436829 |
| Player barks | `bark_rico_jump_010..060`, `bark_rico_stagger_010..060` (12 cues) | `FUN_140b9e080` l.1380845 |
| Music states | `"music.frontend"`, `"music.world"` | `FUN_140b84620` l.1367937 |
| Radio cues | `"sfx_gui_general_open_radio"`, `"…close_radio"`, `"…switch_radio_station"`, `"forward_time_radio"`, `"on_player_death_radio"` | string table; fire `FUN_147bfeb20` l.1714033 |
| Volume params | `"SoundGameplayVolume"`, `"SoundMusicVolume"`, `"SoundVoiceVolume"`, `"SoundCarRadioVolume"` | `FUN_140bb3a00` l.1392104; `FUN_140b964b0` l.1377658 |
| GUI SFX cues | large `sfx_gui_*` family (challenge timers, DLC race/menu ducking, garage, feats, …) | string constants throughout |
| Dialogue chains | `"CDialogueChain - Fast Travel Male/Female"`, `"… Supply Drop Male/Female"` | l.984033/984038, l.1428336/1428341 |
| Dialogue voice path | `sound/dialogue/eng/%s.wavc` (+ `%s/sound/dialogue/eng/%s.wavc`); hash `0xd327d9be` | `thunk_FUN_14aadc440` l.3985524/4719109/4794321 |
| Vocals config asset | `sound/vocals_settings.vocals_settingsc` | `FUN_140317620` l.3649358 |
| Occlusion debug vars | `"LISTENER_RAY"`, `"Listener height over terrain"`, `"LISTENER_EFFECTS"` grid | `FUN_1407cd630` l.833142; `FUN_1430d1610` l.3118603; l.38409 |

## Call-graph highlights

- `FUN_148f960c0` (world-manager registrar) → installs `CSoundSystem` (tag `0x41cc6ff0`) + `thunk_FUN_148f6e780`
  installs `CDialogueCoordinator`/`CDialogueManager`/`CRadioSystem`/`CVocalsManager`.
- `FUN_147e98980` → `DAT_142cb1068` (CSoundSystem singleton), consumed by the bark builder `FUN_140b9e080`.
- Event fire: `FUN_147bfeb20` → `FUN_147625370` (tokenize/lower/hash) → `thunk_FUN_14761f540` (resolve) →
  `thunk_FUN_14762ba00` (dispatch, prio 0xff).
- Event bind: `FUN_14762d8a0` ← weather bodies `FUN_149843a50`, `FUN_149139c10`.
- FMOD banks: `FUN_14017ca20` → `FUN_14017d470` (loader) ← also `FUN_1401ebda0`, `FUN_1408572c0`.
- Dialogue trigger: fast-travel body (l.984033) → `FUN_140f27f60("CDialogueChain - Fast Travel *")` →
  `thunk_FUN_147c9b5c0` (get chain type-id) → `thunk_FUN_1489c0510` (trigger).
- Settings: `FUN_140bb3a00` → `FUN_140b4e090`/`FUN_140b4e210` (pack volume params) → `FUN_140b78060`;
  applied in `FUN_140b964b0` via `DAT_141c901e8`.

## Open questions / lower-confidence (walled)

- **FMOD Studio API surface** — the actual `loadBankFile`/`getEvent`/`EventInstance::start`/`setParameter`
  calls resolve through data-section pointers (`DAT_141c901e8`, the loader internals of `FUN_14017d470`) and
  are not in a functions-only export. **(walled)**
- **Audio RTPC parameters** — beyond the four volume params + `SoundCarRadioVolume`, the per-emitter RTPC set
  (distance, occlusion, surface, engine RPM, etc.) is FMOD-bank-side. The task brief's "audio RTPC" distinct
  from entity RTPC is confirmed conceptually (these are FMOD parameter names carried as strings) but the full
  list is walled. **(open)**
- **Occlusion math** — the shape/ray intersection and the FMOD occlusion value it produces are not in the
  export; only the volume component registrations + debug knobs survive. **(walled)**
- **Dialogue coordination logic** — `CDialogueCoordinator`'s arbitration/priority rules, `CDialogueChain`
  sequencing, and interruption behavior are behind vtables. Only registration + one trigger pattern
  (fast-travel/supply-drop, gender-selected) are proven. **(open)**
- **Component→entity-hash tie-out** — each audio component's `DAT_*` name-hash slot should be matched to its
  entity component hash in `[[rtpc-entity-assembly]]` to read authored values. **(next pass)**
- **Localization** — only `eng` is hard-coded in the voice path seen; other language folders presumably exist
  in data. **(speculative)**

## Appendix — decomp anchors (re-verifiable)

Managers: `CSoundSystem` (l.4138536, tag `0x41cc6ff0`, vtable `PTR_LAB_141d90448`), `CDialogueCoordinator`
(l.4138957, `PTR_LAB_141d90868`), `CDialogueManager` (l.4138997, `PTR_LAB_141d90988`), `CRadioSystem`
(l.4139002, `PTR_LAB_141d909a8`), `CVocalsManager` (l.4139087, `PTR_LAB_141d90bc8`) — all in
`FUN_148f960c0` @0x148f960c0 (mirror `FUN_146cd0bea` @0x146cd0bea). Singleton getter `FUN_147e98980`
@0x147e98980 → `DAT_142cb1068`.

Components (registrar `FUN_140f27f60` name-hash): occlusion set `FUN_1402aaf10` @0x1402aaf10 (l.198747–198889);
`CAudioCaveOcclusionPlane` l.803819; `CSoundLayerEmitter` l.840679; `CSoundAcousticsSettings` l.840656;
`CListenerEffects` l.840171; `CAmbientEffects` l.803773; `CVehicleRadio` l.870141; `CMusicTrack` l.840357;
`CMusicProgressSetter` l.865755; `CVocalsLayer` `FUN_1407dc330` (l.841072); `CDialogueChain` `FUN_1406b7320`
(l.675428); `CDialogueLine` l.675451; `CDialogue`/`CDialogueSettings`/`CUIDialogue` l.199183/199208/199658;
`CTriggerBarkWeaponComponent` `FUN_140744b20` (l.754516); `CChopShopVocals` l.803911;
`CConditional_IsRadioStationActive` l.627477 / mapped l.4140317.

Event bus: fire `FUN_147bfeb20` @0x147bfeb20; tokenize `FUN_147625370` @0x147625370; bind `FUN_14762d8a0`
@0x14762d8a0. FMOD banks: `FUN_14017ca20` @0x14017ca20; loader `FUN_14017d470` @0x14017d470. Barks
`FUN_140b9e080` @0x140b9e080. Music `FUN_140b84620` @0x140b84620. Settings `FUN_140bb3a00` @0x140bb3a00,
`FUN_140b4e090`/`FUN_140b4e210`, read `FUN_140b964b0` @0x140b964b0. Resource types `FUN_147ea7ee0`
@0x147ea7ee0 (l.3648369–3648388). Profiler budgets switch at l.3321668–3321678. Voice path
`thunk_FUN_14aadc440` sites l.3985524/4719109/4794321. Vocals config load `FUN_140317620` l.3649358.
Occlusion debug `FUN_1407cd630` @0x1407cd630 (l.833142), `FUN_1430d1610` (l.3118603). Custom DSP source
paths l.4794598 (`KGAmplitudeModulator`), l.4795504 (`KGCompressor`), l.4796913 (`KGReverb`).

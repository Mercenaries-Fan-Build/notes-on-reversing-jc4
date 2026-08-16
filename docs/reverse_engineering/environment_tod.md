# Environment, time-of-day, sky & atmosphere — the world-look pipeline

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative)

> "proven" = read directly from the decomp (`output/_ghidra_jc4/jc4_all_functions_decomp.txt`).
> All `FUN_`/`DAT_`/`PTR_LAB_` addresses below are from that dump. Names in `"quotes"` are the
> engine's own C++ class strings or GPU pass/constant-buffer/texture names that survive as string
> constants (the decomp is a functions-only export — richest for **graphics**, where pass/CB/texture
> names are recoverable).
>
> **Scope split.** Wind, tornadoes, storms, lightning and the GPU weather *fluid solver*
> (`FUN_1401f7f70`) live in [`wind_and_weather.md`](wind_and_weather.md). This doc is the
> **time-of-day → sun/moon → sky/atmosphere/cloud/fog/exposure look** pipeline. The two meet at the
> `CWeatherPreset` / `CEnvironmentPresets` data layer and at the shared `Atmosphere_*` render zones.

## Overview

Just Cause 4's environment look is produced by two cooperating halves, both visible in the decomp:

1. **A gameplay/data layer** — a family of reflected `CEnvironment*` / `CTimeOfDayController` /
   `CWeatherPreset` classes, registered by name through the same global factory
   (`FUN_140f27f60`) as every other gameplay class. Time-of-day, environment *presets* and localized
   *graphics modifier volumes* are authored as RTPC entity components and blended at runtime. These own
   the *parameters* (sun/moon angle, sky/fog/cloud/exposure tunables). (proven registrations; blend
   details inferred)
2. **A GPU render layer** — a precomputed **atmospheric-scattering** model (Bruneton-style
   transmittance/in-scattering/aerial-perspective LUTs), spherical-harmonic **sky lighting**, **cirrus**
   and **volumetric clouds**, **volumetric fog**, **stars**, and a histogram-based **auto-exposure /
   tonemap** stage. Each is a named pass in the render-graph stage enum. (proven — pass/CB/texture names
   read directly)

The engine's own render-graph stage list (`FUN_140d78c40`) orders them: `WEATHER` →
`ATMOSPHERIC_SCATTERING` → `SKY_LIGHTING` → `VOLUMETRIC_CLOUD_NOISE` → `AERIAL_PERSPECTIVE` → …
`SKY_OCCLUDERS` … `VOLUMETRIC_FOG_*` … `STARS` … `PRE_CLOUDS`/`VOLUMETRIC_CLOUDS`/`APPLY_CLOUDS` …
`GROUNDHAZE` → `POSTEFFECTS` (which hosts exposure/tonemap). (proven)

## Key classes & functions

### Gameplay / data layer (reflected classes)
| Class string (name-len) | Registered at (`FUN_`) | Name-hash slot / descriptor | Role |
|---|---|---|---|
| `CTimeOfDayController` (0x14) | `FUN_140814510` @0x140814510 (caller `FUN_14085fd00`) | `DAT_142cb986c` / `PTR_LAB_141d9ce70` | Drives the day/night clock → sun/moon + environment params (proven reg; role inferred) |
| `CConditional_IsTimeOfDay` (0x18) | `FUN_14065fa40` @0x14065fa40 | `DAT_142cb5f30` | Behavior-VM predicate "is it time-of-day X" (gates events) (proven) |
| `CEnvironmentPresets` (0x13) | reg `DAT_142cb0abc` / `PTR_LAB_141d9e068` (line 198701/863801); ctor names instance `"CEnvironmentPresets <n>"` (line 192313) | `DAT_142cb0abc` | Named set of full environment looks (proven) |
| `CEnvironmentPresetTrigger` (0x19) | `FUN_1406b7450` (per wind doc) | `DAT_142cb80d8` / `PTR_LAB_141d9e040` | Trigger volume that switches the active preset (proven) |
| `CEnvironmentLightingObject` (0x1a) | `FUN_1402abc70` @0x1402abc70 | `_DAT_142cb0ab4` | Placed lighting override object (proven reg; role inferred) |
| `CEnvironmentGraphicsModifier` (0x1c) | `FUN_14080be30` @0x14080be30 (caller `FUN_14085fd00`) | `DAT_142cb8920` | A single graphics-parameter override (proven) |
| `CEnvironmentGraphicsVolume` (0x1a) | `FUN_140795ae0` @0x140795ae0 region | `DAT_142cb89fc` | Spatial volume that applies modifiers where the camera is (proven reg; role inferred) |
| `CEnvironmentGraphicsModifierManager` | installed `thunk_FUN_148f6e780(world,"CEnvironmentGraphicsModifierManager",…)` (line 3209725 / 4139207) | — | World singleton that owns/blends the modifier volumes (proven install; blend inferred) |
| `CEnvironmentGfxManager` | system-factory `FUN_148f958e0` @0x148f958e0, class-tag `0x41d8dd50`; name-getter `FUN_14a055f00` | `0x41d8dd50` | Top-level environment-graphics manager singleton (proven) |
| `CWeatherPreset` (0xe) | `FUN_140844920` @0x140844920 | `DAT_142cb9180` / `PTR_LAB_…` | Named weather look (couples to `wind_and_weather.md`) (proven) |

### GPU render layer (passes / setup functions)
| Setup `FUN_` | What it registers (proven strings) |
|---|---|
| `FUN_1401f4fc0` @0x1401f4fc0 | The **AtmosphericScattering** system: passes `AtmosphericScattering`, `AtmosphericScatteringUnderWater`, `AtmosphericScatteringDebug`, `AtmosphericSkyLighting`, `AtmosphereTransmittance`, `AtmosphereInscattering`, `AtmosphereGathering`, `AtmosphereMultipleScatteringOrder`, `AtmosphereAerialPerspective`. Sets the Bruneton constants (below). |
| `FUN_140201290` @0x140201290 | Atmosphere **LUT/target allocation** (called by the above): the SH sky-lighting + transmittance/gathering/inscattering/aerial-perspective textures (below). |
| `FUN_147a1bbd0` @0x147a1bbd0 | `AtmosphericScatteringLUTs` compute resources (`AtmosphericScatteringLUTOutputResources`, `AtmosphericScatteringLUTResources`). |
| `FUN_1401f5c90` @0x1401f5c90 | **`CirrusClouds`** pass registration. |
| `FUN_140212230` @0x140212230 | Cirrus cloud **mesh + textures** (`textures/weather/cirrus_clouds_alpha_dif.ddsc`, `cirrus_cloud_tile_alpha_dif.ddsc`). |
| `FUN_1401ed750` @0x1401ed750 | Cirrus cloud **render** — `"CirrusClouds"` and `"CirrusCloudsShadow"` variants. |
| `FUN_1402145b0` @0x1402145b0 | **`Stars`** star-quad mesh builder. |
| `FUN_147a243d0` @0x147a243d0 | **`StarsShader`** pass. |
| `FUN_1402147f0` @0x1402147f0 | Precipitation/weather render block (`textures/atmosphere/precipitation.ddsc`, `precipitation_normal.ddsc`, `CRenderBlockTypeWeather`, `"Weather"`). |
| `FUN_147a235c0` @0x147a235c0 | **`Weather`** pass + `RainImposter`. |
| ~`FUN_` at line 1540795 | **VolumetricFog**: `VolumetricFogInit`, `VolumetricFogDensityInject`, `VolumetricFogLightingScatter`, `VolumetricFogIntegrate`, base/detail noise generation, `textures/blue_noise.ddsc`. |
| ~`FUN_` at line 1518632 | **Auto-exposure / tonemap**: `ToneMappingEffect`, `LuminanceToDepth`, `Photometer`, `TonemappingHistogram(Clear)` (below). |
| `FUN_140d78c40` @0x140d78c40 | Render-graph **stage-name enum** (the ordered pass list). |
| `FUN_147624480` @0x147624480 | GPU **profiling-zone enum** — `Atmosphere`, `Atmosphere_{Weather,Tornado,Lighting,Storm,Rain,Coverage,Fog,Wind,Clouds,Sky}` (ids 0x25–0x2f). |

## How it works (from the decomp)

### 1. Time-of-day → environment parameters

`CTimeOfDayController` is registered as a reflected **component** (through the component registrar
`FUN_14085fd00`; the descriptor install is `FUN_140814510`, hash slot `DAT_142cb986c`, vtable
`PTR_LAB_141d9ce70`) (proven). It is the clock that advances day/night and hands the current phase to
the environment-preset blend and to the render CBs. The behavior VM can query it via
`CConditional_IsTimeOfDay` (`FUN_14065fa40`, `DAT_142cb5f30`) to gate scripted events on time-of-day.
(proven registrations; the internal sun/moon vector math is in unnamed helpers — no `SunDirection`/
`MoonDirection` string survived, so the exact solar-position formula is **not** directly recoverable
here — inferred.)

Time-of-day is *also* exposed as an animation/effect driver axis. `FUN_14021b3e0` binds a set of named
VFX-modifier input curves through `FUN_140f27f60`, among them **`"time_of_day"`** (0xb) / **`"TimeOfDay"`**
(9), plus `"Precipitation"`, `"PlayerGlobalWindSpeed"`, `"PlayerAltitude"`, `"PlayerHeightAboveGround"`,
`"altitude"`, `"height_above_ground"`, `"energy"` (proven). So effects/materials can be curve-driven by
the current time-of-day and weather state — this is the bridge from the TOD clock into per-effect look.

### 2. Environment presets & graphics-modifier volumes

The look is data-authored as a stack of overrides (all proven registrations; the additive-blend
semantics are inferred from the class shapes and names):

- **`CEnvironmentPresets`** — a named bundle of full environment settings. The constructor builds a
  human-readable instance name `"CEnvironmentPresets <n>"` (line 192313) and registers it by hash
  (`FUN_140f27f60`, `DAT_142cb0abc`). This is the base "global look".
- **`CEnvironmentPresetTrigger`** — a trigger volume that switches which preset is active as the player
  moves through the world.
- **`CEnvironmentGraphicsModifier`** (`FUN_14080be30`) — one parameter override; **`CEnvironmentGraphicsVolume`**
  (`DAT_142cb89fc`) — the spatial region it applies in; **`CEnvironmentGraphicsModifierManager`**
  (installed by `thunk_FUN_148f6e780`) — the singleton that collects the active volumes and blends their
  modifiers into the final environment parameters based on camera position. This is JC4's local-look
  override system (e.g. a valley that is foggier/darker than the global preset).
- **`CEnvironmentLightingObject`** (`FUN_1402abc70`) — a placed lighting override.
- **`CWeatherPreset`** (`FUN_140844920`) — the weather-side counterpart (shared with
  `wind_and_weather.md`); weather and environment presets both feed the same render CBs.

All of this sits under **`CEnvironmentGfxManager`** (system-factory row `0x41d8dd50` in `FUN_148f958e0`),
the top-level manager that owns the environment-graphics state and pushes it to the renderer each frame.

### 3. Atmospheric scattering (precomputed, Bruneton-style)

`FUN_1401f4fc0` builds the `"AtmosphericScattering"` render system and writes a block of physical
constants into its parameter struct (`param_1 + 0x9d8 …`), then calls `FUN_140201290` to allocate the
LUT textures and registers the compute/render passes (proven). The constant block is gated by
`DAT_142cadba0`: a **full-detail** branch and a reduced **reflection/low-res** branch (inferred use).

The full-detail constants are the canonical **Bruneton "Earth" atmosphere** values (proven byte values;
Bruneton identification inferred but unambiguous):

| Param struct offset | Hex | Float | Meaning (inferred) |
|---|---|---|---|
| `+0x9d8` | `0x45c6c000` | **6360.0** | planet **bottom radius** (km) |
| `+0x9dc` | `0x45c94000` | **6440.0** | atmosphere **top radius** (km) → 80 km shell |
| `+0x9ec` | `0x41000000` | **8.0** | **Rayleigh** scale height (km) |
| `+0x9fc` | `0x3f99999a` | **1.2** | **Mie** scale height (km) |
| `+0x9f8` | `0x3b83126f` | **0.004** | Mie/absorption coefficient |
| `+0xa00` | `0x3f8e38e4` | 1.1111 | (tuning) |
| `+0xa04`/`+0xa08` | `0x3f4ccccd` | 0.8 | Mie phase `g` / anisotropy |

The reflection/low-res branch (`DAT_142cadba0 == 1`) substitutes a half-scale planet (**3386.0 / 3518.0**),
scale heights **11.0 / 3.2**, coefficient **0.003** — a cheaper atmosphere for cubemap/reflection capture
(inferred).

`FUN_140201290` allocates the **precomputed LUTs** (all proven texture names; sizes from the alloc descriptors):

- **SH sky-lighting**: `sky_shlighting_R_texture`, `sky_shlighting_G_texture`, `sky_shlighting_B_texture`
  (three 192×192 targets = per-channel spherical-harmonic sky irradiance).
- **Transmittance**: `atmosphere_transmittance_to_sun`.
- **Multiple-scattering gathering**: `atmosphere_gathering_sum`, `atmosphere_gathering_sum_auxillary`,
  `atmosphere_gathering_order_1/2/3`.
- **In-scattering**: `atmosphere_inscattering_sum`, `atmosphere_inscattering_sum_auxillary`,
  `atmosphere_inscattering_order_N` (128×32 3D-style targets).
- **Aerial perspective**: `aerial_perspective_inscattering`, `aerial_perspective_transmittance_to_camera`.

The render/compute passes registered on top: `AtmosphereTransmittance`, `AtmosphereInscattering`,
`AtmosphereGathering`, `AtmosphereMultipleScatteringOrder`, `AtmosphereAerialPerspective`, plus GPU
constant buffers `AtmosphericScattering`, `AerialPerspective`, `ScatteringOrderDynamic`, `Tuning`,
`SkyLighting`, `AtmosphericSkyRenderFP` (proven). The compute LUT wrapper is `FUN_147a1bbd0`
(`AtmosphericScatteringLUTs`). This is a textbook precomputed transmittance → single-scatter → iterated
multiple-scatter → aerial-perspective pipeline.

### 4. Sky lighting, sky occluders, stars

- **Sky lighting** is the SH triplet above, exposed as render stage `SKY_LIGHTING` (`FUN_140d78c40`
  case 0xd) and pass `AtmosphericSkyLighting` (proven).
- **Sky occluders** (`SKY_OCCLUDERS`, stage 0x1e) mask sky light behind geometry: `sky_occlusion_texture`,
  `sky_occlusion_direction_texture` (line 125586), texture `textures/system/sky_occluder.ddsc`
  (line 1540702), object list `PTR_s_SkyOccluders_141ec0878` (proven).
- **Stars**: `FUN_1402145b0` builds a star-quad vertex/index buffer tagged `"Stars"`; `FUN_147a243d0`
  registers the `StarsShader` pass; stage `STARS` = `FUN_140d78c40` case 0x81 (proven). The night-sky
  gradient/atmosphere ramp uses `textures/atmosphere/gradients.ddsc` (line 148785) (proven).

### 5. Clouds

Two cloud systems (proven names; roles inferred from stage ordering):

- **Cirrus (2D layered) clouds** — `FUN_140212230` builds a procedural quad grid and loads
  `textures/weather/cirrus_clouds_alpha_dif.ddsc` + `cirrus_cloud_tile_alpha_dif.ddsc`; `FUN_1401ed750`
  renders `"CirrusClouds"` and a `"CirrusCloudsShadow"` variant for cloud shadowing; setup `FUN_1401f5c90`.
- **Volumetric clouds** — render stages `VOLUMETRIC_CLOUD_NOISE` (0xe), `PRE_CLOUDS` (0x8e),
  `VOLUMETRIC_CLOUDS` (0x92), `VOLUMETRIC_CLOUDS_COMPOSITE` (0x93), `POST_CLOUDS`/`APPLY_CLOUDS`
  (0x90/0x91), plus `CLOUDSHADOWS_0/1` (0x19/0x1a) and reflection variant `REFLECTION_CLOUDS` (0x14)
  and `UNDERWATER_CLOUDS` (0x86) (all proven, `FUN_140d78c40`).

### 6. Fog

- **Volumetric fog** (`FUN_` at line 1540795) is a froxel/V-buffer pipeline: passes `VolumetricFogInit`,
  `VolumetricFogDensityInject`, `VolumetricFogLightingScatter`, `VolumetricFogIntegrate`, with generated
  `volumetric_fog_base_noise_texture` (64³) and `volumetric_fog_detail_noise_texture` (32³) plus
  `textures/blue_noise.ddsc`, and V-buffer targets `VolumetricFog_LightScatteringAccum_0/1`,
  `VolumetricFog_Inscatter`, `VolumetricFog_LightExtinctionAccum_0/1`, `VolumetricFog_Transmittance`,
  `VolumetricFog_VBuffer` (proven). Render stages `VOLUMETRIC_FOG_INIT` (0x7a),
  `VOLUMETRIC_FOG_DENSITY_INJECTION` (0x7b), `VOLUMETRIC_FOG` (0x7f), `VOLUMETRIC_FOG_TRANSITIONS` (0x34),
  `VOLUMETRIC_FOG_POST_TRANSITIONS` (0x80) (proven).
- **Distance/ground fog**: `UNDERWATER_FOG_GRADIENT` (stage 0x5c) and `GROUNDHAZE` (stage 0xa0), both in
  the `Environment` (0x10) profiling category (proven).

### 7. Exposure & tonemap (ties to `rendering_graphics.md` HDR)

The auto-exposure/tonemap setup (`FUN_` at line 1518632) is a **photometer → histogram → exposure**
loop feeding the HDR tonemapper (proven names):

- Passes: `ToneMappingEffect`, `LuminanceToDepth`, `Photometer`, `TonemappingHistogram`,
  `TonemappingHistogramClear`.
- Targets: `PhotometerTexture` (triple-buffered, 1×1-ish reduction chain), `IrradianceTarget` (0xb4×0x140).
- Debug/analysis: `luminance_to_depth_before_exposure`, `luminance_to_depth_after_exposure`,
  `luminance_to_depth_final_scene`.
- Neutral color-correction LUT: `textures/color_curves/color_correction_neutral_exposure.ddsc` (line 110919).

The mechanism: `Photometer` reduces scene luminance into the small `PhotometerTexture`, the histogram
passes bucket it, an exposure value is derived and applied in `ToneMappingEffect` (eye-adaptation). This
is the environment/TOD → brightness coupling: as the sky/atmosphere in-scatter changes with time-of-day,
the auto-exposure re-adapts. (proven passes; the exact adaptation curve lives in shader bytecode — walled.)

## Render-graph stage order (proven — `FUN_140d78c40`)

The environment/sky/atmosphere-relevant slice of the stage enum, in case order (each stage also carries a
coarse category id in `param_2`; `0x10` = the `Environment` zone):

```
0x0b WEATHER            0x0c ATMOSPHERIC_SCATTERING   0x0d SKY_LIGHTING
0x0e VOLUMETRIC_CLOUD_NOISE   0x0f AERIAL_PERSPECTIVE  0x13 REFLECTION_ATMOSPHERE
0x14 REFLECTION_CLOUDS  0x19/0x1a CLOUDSHADOWS_0/1     0x1e SKY_OCCLUDERS
0x5c UNDERWATER_FOG_GRADIENT   0x7a VOLUMETRIC_FOG_INIT  0x7b …DENSITY_INJECTION
0x7f VOLUMETRIC_FOG     0x81 STARS                     0x86 UNDERWATER_CLOUDS
0x8e PRE_CLOUDS         0x90 POST_CLOUDS               0x91 APPLY_CLOUDS
0x92 VOLUMETRIC_CLOUDS  0x93 VOLUMETRIC_CLOUDS_COMPOSITE   0x9d WATER_GODRAYS
0xa0 GROUNDHAZE         0x8f LENSFLARE                 0xbc POSTEFFECTS (exposure/tonemap host)
```

The parallel **profiling-zone enum** `FUN_147624480` gives the debug-marker names the GPU capture tools
show for this system: `Atmosphere` (0x25) and sub-zones `Atmosphere_Weather/Tornado/Lighting/Storm/Rain/
Coverage/Fog/Wind/Clouds/Sky` (0x26–0x2f) (proven).

## Data & config integration

- Every `CEnvironment*` / `CTimeOfDayController` / `CWeatherPreset` class is instantiated by **string
  name → lookup3 hash** through `FUN_140f27f60` (the shared reflection factory; see README
  "cross-cutting architecture"). The same hash keys the asset paths (`docs/formats/name_hash.md`), so a
  class here maps to an RTPC entity-component hash on the data side. (proven mechanism)
- Presets/modifiers are **RTPC entity components** placed in the world (triggers, volumes) — mirroring the
  weather-object pattern documented in `[[rtpc-entity-assembly]]`. The *values* (sun angle, fog density,
  cloud coverage, exposure bias) are data-side ADF/RTPC, not in the executable. (inferred — consistent
  with the project-wide "tunables live in data" finding)
- Textures consumed are ordinary AVTX `.ddsc` assets (see `docs/formats/avtx.md`): the atmosphere
  gradient ramp, cirrus/precipitation alpha maps, blue noise, and the neutral exposure color-curve LUT.
  (proven paths)

## Notable constants / tunables

- **Bruneton atmosphere** (full-detail, `FUN_1401f4fc0`): bottom radius **6360 km**, top **6440 km**,
  Rayleigh scale height **8 km**, Mie scale height **1.2 km**, Mie coefficient **0.004**, phase-g **0.8**.
  Reflection variant: **3386 / 3518 km**, **11 / 3.2 km**, **0.003** (gated by `DAT_142cadba0 == 1`).
- **SH sky-lighting** targets are **192×192** (`0xc0`), three channels (`FUN_140201290`).
- **Volumetric fog** noise: base **64³**, detail **32³** (`FUN_` @line 1540852/1540870).
- **Stars**: mesh built for up to `0x4e2` (1250) quads (`FUN_1402147f0` loop bound); weather render block
  reserves **5000** instances (`CRenderBlockTypeWeather`).
- **DAT_142cd94a0 ∈ {0,3}** repeatedly gates `thunk_FUN_14a05dad0` / `_14a05db80` — GPU debug-marker
  push/pop around resource creation (a capture/debug build flag). (inferred)

## Call-graph highlights

- `FUN_1401f4fc0` (AtmosphericScattering setup) → `FUN_140201290` (LUT alloc) → `FUN_140200c70`;
  registers CBs via `thunk_FUN_14ad70540`. (proven)
- `CTimeOfDayController` descriptor install `FUN_140814510` ← `FUN_14085fd00` (the component registrar).
  (proven)
- `CEnvironmentGraphicsModifier` install `FUN_14080be30` ← `FUN_14085fd00`. (proven)
- `CEnvironmentGfxManager` row in the system-factory table `FUN_148f958e0` (class-tag `0x41d8dd50`).
  (proven)
- `FUN_140d7a540` / `FUN_140d78c40` — render-graph stage registration/lookup (drives every pass above).
  (proven)

## Open questions / lower-confidence

- **Sun/moon position math is not string-anchored.** No `SunDirection`/`MoonDirection`/solar-angle string
  survived; the actual time→direction formula lives in unnamed arithmetic helpers off `CTimeOfDayController`.
  Needs a live trace (x64dbg) or targeted decomp of its update method. (open)
- **Preset blend semantics** (linear interp? priority stack? per-parameter curves?) are inferred from the
  class shapes; the `CEnvironmentGraphicsModifierManager` blend loop was not read in full. (open)
- **Which CB fields hold sun color/intensity/moon** — the `SkyLighting`/`Tuning`/`AtmosphericScattering`
  CB layouts are named but their field offsets were not fully mapped here. (open)
- **Exposure adaptation curve** and tonemap operator specifics are in shader bytecode (walled). (open)

## Appendix — decomp anchors

Strings + addresses used above (all re-verifiable in `output/_ghidra_jc4/jc4_all_functions_decomp.txt`):

- Classes: `CTimeOfDayController` (`FUN_140814510`, `DAT_142cb986c`, line 869277); `CConditional_IsTimeOfDay`
  (`FUN_14065fa40`, line 627778); `CEnvironmentPresets` (line 192313/198701/863801, `DAT_142cb0abc`);
  `CEnvironmentPresetTrigger` (`DAT_142cb80d8`, line 863769); `CEnvironmentLightingObject` (`FUN_1402abc70`,
  line 199333); `CEnvironmentGraphicsModifier` (`FUN_14080be30`, line 863705, `DAT_142cb8920`);
  `CEnvironmentGraphicsVolume` (`DAT_142cb89fc`, line 804440); `CEnvironmentGraphicsModifierManager`
  (lines 3209725, 4139207); `CEnvironmentGfxManager` (`FUN_148f958e0` line 4138300, `FUN_14a055f00`
  line 4564280); `CWeatherPreset` (`FUN_140844920`, line 899960).
- Atmosphere: `FUN_1401f4fc0` (lines 140190–140244 pass names, 140193–140217 Bruneton constants);
  `FUN_140201290` (lines 143378–143667 LUT texture names); `FUN_147a1bbd0` (`AtmosphericScatteringLUTs`,
  line 3506227).
- Sky/stars: `AtmosphericSkyLighting` (140239); `sky_occlusion_texture` (125586); `SkyOccluders`
  (1538463); `textures/system/sky_occluder.ddsc` (1540702); `Stars` (`FUN_1402145b0` line 148454);
  `StarsShader` (`FUN_147a243d0` line 3508381); `textures/atmosphere/gradients.ddsc` (148785).
- Clouds: `CirrusClouds`/`CirrusCloudsShadow` (`FUN_1401ed750` lines 138393/138396); cirrus textures
  (`FUN_140212230` lines 147423/147425); `FUN_1401f5c90` (line 140549).
- Fog: VolumetricFog passes/targets (`FUN_` @line 1540795–1541019); `textures/blue_noise.ddsc` (1540844);
  fog stages (1550985, 1551249–1551273).
- Exposure/tonemap: `FUN_` @line 1518632 (`ToneMappingEffect`, `Photometer`, `TonemappingHistogram`,
  `PhotometerTexture`, `IrradianceTarget`, `luminance_to_depth_*`); `color_correction_neutral_exposure.ddsc`
  (110919); `ToneMapping::LocalResources` (1515105).
- Enums: render-stage names `FUN_140d78c40` @0x140d78c40 (lines 1550777–1551551); profiling zones
  `FUN_147624480` @0x147624480 (lines 3321618–3321638).
- TOD/VFX curve drivers: `FUN_14021b3e0` (`time_of_day` 149710, `TimeOfDay` 149753, `Precipitation`,
  `PlayerAltitude`, `PlayerHeightAboveGround`).

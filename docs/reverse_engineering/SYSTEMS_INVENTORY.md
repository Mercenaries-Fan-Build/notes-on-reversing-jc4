# JC4 Systems Inventory — the full map

A game this size has *dozens* of systems. This is the complete inventory of documentable systems discovered
in the decomp (`output/_ghidra_jc4/jc4_all_functions_decomp.txt`), grouped into engine pillars. It is the
master checklist behind `README.md`; each row becomes (or has become) a `docs/reverse_engineering/<doc>.md`.

Status: ✅ documented · ◻ pending · 🔶 partial (covered inside another doc).

> **Methodology caveat (proven during the wind dig).** This is a *functions-only* export. Recoverable from it:
> class **registration** skeletons (name→hash→factory via `FUN_140f27f60`/`FUN_14085fd00`/`FUN_148f960c0`),
> the **call graph**, reachable CPU logic, and **GPU shader/compute** setup (named passes + CBs + textures).
> **Not** recoverable here: per-component **tick bodies**, and tunable **magnitudes** (forces, radii, damage,
> timeouts) — these sit behind data-section factory vtables (`PTR_LAB_*`), in omitted ctor address ranges, or
> in RTPC/ADF/Havok-tagfile **data**. Docs must document what's present, tag walled items as open (route:
> ADF type lib via `jc4_adf`, RTPC `.epe` property tables, or a live x64dbg pass), and **never invent values**.

## Pillar 1 — Engine core: ECS, reflection, events, scheduling
| Doc | Systems / anchor classes | Status |
|---|---|---|
| behavior_system.md | Condition VM + action graph + the lookup3 type registry | ✅ |
| entity_core_ecs.md | `CGameObjectManager`, `CTransformComponent`, `CInspectableComponent`, `CConditionalManager`, `CActionTokenManager`, entity/component lifecycle, the reflection/type registry spine | ✅ |
| event_scheduler.md | `CGameplayEventManager` (event bus), `CBinaryStateObjectManager`, `CTimestampManager`, scheduler/tick, `CSettingsManager` | ◻ |

## Pillar 2 — Rendering & graphics
| Doc | Systems / anchor classes | Status |
|---|---|---|
| rendering_graphics.md | `CRenderBlock*` shader family (General/Character/CarPaint/Bark/Hologram/EnergyShield/DemonDome/DemonOrganic/Weather), `CModelInstanceManager`, `CMeshEffectManager`, `ChromaManager`, deferred/HDR/bloom/SSR (`CompositeBloom`, `Checkerboard*HDR`, `CompositeScreenSpaceReflection*`), occlusion | ✅ |
| lighting_shadows.md | lighting, shadow pipeline (cloud shadows, `CarPaintShadow*`, `CharacterDepthShadow*`), `CSpotlightController`, `CLightOcclusionPlaneObject` | ✅ |
| environment_tod.md | `CTimeOfDayController`, `CEnvironmentGfxManager`, `CEnvironmentGraphicsModifierManager`, sky/atmosphere, weather GPU fluid sim (`FUN_1401f7f70`, Navier-Stokes compute) | ✅ |

## Pillar 3 — Environment & weather (signature)
| Doc | Systems / anchor classes | Status |
|---|---|---|
| wind_and_weather.md | wind volumes, tornado family, storms, lightning, force pulses, Mech Wind Cannon | ✅ |

## Pillar 4 — World, streaming & terrain
| Doc | Systems / anchor classes | Status |
|---|---|---|
| world_streaming_terrain.md | `CResourceLoaderManager`, world streaming, `CLandscapeManager`, `CBiomeManager`, `CGameObjectManager` (spawn side), `CDiscoveryManager`, `CCoverageManager` | ✅ |
| roads_rivers_water.md | `CRoadManager`, `COnRoadService`, `CRiverManager`, water rendering/simulation | ◻ |
| spawning_population.md | `CSpawnSystem`, `CPlayerSpawnPointManager`, population/traffic spawning | ◻ |

## Pillar 5 — Physics
| Doc | Systems / anchor classes | Status |
|---|---|---|
| physics_constraints.md | `CPhysicsSystem`, `CPhysicalRestraintsSystem`, `CConstraintFactory`, ragdoll, external force generators, `CPfxBodyPropsSystem` | ✅ |
| destruction.md | Havok NDivision destruction, chaos objects, force pulse | ✅ |

## Pillar 6 — AI, combat & encounters
| Doc | Systems / anchor classes | Status |
|---|---|---|
| ai_combat_encounters.md | `CAiSystem`, `CCombatCoordinator`, `CEncounterManager`, `CEncounterEngagedController`, `CTacticalNodeManager`, `CTargetSystem` | ✅ |
| characters_creatures.md | `CCharacterManager`, `CCreatureManager`, `CPlayerManager`, character controller, NPC roles | ✅ |
| road_graph_driving.md | AI vehicle driving on the road graph (`CRoadManager` consumer side) | ◻ |

## Pillar 7 — The Demon antagonist (endgame/DLC)
| Doc | Systems / anchor classes | Status |
|---|---|---|
| demon_system.md | `CDemonAreaManager`, `CDemonWaveSpawner`, `CDemon*StateObject`, `CDemonChaosCluster`, `CDemonCoreStateObject`, possession mechanics (`CDemonHasPossessedCharacter`, `…PossessionTarget`), flying/movement blend states, `CDemonDome`/`DemonOrganic` render, `CDemoniosOutroStateController` | ✅ |

## Pillar 8 — Faction / Frontline / Chaos metagame (JC4 core loop)
| Doc | Systems / anchor classes | Status |
|---|---|---|
| faction_frontline_chaos.md | `CFrontlineManager`, `CFrontlineConnection`, `CFrontlineEventRouter`, `CAgencyBaseManager`, `CAgencyRing(Group)`, `CChaosReward`, `CBookmarkChaosDescription`, conditions (`IsNearActiveFrontline`, `IsInSecuredTerritory`, `HasEnoughChaos`, `FactionStatus`, `DemonDomePercentage`), `CHeatManager` (wanted), `CDiscoveryManager` | ✅ |

## Pillar 9 — Missions, objectives & activities
| Doc | Systems / anchor classes | Status |
|---|---|---|
| missions_progression.md | quest/objective managers, supply, collection, statistic, DLC gating | ✅ |
| objectives_operations.md | `CMissionManager`, `CObjectiveManager`, `CObjectiveContentManager`, `CObjectiveVehicleController`, `CObjectiveCharacterController`, `COperationManager` (story ops) | 🔶 |
| challenges_stunts.md | `CDaredevilPointManager`, `CJustRunManager`, `CChallengeManager`, stunt/leaderboard feats | ◻ |

## Pillar 10 — Player, traversal, camera & input
| Doc | Systems / anchor classes | Status |
|---|---|---|
| grappling_hook.md | grapple / tether / reel / winch | ✅ |
| traversal_movement.md | parachute / wingsuit / hoverboard / locomotion | ✅ |
| camera_input.md | `CCameraManager`, camera modifiers (`CMoveInput*`/`CRollModifier`), `CInputSystem`, `CUIInputManager` | ✅ |

## Pillar 11 — Vehicles
| Doc | Systems / anchor classes | Status |
|---|---|---|
| vehicles.md | driving, entry/exit/hijack, vehicle winch, driving-force registry | ✅ |
| vehicle_data_pilots.md | `CVehicleDataManager`, `CPilotDataManager`, `CObjectiveVehicleController` | 🔶 |

## Pillar 12 — Weapons & combat items
| Doc | Systems / anchor classes | Status |
|---|---|---|
| weapons.md | weapon component framework + firing math | ✅ |
| weapon_gadget_components.md | the ~30 `C*WeaponComponent` catalog (Turret/Magnet/Laser/LightningBeam/Scope/Spotlight/Physicalization/HitReactionCast/CameraSwap/BarrelSpin/AirplaneFlyby/FOW/Lockon/Cylinder/Cannon/…), `CWeaponManager`, `CAmmunitionManager`, `CRetoolerManager` (mods) | ✅ |

## Pillar 13 — Progression, economy & supply
| Doc | Systems / anchor classes | Status |
|---|---|---|
| progression_economy.md | `CStashManager`, `CSupplyDropManager`, `CSupplyFactory(Manager)`, `CSupplyRewardManager`, `CRewardSystem`, `CNewGamePlusManager`, inventory/equipment, unlocks | ✅ |

## Pillar 14 — Audio & dialogue
| Doc | Systems / anchor classes | Status |
|---|---|---|
| audio_dialogue.md | `CSoundSystem`, `CRadioSystem`, `CVocalsManager`, `CDialogueManager`, `CDialogueCoordinator`, `CDialogueChain`, sound occlusion volumes (`CSoundOcclusion*`) | ✅ |

## Pillar 15 — UI, HUD & menus
| Doc | Systems / anchor classes | Status |
|---|---|---|
| ui_hud_menus.md | `CUIManager`, `CUIController`, `CHUDUI`, `CNotificationManager`, `CButtonHintManager`, `CTutorialManager`, `CGPSController`, `CHealthbarComponent`, `COutline`, Scaleform (`.gfx`/CFX) | ✅ |

## Pillar 16 — Narrative, cutscene & media
| Doc | Systems / anchor classes | Status |
|---|---|---|
| narrative_cutscene_media.md | `CCutsceneManager`, `CDialogueChain`, `CMediaRevolutionManager` (in-world propaganda/news), `CContentIntroductionManager`, `CVideoManager`, `CVideoRecordingManager` | ◻ |

## Pillar 17 — Online, platform & social
| Doc | Systems / anchor classes | Status |
|---|---|---|
| online_platform_social.md | `COnlineSuiteManager`, `COnlinePlatformSystem`, `COnlineFeatureManager`, `CLeaderboardManager`, `CFriendManager`, `CPresenceManager`, `CProfileManager`, `CAchievementsManager`, `CPlayerReportingManager`, `CPlatformController` | ◻ |

## Pillar 18 — Effects & particles
| Doc | Systems / anchor classes | Status |
|---|---|---|
| effects_particles.md | `CEffectSystem`, `CMeshEffectManager`, `CPfxBodyPropsSystem`, GPU particle force-field affectors (`VFX_GlobalWind`, `DampingParams`, `GravityPointParams`, `SimplexCurlNoise`, …), decals (`CHavokDestructionDecalManager`) | ✅ |

---

### Coverage
- **Documented (✅):** 8 — behavior_system, grappling_hook, traversal_movement, vehicles, weapons, destruction, wind_and_weather, missions_progression.
- **Pending (◻/🔶):** ~30 systems across the 18 pillars above.

Docs are produced in **waves** of parallel decomp-mining agents (see `README.md` methodology). Update the
status column as each lands.

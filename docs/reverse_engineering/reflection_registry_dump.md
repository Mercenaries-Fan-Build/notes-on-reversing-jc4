# Live reflection-registry dump — class name → name-hash → vtable

Source: running JustCause4.exe (Denuvo build) under x64dbg, reflection registry arena `0x93000000`.

Key = `lookup3_hashlittle(className, seed=0)` (verified: `CInspectableComponent`→`0x426c9d98`).
Slot = `{u32 hash, u16 next, u16 pad, u64 prototype*}`; prototype = `{u64 class-vtable, u64 flags}`.
Heap VAs are per-run; **vtable RVAs are stable** across launches.

Membership = confirmed name-hash slots + their contiguous slot-array neighbors.

## Named classes (326)

| class | name-hash | vtable RVA |
|---|---|---|
| `CAgencyRingGroup` | `0xde353871` | `.text+0x1d9eae0` |
| `CAimPOIWeaponComponent` | `0x53dcebba` | `.text+0x1d9b020` |
| `CAirplaneFlybyWeaponComponent` | `0x641020fe` | `.text+0x1d9b390` |
| `CAmbientEffects` | `0xdf1c3f90` | `.text+0x1d9cce0` |
| `CAmmoRegenerationWeaponComponent` | `0x7af560ae` | `.text+0x1d9b3e0` |
| `CAnimSpline` | `0xb06e1cf6` | `.text+0x1cf4348` |
| `CAreaDamage` | `0x4ae69958` | `.text+0x1d9daa0` |
| `CAudioCaveOcclusionPlane` | `0x23dca96f` | `.text+0x1d9e0e0` |
| `CBarrelRecoilWeaponComponent` | `0x0875ee52` | `.text+0x1d9b048` |
| `CBarrelSpinWeaponComponent` | `0x6da9c5ba` | `.text+0x1d9b070` |
| `CBigButtonHint` | `0x4a61034c` | `.text+0x1d9da00` |
| `CBiomeVolume` | `0x269c1bb1` | `.text+0x1d9cbc8` |
| `CBirdSpawner` | `0xe748df4a` | `.text+0x1d9c718` |
| `CBookmarkChaosDescription` | `0x508d4431` | `.text+0x1d9a328` |
| `CBroadcastAwarenessEvent` | `0x1a9dfc19` | `.text+0x1d9db68` |
| `CBulletSpawner` | `0x0d6a60c3` | `.text+0x1d9afa8` |
| `CCameraEffectEmitter` | `0xbe75afe7` | `.text+0x1d9df00` |
| `CCameraFollowWeaponComponent` | `0x4461bbb1` | `.text+0x1d9b098` |
| `CCameraObject` | `0x0654372b` | `.text+0x1d9a670` |
| `CCameraSwapWeaponComponent` | `0xc5bd43e6` | `.text+0x1d9b0c0` |
| `CChaosReward` | `0xd540f14d` | `.text+0x1d93338` |
| `CCharacterInfoDescription` | `0xed0760de` | `.text+0x1d9a418` |
| `CChopShopVocals` | `0xeeaf518a` | `.text+0x1d9b598` |
| `CCinematicTrigger` | `0x0ec57450` | `.text+0x1d9a878` |
| `CCollisionFilterEditor` | `0xeb7744b1` | `.text+0x1d9aa58` |
| `CComboScorer` | `0xc8073f5d` | `.text+0x1d9c6c8` |
| `CConditional_AND` | `0x90ad09e1` | `.text+0x1d91f18` |
| `CConditional_AND_Counter` | `0x10064752` | `.text+0x1d91f98` |
| `CConditional_AND_Duration` | `0xbe961710` | `.text+0x1d91f78` |
| `CConditional_AreSupplyDropsUnlocked` | `0xc802017b` | `.text+0x1d92b58` |
| `CConditional_CanUnlockNextMediaRevolutionMilestone` | `0xbe08e3a6` | `.text+0x1d92f18` |
| `CConditional_CollectibleCompletion` | `0x7e5a9603` | `.text+0x1d92b38` |
| `CConditional_CollectibleStatus` | `0x4f491887` | `.text+0x1d925d8` |
| `CConditional_ContentIntroductionStatus` | `0x8e244087` | `.text+0x1d92cf8` |
| `CConditional_DemonDomePercentage` | `0xb3284fdc` | `.text+0x1d93578` |
| `CConditional_DeployedTetherCount` | `0x9ad66fd7` | `.text+0x1d92958` |
| `CConditional_FactionStatus` | `0xc9f9fec8` | `.text+0x1d92338` |
| `CConditional_HasBeatenDaredevilRacesByTier` | `0x40db86c8` | `.text+0x1d92e78` |
| `CConditional_HasEnoughChaos` | `0x742d8129` | `.text+0x1d92398` |
| `CConditional_HasPassenger` | `0x5f71d335` | `.text+0x1d92218` |
| `CConditional_HasPassengers` | `0x82c230e3` | `.text+0x1d92278` |
| `CConditional_HasPilotUnlocked` | `0x0ae18f76` | `.text+0x1d92558` |
| `CConditional_HasUnlockedSupplyItems` | `0xdefdd5e1` | `.text+0x1d92bb8` |
| `CConditional_HeatUnitCount` | `0x38b5ed9a` | `.text+0x1d926d8` |
| `CConditional_IsAgencyRingCourseActive` | `0xdae421ae` | `.text+0x1d92f58` |
| `CConditional_IsAgencyRingCourseComplete` | `0xd912af3f` | `.text+0x1d92f38` |
| `CConditional_IsAnyCutsceneActive` | `0x23cbb3da` | `.text+0x1d92938` |
| `CConditional_IsDaredevilRaceTierReached` | `0xa4eaa2e2` | `.text+0x1d92d38` |
| `CConditional_IsDLCUnlocked` | `0x84e8ee0b` | `.text+0x1d92298` |
| `CConditional_IsHoveringOverDiscoveryLocation` | `0x6b91472a` | `.text+0x1d92eb8` |
| `CConditional_IsHoveringOverMapIcon` | `0x7a142368` | `.text+0x1d92b18` |
| `CConditional_IsLocationDiscovered` | `0x87bad211` | `.text+0x1d92a38` |
| `CConditional_IsMediaRevolutionMilestoneApplied` | `0x5c0d544d` | `.text+0x1d92ef8` |
| `CConditional_IsMenuIconHovered` | `0x866b6f1e` | `.text+0x1d92638` |
| `CConditional_IsMissionActive` | `0x4bf12ca9` | `.text+0x1d923d8` |
| `CConditional_IsMissionCompleted` | `0xb828ffa9` | `.text+0x1d927b8` |
| `CConditional_IsMonthlyChallengeAvailable` | `0xcf9c5034` | `.text+0x1d92d78` |
| `CConditional_IsMonthlyChallengeComplete` | `0x9c4a782a` | `.text+0x1d92d58` |
| `CConditional_IsMonthlyChallengeRewardClaimed` | `0xa0ad69fc` | `.text+0x1d92e98` |
| `CConditional_IsNearActiveFrontline` | `0xb96252c4` | `.text+0x1d92af8` |
| `CConditional_IsNodeDiscovered` | `0x1f65f13a` | `.text+0x1d92418` |
| `CConditional_IsNodeSecured` | `0xaeb2a5ad` | `.text+0x1d92258` |
| `CConditional_IsObjectConnectedToNumWinches` | `0x04ea2bf9` | `.text+0x1d92e58` |
| `CConditional_IsPilotReady` | `0x9aef7ea2` | `.text+0x1d92598` |
| `CConditional_IsPilotSelectedInFastTravelUI` | `0xf0a3b1bd` | `.text+0x1d92e38` |
| `CConditional_IsPilotSelectedInUI` | `0x5cb652be` | `.text+0x1d92918` |
| `CConditional_IsPlayerInHeatAOO` | `0xdff79a78` | `.text+0x1d926b8` |
| `CConditional_IsPlayerInParachute` | `0x4cc49d13` | `.text+0x1d92818` |
| `CConditional_IsPlayerInsideBiome` | `0x2176ddee` | `.text+0x1d92978` |
| `CConditional_IsPlayerInsideNodeDiscoveryVolume` | `0xd9467955` | `.text+0x1d92ed8` |
| `CConditional_IsPlayerInVehicleType` | `0x6756e413` | `.text+0x1d92ad8` |
| `CConditional_IsPlayerInWeather` | `0x3977500c` | `.text+0x1d92658` |
| `CConditional_IsPlayerInWind` | `0x632f9865` | `.text+0x1d92378` |
| `CConditional_IsPlayerInWingsuit` | `0x3ad0e335` | `.text+0x1d92738` |
| `CConditional_IsRadioStationActive` | `0x6c74dbc8` | `.text+0x1d929d8` |
| `CConditional_IsReeledOnObject` | `0x60f7f89d` | `.text+0x1d92578` |
| `CConditional_IsRetoolerFeatureUnlocked` | `0xd9690780` | `.text+0x1d92d18` |
| `CConditional_IsRetoolerGroupSelected` | `0xb7d649c9` | `.text+0x1d92bf8` |
| `CConditional_IsRetoolerLoadoutSelected` | `0xd89d90ed` | `.text+0x1d92cd8` |
| `CConditional_IsRetoolerOptionEquipped` | `0xee3e3a4a` | `.text+0x1d92c98` |
| `CConditional_IsSupplyEquippedByPilot` | `0x9368f75f` | `.text+0x1d92c38` |
| `CConditional_IsTargetingObject` | `0xbd3ae474` | `.text+0x1d925b8` |
| `CConditional_IsTetheredToObject` | `0x0f42b64f` | `.text+0x1d92758` |
| `CConditional_IsTimeOfDay` | `0x983469d4` | `.text+0x1d920b8` |
| `CConditional_IsTutorialPhaseActive` | `0x953f91e8` | `.text+0x1d92a98` |
| `CConditional_Manual` | `0xb1de2ae3` | `.text+0x1d91fd8` |
| `CConditional_Misc` | `0x15b68b95` | `.text+0x1d91fb8` |
| `CConditional_MissionStatus` | `0xa68da343` | `.text+0x1d92238` |
| `CConditional_NOT` | `0xe4e3ff31` | `.text+0x1d91f38` |
| `CConditional_ObjectiveGoalStatus` | `0xa9c839f1` | `.text+0x1d928f8` |
| `CConditional_ObjectTrackerStatus` | `0x6a300b75` | `.text+0x1d92878` |
| `CConditional_OR` | `0x2aebcc3b` | `.text+0x1d91ef8` |
| `CConditional_SpawnTags` | `0xd71f3878` | `.text+0x1d92038` |
| `CConditional_TetherConnection` | `0x35cf32aa` | `.text+0x1d92478` |
| `CConditional_TetherState` | `0x2e0712a4` | `.text+0x1d92098` |
| `CConditional_TetherState_Simple` | `0xc1419d3a` | `.text+0x1d92798` |
| `CConditionalLock` | `0x1e7d39ed` | `.text+0x1d91f58` |
| `CContentIntroductionObject` | `0xc14a053a` | `.text+0x1d9ec70` |
| `CContentSwapperWeaponComponent` | `0x2dcd2fe2` | `.text+0x1d9b3b8` |
| `CCoverageObject` | `0x974d3c3c` | `.text+0x1cf4168` |
| `CCoverVolume` | `0x02f56e7a` | `.text+0x1d9dc30` |
| `CCreatureRigidObject` | `0xf2403ad8` | `.text+0x1d9aaf8` |
| `CCutscene` | `0x81902739` | `.text+0x1d9e338` |
| `CCutsceneScene` | `0x01e8fac0` | `.text+0x1d9e360` |
| `CCylinderWeaponComponent` | `0xe610a815` | `.text+0x1d9b228` |
| `CDamageController` | `0x661e09f9` | `.text+0x1d9dac8` |
| `CDamageControllerHitAnalyze` | `0x78f1da76` | `.text+0x1d9db18` |
| `CDamageControllerHitApply` | `0x70b71958` | `.text+0x1d9daf0` |
| `CDaredevilAnnouncement` | `0x2a4bac47` | `.text+0x1d93538` |
| `CDaredevilCheckpoint` | `0x474b8adc` | `.text+0x1d934f8` |
| `CDaredevilComboLevel` | `0x4446193a` | `.text+0x1d93518` |
| `CDaredevilPointCharacterKilled` | `0xbbeb5c41` | `.text+0x1d93478` |
| `CDaredevilPointEvent` | `0x3421e9c6` | `.text+0x1d934b8` |
| `CDaredevilPointId` | `0xd078575c` | `.text+0x1d934d8` |
| `CDaredevilPointModelDestroyed` | `0x6d5bc4c2` | `.text+0x1d93498` |
| `CDaredevilPointVehicleDestroyed` | `0x21376e16` | `.text+0x1d93458` |
| `CDaredevilRaceObject` | `0xdc94478d` | `.text+0x1d9e900` |
| `CDemonArea` | `0xd24813ec` | `.text+0x1d9e9a0` |
| `CDemonCoreStateObject` | `0xdd25cbad` | `.text+0x1d9ea40` |
| `CDemoniosOutroStateController` | `0x1379cbdf` | `.text+0x1d9eab8` |
| `CDemonStateObject` | `0xfadabce2` | `.text+0x1d9e978` |
| `CDialogue` | `0x2d812521` | `.text+0x1cf42a8` |
| `CDialogueChain` | `0x95dd7a47` | `.text+0x1d9e310` |
| `CDialogueLine` | `0x151b8ad2` | `.text+0x1d932f8` |
| `CDialogueSettings` | `0x0a2619a0` | `.text+0x1cf42d0` |
| `CDiscoveryLocation` | `0xea42efa1` | `.text+0x1d9cb78` |
| `CDiscoveryLocationSetter` | `0xb61d43bb` | `.text+0x1d9ce98` |
| `CDiscoveryMapIcon` | `0xf48c1805` | `.text+0x1d9cba0` |
| `CDiscoveryVolume` | `0x12c791c8` | `.text+0x1d9cb50` |
| `CDoActWeaponComponent` | `0xb7962d6a` | `.text+0x1d9b0e8` |
| `CDynamicLightFlasher` | `0xd1b97988` | `.text+0x1cf4460` |
| `CDynamicLightObject` | `0x5f931a9b` | `.text+0x1cf43e8` |
| `CDynamicNavMeshCutter` | `0x0d0a7cfc` | `.text+0x1d9dc80` |
| `CEffectBoxEmitter` | `0xf41d3b65` | `.text+0x1d9ded8` |
| `CEffectCollectibleEmitter` | `0xd3612152` | `.text+0x1d9de10` |
| `CEffectLayerEmitter` | `0xe4d07059` | `.text+0x1d9df78` |
| `CEffectLineEmitter` | `0xb8d4e990` | `.text+0x1d9de60` |
| `CEffectPlaneEmitter` | `0x7e2b9b3e` | `.text+0x1d9de88` |
| `CEffectPointEmitter` | `0x4b8ace51` | `.text+0x1d9de38` |
| `CEffectSphereEmitter` | `0x8f6de952` | `.text+0x1d9deb0` |
| `CEffectUIEmitter` | `0xab3b3738` | `.text+0x1d9dfa0` |
| `CEffectVolumeEmitter` | `0x19f61b49` | `.text+0x1d9df28` |
| `CEncounterDependency` | `0x8de00787` | `.text+0x1d9e5e0` |
| `CEncounterWeightModifier` | `0x4fc2afc4` | `.text+0x1d9e5b8` |
| `CEntityObject` | `0xb19cf538` | `.text+0x1d9a6e8` |
| `CEntitySpline` | `0x4049ce3f` | `.text+0x1d9c830` |
| `CEntitySplineOccupant` | `0x4211e8d8` | `.text+0x1d9c8a8` |
| `CEntitySplinePoint` | `0xc8443d08` | `.text+0x1d9c880` |
| `CEntitySplineSegment` | `0x4e16497e` | `.text+0x1d9c858` |
| `CEnvironmentGraphicsModifier` | `0xda7fb1c7` | `.text+0x1d9cc68` |
| `CEnvironmentGraphicsVolume` | `0xc3bcaa74` | `.text+0x1d9cc90` |
| `CEnvironmentLightingObject` | `0xb7dc8115` | `.text+0x1cf4500` |
| `CEnvironmentPresets` | `0x484d3d68` | `.text+0x1d9e068` |
| `CEnvironmentPresetTrigger` | `0x65bf7866` | `.text+0x1d9e040` |
| `CExtrudeSplineObject` | `0x08c23b0e` | `.text+0x1cf4320` |
| `CFastTravelSpawnPoint` | `0xf6c6ffa3` | `.text+0x1d9cbf0` |
| `CForceField` | `0x33915655` | `.text+0x1d9b610` |
| `CForcePoint` | `0xdadf0d70` | `.text+0x1d9b638` |
| `CForcePulse` | `0xbcb5a6d7` | `.text+0x1d9b5e8` |
| `CFOWControllerWeaponComponent` | `0xf55b5afb` | `.text+0x1d9b138` |
| `CFrontlineEventRouter` | `0x991cd83e` | `.text+0x1d9a580` |
| `CFuelLineConnection` | `0x544c4ef5` | `.text+0x1d9c948` |
| `CGameObjectExtension` | `0x75b0aa93` | `.text+0x1d9ed38` |
| `CGameObjectList` | `0xa4afeb6d` | `.text+0x1d9dff0` |
| `CGameObjectReferenceObject` | `0xfcfd0378` | `.text+0x1cf4230` |
| `CGrapplePoint` | `0xcc8afe47` | `.text+0x1d9dbb8` |
| `CGrapplingHook` | `0xdc0c056d` | `.text+0x1d9b4f8` |
| `CGrapplingHookPropData` | `0xde9d1ef1` | `.text+0x1d9b548` |
| `CGrapplingHookWire` | `0x229ab4be` | `.text+0x1d9b520` |
| `CGroupHealthCheckpoint` | `0x11748900` | `.text+0x1d93418` |
| `CHavokDestructionObject` | `0xecb5e383` | `.text+0x1d9aad0` |
| `CHealthbarComponent` | `0xe2e6163c` | `.text+0x1d933f8` |
| `CHealthControl` | `0x8e0ed000` | `.text+0x1d9a5f8` |
| `CHelicopter` | `0xf34b4d33` | `.text+0x1d9c3f8` |
| `CHitReactionCastWeaponComponent` | `0x32fb1a25` | `.text+0x1d9b250` |
| `CHoverboardObject` | `0xb8de8ac4` | `.text+0x1d9b458` |
| `CHoverboardUnlocker` | `0xd496e952` | `.text+0x1d9b4d0` |
| `CInspectableComponent` | `0x426c9d98` | `.text+0x1d9d118` |
| `CLaserBeamWeaponComponent` | `0x93e6c543` | `.text+0x1d9b160` |
| `CLightningBeamWeaponComponent` | `0xf1e5d4a7` | `.text+0x1d9b188` |
| `CLightningObject` | `0x9107a15d` | `.text+0x1d9cad8` |
| `CLightningStrikeController` | `0x492258a2` | `.text+0x1d9cb00` |
| `CLightOcclusionPlaneObject` | `0x5a6b03cc` | `.text+0x1cf4528` |
| `CLightWeaponComponent` | `0x079c8a3e` | `.text+0x1d9b1b0` |
| `CListenerEffects` | `0xf24cb1e6` | `.text+0x1d9e108` |
| `CLocalDensityVolumeObject` | `0x3d794bf8` | `.text+0x1cf4438` |
| `CLocalWindObject` | `0xf81559f5` | `.text+0x1d9c5b0` |
| `CLocationResolverPilot` | `0xc283757e` | `.text+0x1d9e400` |
| `CLocationResolverSupply` | `0x7177f6f8` | `.text+0x1d9e450` |
| `CLockonWeaponComponent` | `0xc28c9ead` | `.text+0x1d9b368` |
| `CMagnet` | `0x08c1f8bb` | `.text+0x1d9b660` |
| `CMagnetWeaponComponent` | `0xee8184e7` | `.text+0x1d9b1d8` |
| `CMagrail` | `0xcc898ea2` | `.text+0x1d9c8f8` |
| `CModelAttachementWeaponComponent` | `0xbb54b36c` | `.text+0x1d9b200` |
| `CMountedComponent` | `0xad2b305f` | `.text+0x1d9afd0` |
| `CMusicProgressSetter` | `0x6e079e50` | `.text+0x1d9cee8` |
| `CMusicTrack` | `0xea205eb3` | `.text+0x1d9e090` |
| `CNavMeshExcluder` | `0xdae089c9` | `.text+0x1d9dc08` |
| `CObjectHealthCheckpoint` | `0x263bbae7` | `.text+0x1d9ebf8` |
| `CObjectiveContentImage` | `0x72d16323` | `.text+0x1d93238` |
| `CObjectiveContentImage_Vehicle` | `0x6f03c0ad` | `.text+0x1d93258` |
| `CObjectiveDebugText` | `0x157f88cd` | `.text+0x1d93178` |
| `CObjectiveGoal_Area` | `0xfcef610d` | `.text+0x1d93158` |
| `CObjectiveGoal_CategoryCounter` | `0x633c667f` | `.text+0x1d93118` |
| `CObjectiveGoal_Conditional` | `0x39d17e2e` | `.text+0x1d93098` |
| `CObjectiveGoal_Counter` | `0xa86bc3f7` | `.text+0x1d930f8` |
| `CObjectiveGoal_Distance` | `0x2803fe55` | `.text+0x1d930d8` |
| `CObjectiveGoal_StayNearObject` | `0xfb74c68b` | `.text+0x1d92fd8` |
| `CObjectiveGoal_TimeLimit` | `0x745dfeed` | `.text+0x1d930b8` |
| `CObjectiveParam` | `0xdbf9f42c` | `.text+0x1d931d8` |
| `CObjectiveParam_DaredevilScoreAndTimer` | `0xd7239401` | `.text+0x1d92f78` |
| `CObjectiveParam_HealthBar` | `0x08a5464a` | `.text+0x1d93058` |
| `CObjectiveParam_ProgressBar` | `0xffd75311` | `.text+0x1d93038` |
| `CObjectiveParam_TornadoWidget` | `0x07bed6e0` | `.text+0x1d92ff8` |
| `CObjectivesInfoDescription` | `0xd943820b` | `.text+0x1d9a300` |
| `CObjectiveTime` | `0xd18215c6` | `.text+0x1d93218` |
| `CObjectiveTimeCap` | `0x045bebc7` | `.text+0x1d931b8` |
| `CObjectiveTimeSet` | `0x086b8bbe` | `.text+0x1d93198` |
| `CObjectiveVehicleControllerSeat` | `0xaf0b0a81` | `.text+0x1d92f98` |
| `CObjectiveVehicleOverride` | `0x76e5241e` | `.text+0x1d93438` |
| `CObjectSource` | `0x606ddae3` | `.text+0x1d9d640` |
| `CObjectSourceSpawners` | `0xc8973e78` | `.text+0x1d9d668` |
| `CObjectTracker` | `0x70bd78ba` | `.text+0x1d9d280` |
| `CObjectTrackerFilter` | `0xd3c6d6d0` | `.text+0x1d9d2a8` |
| `CObjectTrackerFilterChaosObject` | `0xd32fa975` | `.text+0x1d9d398` |
| `CObjectTrackerFilterFaction` | `0xc919b299` | `.text+0x1d9d2d0` |
| `CObjectTrackerFilterSpawnSources` | `0xe50b1246` | `.text+0x1d9d370` |
| `CObjectTrackerFilterSpawnTags` | `0x292b3597` | `.text+0x1d9d4b0` |
| `CObjectTrackerFilterVehicleType` | `0xe334676f` | `.text+0x1d9d2f8` |
| `COutline` | `0x32f276ea` | `.text+0x1d9dfc8` |
| `CParachuteObject` | `0x8ec44dac` | `.text+0x1d9b408` |
| `CPhysicalizationWeaponComponent` | `0xd7dc2b0d` | `.text+0x1d9b278` |
| `CPhysicallyRestrainedBehaviour` | `0xd77216e5` | `.text+0x1d9dd70` |
| `CPilotData` | `0xf2758ebb` | `.text+0x1d9d208` |
| `CPlatformController` | `0x5c5c120e` | `.text+0x1d9b778` |
| `CPlayerGameplayConfiguration` | `0x6f7804a9` | `.text+0x1d9c9c0` |
| `CPlayerSpawnPoint` | `0x27499f17` | `.text+0x1d9c7b8` |
| `CPlayerSpawnPointTrigger` | `0x380ad295` | `.text+0x1d9c7e0` |
| `CPremiumWingsuitUnlocker` | `0xdd0367a0` | `.text+0x1d9b4a8` |
| `CPrerequisiteObject` | `0x107d5ec7` | `.text+0x1d9cfd8` |
| `CRadio` | `0x415c0d7d` | `.text+0x1d9e220` |
| `CRadioStation` | `0x7adfbf1d` | `.text+0x1d9e1f8` |
| `CRagdollBoneProxyObject` | `0xfd716525` | `.text+0x1d9a710` |
| `CRagdollObject` | `0xe1b4823d` | `.text+0x1d9b700` |
| `CRaycastObject` | `0x6f56e107` | `.text+0x1d9c970` |
| `CReparentOnSpawn` | `0x4c7a0302` | `.text+0x1d9c808` |
| `CResupplyPoint` | `0x47d76c07` | `.text+0x1d9c790` |
| `CRetoolerLoadoutEditable` | `0xe982887f` | `.text+0x1d93378` |
| `CRetoolerLoadoutObject` | `0x9db7ecf5` | `.text+0x1d9a490` |
| `CRetoolerLoadoutVisibility` | `0xa17b6835` | `.text+0x1d93358` |
| `CRetoolerObject` | `0x8fc40583` | `.text+0x1d9b480` |
| `CRetoolerSettings` | `0xe1c1d536` | `.text+0x1d9e838` |
| `CRigidObject` | `0xce15d4ab` | `.text+0x1d9aa08` |
| `CRiverControllerObject` | `0xad86c677` | `.text+0x1cf40f0` |
| `CRiverObject` | `0x3d2d05dd` | `.text+0x1cf40a0` |
| `CRiverPalette` | `0x947eb5ec` | `.text+0x1cf40c8` |
| `CScopeWeaponComponent` | `0xf90310f8` | `.text+0x1d9b340` |
| `CScoreMultiplierEventCounter` | `0xd15d10be` | `.text+0x1d9e6f8` |
| `CScoreMultiplierObjectTracker` | `0xce51231f` | `.text+0x1d9e720` |
| `CScoreMultiplierTime` | `0x7438af5d` | `.text+0x1d9e680` |
| `CSequenceObject2` | `0xfe38f6e8` | `.text+0x1d9ddc0` |
| `CSmallButtonHint` | `0xe06a5a62` | `.text+0x1d9da28` |
| `CSoundAcousticsSettings` | `0x3adb2e1b` | `.text+0x1d9e1d0` |
| `CSoundLayerEmitter` | `0xabf98875` | `.text+0x1d9e0b8` |
| `CSoundOcclusionBox` | `0x71257494` | `.text+0x1d9e158` |
| `CSoundOcclusionCylinder` | `0x1fe0a325` | `.text+0x1d9e1a8` |
| `CSoundOcclusionDisc` | `0xf7b1d53d` | `.text+0x1d9e180` |
| `CSoundOcclusionPlane` | `0x7f6d6fd9` | `.text+0x1d9e130` |
| `CSpawnParam_ObjectCritical` | `0xae2ca5cd` | `.text+0x1d932d8` |
| `CSpawnParam_ObjectiveCritical` | `0x2ba2514d` | `.text+0x1d932b8` |
| `CSplineFollowObject` | `0x36972c0d` | `.text+0x1d9dbe0` |
| `CSplineTrigger` | `0x68e1d8b5` | `.text+0x1d9d780` |
| `CSplineTriggerPoint` | `0x765a56ae` | `.text+0x1d9d7a8` |
| `CSpotlightController` | `0x26600030` | `.text+0x1d9c6f0` |
| `CSpotlightWeaponComponent` | `0xf0b46fa1` | `.text+0x1d9b318` |
| `CStaticDecalObject` | `0x67439e99` | `.text+0x1cf44d8` |
| `CStormObject` | `0xe6db98da` | `.text+0x1d9c5d8` |
| `CStormRadiusControl` | `0xb529ebeb` | `.text+0x1d9cd80` |
| `CSupplyDropDescription` | `0xb3bcdc15` | `.text+0x1d9a3f0` |
| `CSupplyDroppableItem` | `0xb6329146` | `.text+0x1d9d1e0` |
| `CSupplyFactory` | `0x6b26887b` | `.text+0x1d9d1b8` |
| `CSupplyTutorialLimiter` | `0xacec6edc` | `.text+0x1d9eba8` |
| `CTacticalNodeDiscover` | `0xb63a4470` | `.text+0x1d93298` |
| `CTacticalNodeResource` | `0x1457fa88` | `.text+0x1d93278` |
| `CTimeLimitBeat` | `0xc793a7b7` | `.text+0x1d931f8` |
| `CTimeLimitCheckpoint` | `0xbd27c746` | `.text+0x1d93138` |
| `CTimeOfDayController` | `0x662461bc` | `.text+0x1d9ce70` |
| `CTornadoController` | `0x6d5053c4` | `.text+0x1d9ca10` |
| `CTornadoFan` | `0x3475a355` | `.text+0x1d9ca38` |
| `CTornadoObject` | `0x1663dc5a` | `.text+0x1d9c9e8` |
| `CTornadoReachPoint` | `0x38cff2af` | `.text+0x1d9ca88` |
| `CTornadoSpawnPoint` | `0x99af9915` | `.text+0x1d9ca60` |
| `CTornadoTrigger` | `0xbc2e3ea6` | `.text+0x1d9cab0` |
| `CTrafficLight` | `0x8d6f70e1` | `.text+0x1d9b7f0` |
| `CTrain` | `0xa12dae2e` | `.text+0x1d9c4c0` |
| `CTrainRoute` | `0xda3ef036` | `.text+0x1d9c4e8` |
| `CTransformComponent` | `0x9cbe6669` | `.text+0x1d933d8` |
| `CTriggerBarkWeaponComponent` | `0x3edb4e91` | `.text+0x1d9b110` |
| `CTurretWeaponComponent` | `0xef3fcd21` | `.text+0x1d9b2a0` |
| `CTutorialArrow` | `0xff283325` | `.text+0x1d9e7c0` |
| `CUIController` | `0x3b3ffc36` | `.text+0x1d9d960` |
| `CUIDialogue` | `0x2bb78aba` | `.text+0x1cf42f8` |
| `CUIInputPrompt` | `0xc60a83f4` | `.text+0x1d9da50` |
| `CUIManagerGO` | `0x32e601da` | `.text+0x1d9d938` |
| `CVehicleActionVolume` | `0x8e4a7740` | `.text+0x1d9b6b0` |
| `CVehicleCargoBay` | `0x435afa82` | `.text+0x1d9b688` |
| `CVehicleConfig` | `0xd26c935d` | `.text+0x1d9ce48` |
| `CVehicleData` | `0xbd9f83ce` | `.text+0x1d9d230` |
| `CVehiclePartProxy` | `0x2d814d36` | `.text+0x1d9c510` |
| `CVehiclePartRemoverWeaponComponent` | `0x66b538c5` | `.text+0x1d9b2c8` |
| `CVehicleRadio` | `0xb50c7289` | `.text+0x1d9e248` |
| `CVehicleTelescopicController` | `0x7171600e` | `.text+0x1d9c538` |
| `CVehicleWinchController` | `0x8ed490ff` | `.text+0x1d9c560` |
| `CVocalsLayer` | `0x60cd43c4` | `.text+0x1d9b570` |
| `CWallPenetrationWeaponComponent` | `0x54552ee3` | `.text+0x1d9b2f0` |
| `CWaterBox` | `0xf8e5e143` | `.text+0x1d9a8c8` |
| `CWaterEmitterObject` | `0x731455e9` | `.text+0x1cf4118` |
| `CWaterExtensions` | `0x47b833c0` | `.text+0x1d9a698` |
| `CWeapon` | `0xd7c165bb` | `.text+0x1d9ab48` |
| `CWeaponConfig` | `0xcc78c492` | `.text+0x1d9ce20` |
| `CWeaponRemover` | `0xb1d540a0` | `.text+0x1d9cdf8` |
| `CWeaponSetter` | `0xadf0e5ca` | `.text+0x1d9cdd0` |
| `CWeaponUIInfo` | `0x11c791e9` | `.text+0x1d9cf38` |
| `CWeatherPreset` | `0x325acf5a` | `.text+0x1d93318` |
| `CWindTunnelObject` | `0x48db9a8a` | `.text+0x1cf4140` |
| `CWingsuitObject` | `0xde095877` | `.text+0x1d9b430` |

## Registered but unnamed in docs (250) — hash + vtable known

| name-hash | vtable RVA |
|---|---|
| `0x02f9b4dd` | `.text+0x1cf4078` |
| `0x99bded97` | `.text+0x1cf4190` |
| `0xa47b90f2` | `.text+0x1cf41b8` |
| `0x2cbf995d` | `.text+0x1cf41e0` |
| `0x5eacb8fa` | `.text+0x1cf4208` |
| `0xd38016a6` | `.text+0x1cf4258` |
| `0x07c27de8` | `.text+0x1cf4280` |
| `0x4e460dc7` | `.text+0x1cf4370` |
| `0x07d1a043` | `.text+0x1cf4398` |
| `0x6031d465` | `.text+0x1cf43c0` |
| `0xcf55db73` | `.text+0x1cf4410` |
| `0xf7c04df1` | `.text+0x1cf4488` |
| `0xa15c6e67` | `.text+0x1cf44b0` |
| `0x691bbc58` | `.text+0x1d91ff8` |
| `0xb772aef0` | `.text+0x1d92018` |
| `0x80b99ee5` | `.text+0x1d92058` |
| `0x97be9b10` | `.text+0x1d92078` |
| `0x74805216` | `.text+0x1d920d8` |
| `0x3be1e4ae` | `.text+0x1d920f8` |
| `0x4cccbb67` | `.text+0x1d92118` |
| `0x346ddfff` | `.text+0x1d92138` |
| `0x81b233d8` | `.text+0x1d92158` |
| `0x4f58c611` | `.text+0x1d92178` |
| `0x1a5058ee` | `.text+0x1d92198` |
| `0x35a6d7cc` | `.text+0x1d921b8` |
| `0x2a0b1264` | `.text+0x1d921d8` |
| `0x12ea83f5` | `.text+0x1d921f8` |
| `0xb644c06e` | `.text+0x1d922b8` |
| `0x455de7de` | `.text+0x1d922d8` |
| `0x00f8acdb` | `.text+0x1d922f8` |
| `0xe68544a2` | `.text+0x1d92318` |
| `0x000a5550` | `.text+0x1d92358` |
| `0xf620a8ee` | `.text+0x1d923b8` |
| `0xb6372001` | `.text+0x1d923f8` |
| `0x38fc228a` | `.text+0x1d92438` |
| `0x8729b5f7` | `.text+0x1d92458` |
| `0xd79e3a93` | `.text+0x1d92498` |
| `0x9bbab84b` | `.text+0x1d924b8` |
| `0xbf1b9ad7` | `.text+0x1d924d8` |
| `0x0cefba07` | `.text+0x1d924f8` |
| `0x1777bc7e` | `.text+0x1d92518` |
| `0xf93d9028` | `.text+0x1d92538` |
| `0x6177900a` | `.text+0x1d925f8` |
| `0x46a55ac4` | `.text+0x1d92618` |
| `0x220f8556` | `.text+0x1d92678` |
| `0x1aed15d9` | `.text+0x1d92698` |
| `0xb99a0a0d` | `.text+0x1d926f8` |
| `0x686feec9` | `.text+0x1d92718` |
| `0x986c9d4e` | `.text+0x1d92778` |
| `0x5b5bdb65` | `.text+0x1d927d8` |
| `0x256b1fdc` | `.text+0x1d927f8` |
| `0x55975e13` | `.text+0x1d92838` |
| `0xf9dd308d` | `.text+0x1d92858` |
| `0x2a58dbfa` | `.text+0x1d92898` |
| `0x954f8db4` | `.text+0x1d928b8` |
| `0xc6c78b34` | `.text+0x1d928d8` |
| `0x111208cc` | `.text+0x1d92998` |
| `0x8de63bb8` | `.text+0x1d929b8` |
| `0x1d756505` | `.text+0x1d929f8` |
| `0x07d438f3` | `.text+0x1d92a18` |
| `0x5b9dfc31` | `.text+0x1d92a58` |
| `0x4e80def6` | `.text+0x1d92a78` |
| `0x3b2a4371` | `.text+0x1d92ab8` |
| `0xe97b37d8` | `.text+0x1d92b78` |
| `0x843e47fe` | `.text+0x1d92b98` |
| `0x95d4d0d1` | `.text+0x1d92bd8` |
| `0x9e54948c` | `.text+0x1d92c18` |
| `0x6630d28f` | `.text+0x1d92c58` |
| `0x048b4fba` | `.text+0x1d92c78` |
| `0x8523ee81` | `.text+0x1d92cb8` |
| `0x0e79ae10` | `.text+0x1d92d98` |
| `0x6dd4cf0f` | `.text+0x1d92db8` |
| `0x5a6c7dcd` | `.text+0x1d92dd8` |
| `0xce3cdf1e` | `.text+0x1d92df8` |
| `0xe2db4a5c` | `.text+0x1d92e18` |
| `0xf82461a5` | `.text+0x1d92fb8` |
| `0xd6429801` | `.text+0x1d93018` |
| `0x0b9a17a7` | `.text+0x1d93078` |
| `0x21db23f6` | `.text+0x1d93398` |
| `0xe1350ae9` | `.text+0x1d933b8` |
| `0xf008ed2d` | `.text+0x1d93558` |
| `0x07cbec40` | `.text+0x1d9a350` |
| `0xc37e0c65` | `.text+0x1d9a378` |
| `0x1e3cf0b1` | `.text+0x1d9a3a0` |
| `0x67f286d4` | `.text+0x1d9a3c8` |
| `0xfd0d8f9b` | `.text+0x1d9a440` |
| `0x45f56b99` | `.text+0x1d9a468` |
| `0x2a8cac8a` | `.text+0x1d9a4b8` |
| `0xc8fe2b14` | `.text+0x1d9a4e0` |
| `0xe4067658` | `.text+0x1d9a508` |
| `0xc0013a3d` | `.text+0x1d9a530` |
| `0xde90985e` | `.text+0x1d9a558` |
| `0xe8e2be4c` | `.text+0x1d9a5a8` |
| `0xc453f7a7` | `.text+0x1d9a5d0` |
| `0x8f1b7534` | `.text+0x1d9a620` |
| `0x7f646478` | `.text+0x1d9a648` |
| `0xbee36fa1` | `.text+0x1d9a6c0` |
| `0x6ca5cb60` | `.text+0x1d9a738` |
| `0x7b73c776` | `.text+0x1d9a760` |
| `0x4e4ba665` | `.text+0x1d9a788` |
| `0xa48757e2` | `.text+0x1d9a7b0` |
| `0x95a82d13` | `.text+0x1d9a7d8` |
| `0x6c6ed4ed` | `.text+0x1d9a800` |
| `0x94535c5e` | `.text+0x1d9a828` |
| `0x42c39e75` | `.text+0x1d9a850` |
| `0xd86b6703` | `.text+0x1d9a8a0` |
| `0xe6e4c62f` | `.text+0x1d9a8f0` |
| `0xb28e39bb` | `.text+0x1d9a918` |
| `0xcd0c5f45` | `.text+0x1d9a940` |
| `0x14d90331` | `.text+0x1d9a968` |
| `0xf02b9908` | `.text+0x1d9a990` |
| `0x5bafd967` | `.text+0x1d9a9b8` |
| `0x1398bf86` | `.text+0x1d9a9e0` |
| `0x94b7c8f3` | `.text+0x1d9aa30` |
| `0xd1519922` | `.text+0x1d9aa80` |
| `0xdfa1e8ca` | `.text+0x1d9aaa8` |
| `0xe309a059` | `.text+0x1d9ab20` |
| `0x02bc365f` | `.text+0x1d9ab70` |
| `0x806d9774` | `.text+0x1d9ab98` |
| `0x19d18e54` | `.text+0x1d9abc0` |
| `0x5ae484a1` | `.text+0x1d9abe8` |
| `0x2ea73607` | `.text+0x1d9ac10` |
| `0xee2ad99a` | `.text+0x1d9ac38` |
| `0x48138f81` | `.text+0x1d9ac60` |
| `0xc703da89` | `.text+0x1d9ac88` |
| `0x59508517` | `.text+0x1d9acb0` |
| `0x9cea698c` | `.text+0x1d9acd8` |
| `0x0de9f70e` | `.text+0x1d9ad00` |
| `0x32abb7aa` | `.text+0x1d9ad28` |
| `0x46e7af62` | `.text+0x1d9ad50` |
| `0xd200ae6e` | `.text+0x1d9ad78` |
| `0x0850b75b` | `.text+0x1d9ada0` |
| `0x83b0607b` | `.text+0x1d9adc8` |
| `0x61ecf475` | `.text+0x1d9adf0` |
| `0xd831ed0d` | `.text+0x1d9ae18` |
| `0xc441dbb9` | `.text+0x1d9ae40` |
| `0xf3b5c626` | `.text+0x1d9ae68` |
| `0xea1cc667` | `.text+0x1d9ae90` |
| `0x4a2b7fb6` | `.text+0x1d9aeb8` |
| `0x83336219` | `.text+0x1d9aee0` |
| `0x91bc484d` | `.text+0x1d9af08` |
| `0xe8b26730` | `.text+0x1d9af30` |
| `0xdfcb5941` | `.text+0x1d9af58` |
| `0xaee9b391` | `.text+0x1d9af80` |
| `0x9ddb9e54` | `.text+0x1d9aff8` |
| `0xf52158e3` | `.text+0x1d9b5c0` |
| `0x50e77de6` | `.text+0x1d9b6d8` |
| `0x07154747` | `.text+0x1d9b728` |
| `0x82c3fe1a` | `.text+0x1d9b750` |
| `0x8ef0951a` | `.text+0x1d9b7a0` |
| `0xec0859d5` | `.text+0x1d9b7c8` |
| `0x4d6706d4` | `.text+0x1d9c588` |
| `0xfbc0d0da` | `.text+0x1d9c600` |
| `0x72cbdd76` | `.text+0x1d9c628` |
| `0x40ea1136` | `.text+0x1d9c650` |
| `0x766f4d45` | `.text+0x1d9c678` |
| `0xc7894854` | `.text+0x1d9c6a0` |
| `0xd036f9f0` | `.text+0x1d9c740` |
| `0xdfdb5b1d` | `.text+0x1d9c768` |
| `0xc846a6a2` | `.text+0x1d9c8d0` |
| `0x605bc7a7` | `.text+0x1d9c920` |
| `0xe58440c8` | `.text+0x1d9c998` |
| `0x86e2a08d` | `.text+0x1d9cb28` |
| `0x699bc077` | `.text+0x1d9cc18` |
| `0xd172d3cb` | `.text+0x1d9cc40` |
| `0x213e7453` | `.text+0x1d9ccb8` |
| `0x347b0f02` | `.text+0x1d9cd08` |
| `0x60f329ba` | `.text+0x1d9cd30` |
| `0x1b002871` | `.text+0x1d9cd58` |
| `0xa1801910` | `.text+0x1d9cda8` |
| `0xc3cf96cb` | `.text+0x1d9cec0` |
| `0x776575aa` | `.text+0x1d9cf10` |
| `0xd9b226ed` | `.text+0x1d9cf60` |
| `0x6518c8ea` | `.text+0x1d9cf88` |
| `0x3f942252` | `.text+0x1d9cfb0` |
| `0x12724dc9` | `.text+0x1d9d000` |
| `0x6f7f6fce` | `.text+0x1d9d028` |
| `0xeb073edc` | `.text+0x1d9d050` |
| `0xcf553ec9` | `.text+0x1d9d078` |
| `0x1148972b` | `.text+0x1d9d0a0` |
| `0xfba03f66` | `.text+0x1d9d0c8` |
| `0xf3cca094` | `.text+0x1d9d0f0` |
| `0x5fbf9ca0` | `.text+0x1d9d140` |
| `0xbb6fc195` | `.text+0x1d9d168` |
| `0xc08dd40a` | `.text+0x1d9d190` |
| `0xb368393a` | `.text+0x1d9d258` |
| `0x49edf1e1` | `.text+0x1d9d320` |
| `0x8e4098ec` | `.text+0x1d9d348` |
| `0xff39fd28` | `.text+0x1d9d3c0` |
| `0xaa707c50` | `.text+0x1d9d3e8` |
| `0xe99a452a` | `.text+0x1d9d410` |
| `0x082e6b9c` | `.text+0x1d9d438` |
| `0xaad8a83b` | `.text+0x1d9d460` |
| `0x97318625` | `.text+0x1d9d488` |
| `0x64bdc798` | `.text+0x1d9d4d8` |
| `0x0b624785` | `.text+0x1d9d500` |
| `0x675dc831` | `.text+0x1d9d528` |
| `0x0e0803d7` | `.text+0x1d9d550` |
| `0x9f9e0b45` | `.text+0x1d9d578` |
| `0x7af2bb2e` | `.text+0x1d9d5a0` |
| `0x81663b05` | `.text+0x1d9d5c8` |
| `0x35bf72f5` | `.text+0x1d9d5f0` |
| `0xd70b29bd` | `.text+0x1d9d618` |
| `0x40bf6526` | `.text+0x1d9d690` |
| `0xf99dfb8b` | `.text+0x1d9d6b8` |
| `0x2c1178a6` | `.text+0x1d9d6e0` |
| `0xe7b028e9` | `.text+0x1d9d708` |
| `0xbc29385d` | `.text+0x1d9d730` |
| `0x9f3846aa` | `.text+0x1d9d758` |
| `0xee1da58c` | `.text+0x1d9d7d0` |
| `0x845318f4` | `.text+0x1d9d7f8` |
| `0xee285e4a` | `.text+0x1d9d820` |
| `0x75b90736` | `.text+0x1d9d848` |
| `0x396116d9` | `.text+0x1d9d870` |
| `0x4b52cf32` | `.text+0x1d9d898` |
| `0x4838a1df` | `.text+0x1d9d8c0` |
| `0x24031bca` | `.text+0x1d9d8e8` |
| `0x0daa555a` | `.text+0x1d9d910` |
| `0x063e94b4` | `.text+0x1d9d988` |
| `0x87cfaceb` | `.text+0x1d9d9b0` |
| `0xb1abf9ba` | `.text+0x1d9d9d8` |
| `0x2c799c71` | `.text+0x1d9da78` |
| `0xd93b9dda` | `.text+0x1d9db40` |
| `0x5bbbf6d5` | `.text+0x1d9db90` |
| `0xe23fc2f6` | `.text+0x1d9dc58` |
| `0xae86c811` | `.text+0x1d9dca8` |
| `0x03b5fcb3` | `.text+0x1d9dcd0` |
| `0x045326c2` | `.text+0x1d9dcf8` |
| `0xe3b64f61` | `.text+0x1d9dd20` |
| `0xd7916433` | `.text+0x1d9dd48` |
| `0x9f566712` | `.text+0x1d9dd98` |
| `0x41664528` | `.text+0x1d9dde8` |
| `0x8527ca8f` | `.text+0x1d9df50` |
| `0xad362090` | `.text+0x1d9e018` |
| `0xd9011260` | `.text+0x1d9e270` |
| `0xffc2a1d8` | `.text+0x1d9e298` |
| `0x1dc0c59e` | `.text+0x1d9e2c0` |
| `0xc0e9c300` | `.text+0x1d9e2e8` |
| `0x41a1ab43` | `.text+0x1d9e388` |
| `0xb5a56139` | `.text+0x1d9e3b0` |
| `0x1ffa77f6` | `.text+0x1d9e3d8` |
| `0x9d8362bd` | `.text+0x1d9e428` |
| `0x90714a93` | `.text+0x1d9e478` |
| `0xebfe4eed` | `.text+0x1d9e4a0` |
| `0x861ddda0` | `.text+0x1d9e540` |
| `0x270a5342` | `.text+0x1d9e860` |
| `0x15175f52` | `.text+0x1d9eb30` |
| `0x51330438` | `.text+0x1d9ecc0` |
| `0x88f2c5c4` | `.text+0x1d9ed88` |
| `0x4d56dcd8` | `.text+0x1d9ee00` |
# Online, Platform & Social — the backend live-service + platform integration

**Status:** draft · **Evidence key:** each claim tagged (proven | inferred | speculative). Every anchor cites a
`FUN_`/`DAT_`/`PTR_LAB_` from `output/_ghidra_jc4/jc4_all_functions_decomp.txt`.

> **Methodology caveat (functions-only export).** Recoverable here: the online **manager registration**
> skeletons, the **platform-adapter** call graph, reachable **CPU logic** (Steam bootstrap, auth handshake,
> the async HTTP/JSON client, the telemetry serializer), and **string constants** (backend URL, REST resource
> roots, DTO type names, achievement IDs, baked SDK build paths). **Not** recoverable: the per-manager tick /
> handler **bodies** (`COnlineSuiteManager`, `CFriendManager`, `CPresenceManager`, `CLeaderboardManager`, …)
> which sit behind data-section vtables (`PTR_LAB_141d90*`); the full endpoint URL **templates** with IDs and
> request bodies; and the achievement→Steam-stat **mapping table** (data). Those are tagged *walled/open*.

---

## Overview  (proven unless noted)

JC4's entire online/live-service and platform layer is built on **Square Enix / Eidos "Online Suite v6"**
(the `osdk::` namespace — "Online SDK"). The exact middleware build is baked into an assert string:

```
d:\needy\3rdparty\onlinesuitev6\onlinesuitev6-6.1.0.2.0.8.1707831\include\osdk\impl\steam\ossteamsupport.cpp
```
(`FUN_149668e40` @0x149668e40). So the SDK is **OnlineSuiteV6 `6.1.0.2.0.8.1707831`**.

The design is a two-layer abstraction:

1. **Platform adapters** (`osdk::impl::*`) wrap a concrete storefront/OS. This PC binary contains exactly two:
   `osdk::steam::osSteamSupport` (Steam) and `osdk::…::osWindowsSupport` (OS/window services). **No Epic, Xbox
   Live, PSN, or EOS adapter is present in this build** — the only `psn`/`epic`/`xbox` string hits are generic
   (2 stray `psn`), and the console/EGS SKUs would ship different adapter objects. (steam+windows adapters =
   proven; absence of others = inferred *for this PC/Steam binary*.)
2. **The backend service client** talks REST+JSON to the JC4 live-service at
   **`https://jc4v6.os.eidos.com/game`** (`FUN_1409ff030` region). All backend model types live under
   `osdk::jc4v6::` and are marshalled by a codegen'd `osdk::jc4v6::Client`.

The engine-side game managers (`COnlineSuiteManager`, `CLeaderboardManager`, `CAchievementsManager`, …) are
thin front-ends that drive this OSDK stack. They are all name-registered through the standard JC4 manager
registrar; the registration is present, but the tick bodies are behind factory vtables (walled).

Related privacy weblets are also baked in: `https://weblet.square-enix.com/geojmp.php?d=DOCUMENTS&l=privacyingame`,
`www.jp.square-enix.com`, `avalanchestudios.com/privacy-policy/` (line ~881144).

---

## Key classes & functions

| Anchor | Role | Evidence |
|---|---|---|
| `osdk::steam::osSteamSupport` | Steam platform adapter (init/shutdown/callbacks) | `FUN_149668e40` (SteamAPI_Init), proc-table `~0x140a44*` |
| `osdk::steam::osSteamIdentityProvider::AuthenticationRequest` | Steam encrypted-app-ticket → OSDK identity | lines 1146893, 1186767 |
| `os::client::identity` / `OSJWTClaimsDTO` | OSDK identity, JWT bearer token | line 1841943, `"Bearer "` @1828023 |
| `osdk::osClientBase::executeJSONAndConvertAsync<DTO>` | async REST call → typed DTO | line 1139527 |
| `osdk::osHTTPRequestBuilder` / `osHTTPClient` / `osHTTPResponse` | HTTP request/response stack | 28/… refs |
| `osdk::osTFuture` / `osPromise` / `osTaskImpl` | async future/promise/task framework | pervasive |
| `osMetricQueue` / `osMetricFilterByName` | telemetry pipeline | 35 / 27 refs |
| `FUN_1409ff030` @0x1409ff030 (size=5300) | telemetry snapshot JSON serializer | caller `0x14949d990` |
| `FUN_1409b8480` @0x1409b8480 | `ShowDLCStoreDialogAsync` deep-link | callers below |
| `COnlineSuiteManager` … `CPresenceManager` | engine-side online managers | manager-list site |
| `CSquareEnixMembership` | **SE Members** program manager (new find) | `PTR_LAB_141d90ae8` |

---

## How it works (from the decomp)

### 1. Platform abstraction = OSDK with a Steam adapter  (proven)

The Steam adapter is loaded dynamically. A proc-address table (`~FUN` at line 4306049) binds the exports of
`steam_api64` into `DAT_142cbab*` slots:

```
DAT_142cbab78 = GetProcAddress(steam_api64, "SteamAPI_Init");
DAT_142cbab98 = GetProcAddress(steam_api64, "SteamAPI_RunCallbacks");
DAT_142cbabd8 = GetProcAddress(steam_api64, "SteamAPI_ISteamUser_RequestEncryptedAppTicket");
```
`osSteamSupport::SteamAPI_Init` (`FUN_149668e40`) is ref-counted and, on failure, logs the tell-tale
*"Failed to initialize steam — check that steam is running, you have a steam_appid.txt file …"*. Interface
access goes through `SteamInternal_CreateInterface("SteamClient017")` → `GetISteamUser(…, "SteamUser019")`
(`~FUN` @4266488). The full Steamworks versioned-interface string set is baked in (proven the platform is
Steam and only Steam here): `SteamClient017`, `SteamUser019`, `SteamUtils009`, `SteamFriends015`,
`SteamApps…008`, `SteamUserStats…011`, `SteamRemoteStorage014`, `SteamMatchmaking009`,
`SteamMatchmakingServers002`, `SteamHTTP002`, `SteamInventory_v002`, `SteamUGC010`, `SteamController005`,
`SteamScreenshots003`, `SteamHTMLSurface004`, `SteamNetworking005`, `SteamMusic001`, `SteamAppList001`,
`SteamParentalSettings001`, `SteamVideo002`, `SteamUnifiedMessages001`. `SteamAPI_RunCallbacks()` is pumped
from the frame loop (`FUN` @4439273).

Note the interface *strings* are linked but that does **not** prove every subsystem is used — they are the
full Steamworks header, embedded by the import shim. Confirmed-driven: User (auth/ticket), Friends
(persona name), Apps, UserStats (achievements). (proven driven for those four; others = present-but-unconfirmed.)

### 2. Authentication — Steam encrypted app ticket → OSDK JWT  (proven)

The identity handshake is the classic OSDK flow:

* `osdk::steam::osSteamIdentityProvider::AuthenticationRequest(osUserId, osString)` (line 1146893) issues
  `SteamAPI_ISteamUser_RequestEncryptedAppTicket`.
* The result arrives at `AuthenticationRequest::OnRequestEncryptedAppTicketCB(Steam::EncryptedAppTicketResponse_t*, bool)`
  (line 1186767).
* The ticket is posted to the OSDK backend, which returns a signed **JWT**. The claims type is
  `os::client::identity::OSJWTClaimsDTO` (`DebugPrintOSJWTClaims`, line 1841943); the token is then attached
  to subsequent requests as an `"Bearer "` `Authorization` header (line 1828023). `osIdentity` (198 refs) is
  the resulting authenticated-session object.

So: **Steam encrypted app ticket is the credential; the OSDK JWT is the session bearer token** for all
`jc4v6` REST calls.

### 3. The async REST/JSON client  (proven)

Every backend call goes through one generic path:
```
osdk::osClientBase::executeJSONAndConvertAsync<DTO>(const osHTTPRequestBuilder&)
      -> osTFuture< osResponse<DTO>, osTaskImpl >
```
(line 1139527). The request is built with `osHTTPRequestBuilder` (method via `osHTTPMethod`, headers
`Content-Type: application/json`, priority `osHTTPRequestPriority`, filters `osHTTPFilter`), executed by
`osHTTPClient`, and the `osJSONResponse` body is converted to a typed DTO by
`osTFuture::thenOnSuccessReturn<osResponse<DTO>>`. Everything is continuation-passing over
`osTFuture`/`osPromise`/`osTaskImpl` with `osTaskException`/`osStatus` for error handling. JSON marshalling
uses the OSDK json layer (`osJSONObjectConverter`, `osJSONEnumConverter`, `osdk_JsonObject_SetMemberString`,
`osdk_JsonObject_Find`, `osdk_JsonValue_AsString`); memory via `osdk_Allocator_Malloc/Free`; strings via the
`osString` family. The many baked source paths corroborate the stack: `osdk/ospromise.inline.hpp`,
`osdk/osfuture.inline.hpp`, `osdk/ostask.hpp`, `osdk/osdelegate.hpp`, `osdk/osscheduler.hpp`,
`osdk/oscommunity.hpp`, `osdk/osmetricfilterbyname.hpp`, `osdk/json/osjsonenumconverter.hpp`.

### 4. Backend resource map  (proven)

Baked REST resource roots (path fragments) and their `osdk::jc4v6::` DTOs:

| Resource root | DTO(s) | Purpose |
|---|---|---|
| `/feats` | `FeatLeaderboardDTO`, `FeatValueDTO`, `FeatNotificationDTO` | **Leaderboards / "Feats"** scoring |
| `/events` | `EventDTO`, `EventChallengeDTO` | Live / timed events + their challenges |
| `/participations` | `EventParticipationDTO`, `EventParticipationResponseDTO` (+`…Status`/`…Result` enums) | Player's event entry/progress |
| `/infocasts` | `InfocastDTO` (+`LocalizedContentType`) | In-game news / info feed |
| `/membership` | `MembershipResource_MembershipAccountResponseDTO` (+`MembershipAccountStatus`) | **Square Enix Members** account |
| `/notifications` | `FeatNotificationDTO` | Server push notifications |
| `/profiles`, `/profile/item.onlinec`, `/profile/offer.onlinec`, `/profile/task.onlinec`, `/profile/taskgroup.onlinec`, `/profile/upgrade.onlinec` | `ProfileValueDTO`, `ItemDTO`, `ItemQuantityDTO` | Player profile + live-service inventory / offers / tasks / upgrades |

The `.onlinec` suffix = "online content" resource. The `/profile/{item,offer,task,taskgroup,upgrade}` set is a
full live-service progression/store backbone (owned items, purchasable offers, task/taskgroup goals,
upgrades). (proven the roots + DTOs exist; whether a live store was ever populated is out of scope.)

### 5. Leaderboards = the "Feats" service  (proven)

`CLeaderboardManager` (registered) drives `/feats`. Query/submit is the generic client path specialised to the
leaderboard DTO — both a single row and a page:
```
osClientBase::executeJSONAndConvertAsync<FeatLeaderboardDTO>(…)            (line 1139527)
osClientBase::executeJSONAndConvertAsync<std::vector<FeatLeaderboardDTO>>(…) (line 1140691)
```
Results flow back as `osTFuture<osResponse<FeatLeaderboardDTO>>`. Locally, scores are cached/persisted through
`CLeaderboardPersistanceController` whose `SFeatSaveData` (aka `FeatSaveData`) struct is registered in the
reflection type registry (`ArGetTypeId<CLeaderboardPersistanceController::SFeatSaveData>`, lines 1119684 /
1142604) → i.e. serialised into the ADF save. This is how leaderboard/feat state survives offline and syncs.

**Tie to challenges:** "Feats" *are* the challenge scoring layer — `CChallengeManager` is registered
immediately beside `CLeaderboardManager`, and two achievements key off it: `ach_feat_track_and_beat`,
`ach_feat_try_all`. `FeatValueDTO`/`FeatNotificationDTO` carry the per-feat score and the "you were beaten"
push. (challenge↔feat linkage = inferred from co-registration + achievement naming; the feat submit call
site body is walled.)

### 6. Telemetry / metrics — `FUN_1409ff030`  (proven)

The metrics pipeline is `osMetricQueue` (enqueue) + `osMetricFilterByName` (server-side name filtering).
`FUN_1409ff030` (@0x1409ff030, size=5300, caller `0x14949d990`) is the **snapshot serializer**: it builds one
big OSDK JSON metrics event from live game state. Keys are typed by prefix — `s_`=string, `i32_`/`i64_`=int,
`f_`=float, `b_`=bool — e.g. (full captured set):

```
s__UUID  s__Zone  f__X f__Y f__Z  s_playthrough_id  s_sem_id  s_VOLanguage  s_Subtitle
i64_chaos_points_total  i64_feat_count  i64_encounter_count  i64_encounter_duration_ms
i32_quests_completed_count  i32_nodes_completed_count  i32_operation_completes_count
i32_operation_mission_completes[_count]  i32_operation_mission_attempts
i32_collectible_count  i32_weapons_unlocked_count  i32_weapons_used_count
i32_vehicles_unlocked_count  i32_vehicles_driven_count
i32_supply_drops_requested_count  i32_supply_drops_collected_count
i32_frontline_troops_earned  i32_frontline_troops_deployed
s_current_weapon_1  s_current_weapon_2  b_secondary_fire_used
s_last_quest_completed  s_last_node_visited  s_last_node_completed
s_last_encounter_id  s_encounter_id  s_encounter_status  s_encounter_fail_reason
s_operation_mission_checkpoint_id / _status / _fail_reason
s_supply_drop_object_id / _pilot / _status  s_black_market_item_id
s_parachute_skin_id  s_wingsuit_skin_id
s_fp_achievement_id  s_fp_previous_achievement_id
s_dlc_demons_and_danger_mission_status / _objective_id / _fail_status
```
`s_sem_id` = the Square Enix Membership id; `s_playthrough_id`/`s__UUID` tag the session; `f__X/_Y/_Z` +
`s__Zone` stamp world position. (This is the same serializer the missions/progression doc found — expanded
here with the full key list.)

### 7. Achievements  (proven IDs; Steam mapping inferred)

`CAchievementsManager` (registered, `PTR_LAB_141d90ac8`) is the front-end; on PC it maps to Steam
`ISteamUserStats` (`SteamUserStats…011` interface string present). Telemetry mirrors unlocks via
`s_fp_achievement_id` / `s_fp_previous_achievement_id`. **41 distinct `ach_*` IDs** are baked in:

```
ach_beat_game  ach_one_hundred_percent  ach_complete_campaign_sandstorm
ach_complete_campaign_tornado  ach_complete_campaign_tropical_storm
ach_quest_all_intros  ach_quest_finale_garland  ach_quest_finale_javi  ach_quest_finale_sargento
ach_all_factories  ach_all_nodes  ach_all_pilots  ach_all_supply_drop  ach_all_weapons
ach_discover_half  ach_discover_everything  ach_max_chaos_milestone  ach_import_harbor
ach_collectible_all_pointing_statue  ach_collectible_all_speed_ring  ach_collectible_all_surveillance
ach_collectible_all_vehicle_ring  ach_collectible_all_wingsuit_ring
ach_feat_track_and_beat  ach_feat_try_all
ach_retooler_all_unlocked  ach_retooler_air_lifter_only  ach_retooler_booster_only  ach_retooler_retract_only
ach_highest_point  ach_hover_or_die  ach_plane_chicken  ach_cow_gun  ach_shotgun_secondary_vehicle
ach_who_the_idiot  ach_speak_of_this  ach_aoc_tier
ach_dlc1_destroy_some_vehicles  ach_dlc1_destroy_somemore_vehicles  ach_dlc1_gold_all_challenge  ach_dlc3_completed_outro
```
(`ach_dlc1_*`/`ach_dlc3_*`/`ach_aoc_tier` = DLC/Age-of-… content; `dlc_demons_and_danger` shows up in
telemetry too.) The achievement→Steam-stat-id map itself is data (walled).

### 8. Entitlement / DLC ownership  (proven)

* Ownership is a **condition-VM predicate**: `CConditional_IsDLCUnlocked` is registered in the reflection
  factory via `FUN_140f27f60("CConditional_IsDLCUnlocked", 0x1a)` → cached at `DAT_142cb94bc` (line 1419742)
  and `DAT_142cb5ae8` (line 605915). Missions/conditionals gate on it exactly like any other
  `CConditional_Is*`. (proven the predicate is registered; its evaluator body is behind the factory vtable.)
* `CDownloadableContentManager` (registered, `PTR_LAB_141d909e8`) owns DLC state.
* **Unowned DLC → store deep-link:** `FUN_1409b8480` @0x1409b8480 calls
  `FUN_1409a58e0(dialog, "ShowDLCStoreDialogAsync", …)`, invoked from `FUN_140dcbe50`, `FUN_140dd2310`,
  `FUN_14a633e60` — i.e. touching locked DLC opens the platform store page. (proven)

### 9. Presence, friends, profile, player-reporting  (registration proven; bodies walled)

* **Friends / presence / community:** `CFriendManager` (`PTR_LAB_141d90a08`) and `CPresenceManager`
  (`PTR_LAB_141d90ee8`) are registered; the OSDK backing is `osCommunity` (`osdk/oscommunity.hpp`, 9 refs).
  On PC these ride Steam (`SteamFriends015`, `SteamAPI_ISteamFriends_GetPersonaName`). Rich-presence *strings*
  aren't in this export (walled).
* **Profile:** `CProfileManager` is registered by the app-system factory pass (region1, near
  `PTR_LAB_141d90468`), backed by `/profiles` + `ProfileValueDTO` and the `/profile/*.onlinec` inventory set.
* **Player reporting:** `CPlayerReportingManager` (`PTR_LAB_141d90a88`) is registered; string refs at lines
  1210521 / 1215860 / 1215891 / 1227898. Reporting UI/endpoint bodies are walled.
* **Square Enix Membership (new find):** `CSquareEnixMembership` (`PTR_LAB_141d90ae8`) is a first-class
  manager backed by `/membership` + `MembershipResource_MembershipAccountResponseDTO`
  (+`MembershipAccountStatus`). Field names `membership_token` / `membership_id` / `membership_email` /
  `membershipToken` appear, and `s_sem_id` threads the membership id into telemetry.

---

## The manager registry  (proven)

The engine-side online/social managers are all registered through the standard JC4 name→factory registrar
(the same `thunk_FUN_148f6e780(container, "CName", factoryObj)` list used across the codebase; sibling of the
app-system-factory registrar `FUN_148f960c0` @0x148f960c0 / `FUN_148f958e0` @0x148f958e0). The manager-list
site's enclosing exported symbol is `FUN_146cd0bea` @0x146cd0bea (its head is an obfuscated
CFG/anti-tamper stub — `POPCOUNT`/xor prologue — with the registration list inlined after it; content lines
~3209060–3209740, caller `FUN_149b2f323`). Online/social entries and their factory pointers:

| Manager | Factory ptr |
|---|---|
| `COnlineSuiteManager` | `PTR_LAB_141d909c8` |
| `CDownloadableContentManager` | `PTR_LAB_141d909e8` |
| `CFriendManager` | `PTR_LAB_141d90a08` |
| `CPlayerReportingManager` | `PTR_LAB_141d90a88` |
| `CAchievementsManager` | `PTR_LAB_141d90ac8` |
| `CSquareEnixMembership` | `PTR_LAB_141d90ae8` |
| `COnlineFeatureManager` | `PTR_LAB_141d90b08` |
| `CLeaderboardManager` | `PTR_LAB_141d90dc8` |
| `CPresenceManager` | `PTR_LAB_141d90ee8` |
| `COnlineAppSystemFactory`, `COnlinePlatformSystem`, `CProfileManager` | registered by `FUN_148f958e0`/`FUN_148f960c0` (region1) |

`CNotificationManager`, `CStatisticManager`, `CChallengeManager` register adjacently and feed the same OSDK
resources (`/notifications`, feats, events).

---

## Call-graph highlights  (proven)

* **Boot:** frame/init → `osSteamSupport::SteamAPI_Init` (`FUN_149668e40`) → proc-table bind (`~`@4306049) →
  `SteamInternal_CreateInterface("SteamClient017")` (`~`@4266488) → per-interface getters.
* **Auth:** `osSteamIdentityProvider::AuthenticationRequest` → `SteamAPI_ISteamUser_RequestEncryptedAppTicket`
  → `OnRequestEncryptedAppTicketCB` → OSDK backend → `OSJWTClaimsDTO` → `"Bearer "` header on `osIdentity`.
* **Any REST call:** `osHTTPRequestBuilder` → `osClientBase::executeJSONAndConvertAsync<DTO>` →
  `osHTTPClient` → `osJSONResponse` → `thenOnSuccessReturn<osResponse<DTO>>` → game manager continuation.
* **Frame pump:** frame loop → `SteamAPI_RunCallbacks()` (`~`@4439273).
* **Telemetry:** `0x14949d990` → `FUN_1409ff030` (JSON metrics snapshot) → `osMetricQueue`.
* **DLC gate:** locked DLC touch (`FUN_140dcbe50`/`FUN_140dd2310`/`FUN_14a633e60`) → `FUN_1409b8480` →
  `FUN_1409a58e0("ShowDLCStoreDialogAsync")`; conditional check via registered `CConditional_IsDLCUnlocked`.

---

## Open questions / lower-confidence

* **Per-manager bodies walled.** `COnlineSuiteManager`/`CFriendManager`/`CPresenceManager`/`CLeaderboardManager`
  tick + handler code is behind `PTR_LAB_141d90*` factory vtables — not in a functions-only export.
  Route: resolve the vtables in a live x64dbg pass, or via the RTPC/ADF data. (walled)
* **Full endpoint templates.** Only the resource roots (`/feats`, `/events`, `/membership`, …) and the base
  URL (`jc4v6.os.eidos.com/game`) are string-visible; the `{id}`-templated paths and request bodies are built
  at the call sites (mostly inlined into the walled bodies). (walled)
* **Achievement → Steam stat map.** The 41 `ach_*` ids are proven; their mapping to Steamworks stat/achievement
  API names is a data table, not in the functions export. (walled)
* **Console/EGS adapters.** Absence of Epic/Xbox/PSN here is *for this Steam PC binary*; other SKUs would carry
  `osdk::impl::{eos,xboxlive,psn}` adapters. (inferred)
* **Matchmaking/multiplayer.** `SteamMatchmaking*` interface strings are linked, but no driven matchmaking call
  site surfaced — JC4 is single-player, so these are almost certainly the unused Steamworks header. (inferred)

---

## Appendix — decomp anchors

**SDK build path (proven):** `onlinesuitev6-6.1.0.2.0.8.1707831`, `…\include\osdk\impl\steam\ossteamsupport.cpp`
(`FUN_149668e40` @0x149668e40).

**Backend (proven):** base URL `https://jc4v6.os.eidos.com/game`; privacy weblets
`weblet.square-enix.com/geojmp.php`, `www.jp.square-enix.com`, `avalanchestudios.com/privacy-policy/` (~line 881144).

**Steam adapter (proven):** `FUN_149668e40` (SteamAPI_Init, refcounted, `steam_appid.txt` error string);
proc-table bind `~`@4306049 (`DAT_142cbab78/98/d8`); `SteamInternal_CreateInterface("SteamClient017")` `~`@4266488;
callbacks pump `~`@4439273. Interface strings: `SteamClient017`, `SteamUser019`, `SteamFriends015`,
`SteamUserStats…011`, `SteamApps…008`, `SteamUtils009`, + full Steamworks header set.

**Auth (proven):** `osSteamIdentityProvider::AuthenticationRequest` (line 1146893),
`OnRequestEncryptedAppTicketCB` (line 1186767), `SteamAPI_ISteamUser_RequestEncryptedAppTicket`
(`DAT_142cbabd8` @4306061), `OSJWTClaimsDTO`/`DebugPrintOSJWTClaims` (line 1841943), `"Bearer "` (@1828023).

**Async REST client (proven):** `osClientBase::executeJSONAndConvertAsync<FeatLeaderboardDTO>` (line 1139527),
`<vector<FeatLeaderboardDTO>>` (line 1140691); `osHTTPRequestBuilder`/`osHTTPClient`/`osHTTPResponse`/
`osJSONResponse`/`osHTTPMethod`/`osHTTPFilter`/`osHTTPRequestPriority`; futures `osTFuture`/`osPromise`/
`osTaskImpl`/`osTaskException`/`osStatus`.

**Backend DTOs (proven, `osdk::jc4v6::`):** `FeatLeaderboardDTO`, `FeatValueDTO`, `FeatNotificationDTO`,
`EventDTO`, `EventChallengeDTO`, `EventParticipationDTO`, `EventParticipationResponseDTO`
(+`EventParticipationStatus`/`EventParticipationResult`), `InfocastDTO` (+`LocalizedContentType`),
`MembershipResource_MembershipAccountResponseDTO` (+`MembershipResource_MembershipAccountStatus`),
`ProfileValueDTO`, `ItemDTO`, `ItemQuantityDTO`, `OSJWTClaimsDTO`; codegen client `osdk::jc4v6::Client`.

**REST roots (proven):** `/feats`, `/events`, `/participations`, `/infocasts`, `/membership`,
`/notifications`, `/profiles`, `/profile/{item,offer,task,taskgroup,upgrade}.onlinec`.

**Telemetry (proven):** `FUN_1409ff030` @0x1409ff030 (size=5300, caller `0x14949d990`); pipeline
`osMetricQueue` + `osMetricFilterByName`; typed-prefix key set (see §6).

**DLC / entitlement (proven):** `CConditional_IsDLCUnlocked` registered via `FUN_140f27f60(…,0x1a)`
(`DAT_142cb94bc` @1419742, `DAT_142cb5ae8` @605915); `ShowDLCStoreDialogAsync` `FUN_1409b8480` @0x1409b8480 →
`FUN_1409a58e0` (callers `FUN_140dcbe50`, `FUN_140dd2310`, `FUN_14a633e60`).

**Manager registry (proven):** manager-list site enclosing symbol `FUN_146cd0bea` @0x146cd0bea (obfuscated
head; caller `FUN_149b2f323`); app-system-factory registrars `FUN_148f958e0` @0x148f958e0 / `FUN_148f960c0`
@0x148f960c0. Online factory ptrs `PTR_LAB_141d909c8`…`PTR_LAB_141d90ee8` (table above).

**Achievements (proven):** 41 `ach_*` ids (full list in §7); `CAchievementsManager` `PTR_LAB_141d90ac8`.

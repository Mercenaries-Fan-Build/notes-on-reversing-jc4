# Live-read value log (x64dbg)

Values read from the **live, paused** JustCause4.exe (Denuvo anti-debug bypassed via ScyllaHide PEB mask —
see `[[debugging-jc4-denuvo-setup]]`). **Module base `0x140000000`, no ASLR relocation** → every `FUN_`/`DAT_`
address in the decomp and the 34 system docs is a directly-readable live VA. Offline fallback source for the
same reads: `C:\Users\Shadow\AppData\Local\Temp\JustCause4.DMP`.

**Grading:** a value here is **proven** (it is the actual byte content of the loaded image). The *role* of each
address is only as strong as the doc that cited it — mostly **inferred** from surrounding decomp. Read date:
2026-08-16.

## Headline finding — where the tunables actually live

Sweeping the `.rdata` `DAT_` globals the 34 docs cited returned mostly **shared engine math constants**
(`π`, `deg2rad`, `1/2π`, `FLT_MAX`, epsilons) and **sentinels** — *not* per-system gameplay knobs. This
**confirms the docs' repeated prediction**: the executable holds the *mechanism*, but the gameplay
*magnitudes* (grapple/tether forces, tornado pull, weapon damage, spawn budgets) are **not** exe globals —
they live in the **RTPC/ADF component data**, deserialized into live object instances at runtime. So the
remaining "walled magnitude" gaps are unlocked not by reading globals but by **reading live component
instances** (or decoding the RTPC/ADF data with `jc4_adf`). The global sweep below is complete; the
value-bearing frontier is live-instance reads.

## 1. Genuine tunables (proven values)

Roles are as the citing doc inferred them; the value is proven-live.

| Address | Value | Cited in / near | Role (grade) |
|---|---|---|---|
| `DAT_141ca6f44` | `0.5` | destruction (`FUN_140ab9cf0`) | CForcePulse apply global scalar (proven value; role inferred — also = shared const 0.5) |
| `DAT_141ca9c7c` | `100.0` | physics/force | force/limit constant (value proven; role inferred) |
| `DAT_141cae1ac` | `5.0` | destruction/force | tunable (value proven; role inferred) |
| `DAT_141cae1bc` | `20.0` | missions_progression (supply settle) | supply-drop settle threshold (value proven; role inferred) |
| `DAT_141cae1c0` | `50.0` | destruction (`FUN_140abd680`) | CForcePulse radius-growth cap (value proven; role inferred) |
| `DAT_141cca7dc` | `2500.0` | — | tunable (value proven; role unattributed) |
| `DAT_141ce55d0` | `30.0` | — | tunable (value proven; role unattributed) |
| `DAT_141cfb42c` | `0.005554` | — | small rate/step (value proven; role unattributed) |
| `DAT_141d083fc` | `900.0` | — | tunable (value proven; role unattributed) |
| `DAT_141d3b230` | `26.0` | — | tunable (value proven; role unattributed) |
| `DAT_141ebfa94` | `12.31` | — | tunable (value proven; role unattributed) |
| `DAT_141d37f20` / `DAT_141d37f40` | `190.0` / `-190.0` | — | symmetric ± clamp (angle/pos range?) heading a param table (value proven) |
| `DAT_141d143d4` | `0.0` | — | first field of a config/curve table (`{0.33, 0.66, 0.94, 1.0, 2.3, 19, 76.5, 22500, 25600, 90000, 160000, 1.5e6, -30, -50, …}` then int ids 1008–1011) |

## 2. Shared math-constant pool & sentinels (NOT gameplay tunables)

The `0x141ca6c9x` cluster is the engine's global math-constant table. Several docs cited these as if
system-specific — they are not; they are shared constants. **Correction flag for the docs.**

| Address | Value | Identity |
|---|---|---|
| `DAT_141ca6c98` | `0.0174533` | `deg2rad` (π/180) |
| `DAT_141ca6ca0` | `0.159155` | `1/(2π)` |
| `DAT_141ca6cac` | `1.0` | unit (the "explosion strength scalar" is identity) |
| `DAT_141ca6cb4` | `1.5708` | `π/2` |
| `DAT_141ca6cb8` | `3.14159` | `π` |
| `DAT_141ca6cc4` | `10.0` | ten |
| `DAT_141cb8224` | `57.2958` | `rad2deg` (180/π) |
| `DAT_141ca70ac` | `0.01` | hundredth |
| `DAT_141ca70d4` / `70d8` | `2.0` / `3.0` | small ints |
| `DAT_141ca9c6c` | `0.001` | milli |
| `DAT_141cae160` | `0.02` | — |
| `DAT_141ca7170` | `1e-4` | epsilon |
| `DAT_141ca72c4` | `1e-5` | epsilon |
| `DAT_141ca71f0` | `-0.5` | — |
| `DAT_141ca71fc` | `3.40282e38` | `FLT_MAX` |
| `DAT_141ecb27c` | `+inf` | infinity sentinel |
| `DAT_141ca6dd0` | `0xffffffff` | all-ones mask / NaN |
| `DAT_141ca7710`, `DAT_141eef470`, `DAT_141f07d08` | `0.0` | zero |

## 3. Cited `DAT_`s that are not numeric (strings / hashes / pointers)

So future reads don't mis-treat these as tunables:

| Address | Content | Kind |
|---|---|---|
| `DAT_141cf8f48` | `".vb"` / `".fb"` | string (file-ext table) |
| `DAT_141dd5750` | `"fore"` | string fragment |
| `DAT_141d94258` | `"CCar"` | class-name string fragment |
| `DAT_141d362ec` | `45482de4b02c4625d287a939a6465f30` | 16-byte GUID/hash blob |
| `DAT_141c8f550`, `DAT_141c901e8`, `DAT_141d008e8`, `DAT_141d1c474`, `DAT_141ee61b4` | large/negative dwords | hash/mask constants (not floats) |
| `DAT_141d919f0` | `f8cc8778…` | data/pointer (0x141d9… region = vtable/pointer `.data`, not tunables) |

## Method

`x64dbg` MCP `MemoryRead`, read-only while paused (user drives execution; never resumed). Bulk `.rdata` block
reads decoded little-endian as f32/u32 and classified (float tunable / math-const / sentinel / string / hash).
Candidate address list = every `DAT_141……` cited across the 34 docs (56 unique). The `DAT_142c……` range was
excluded from the *tunable* sweep — it is runtime `.data` (lookup3 type-id tokens and singleton holders),
covered by `[[reflection-registry-live-crack]]`.

## Next (value-bearing) step

Gameplay magnitudes are in component instances, not globals → **live-instance reads**: from a paused runtime
positioned near a spawned object (a tornado, a tethered vehicle, an equipped weapon), get the instance pointer
(register/singleton walk), read its component fields, and cross-check against the `jc4_adf` decode of the same
component's ADF/RTPC type. That turns the "walled magnitude" tags across the docs into proven values.

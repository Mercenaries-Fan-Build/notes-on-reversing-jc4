//! Just Cause 4 (Avalanche "Apex" engine) format codecs — the shared library behind the jc4_* CLIs
//! and jc4_workshop. Every format here was cracked from the game binary and community-audited
//! (2026-08-15); specs live in ../../docs/formats/.
//!
//! - [`hash`]  — lookup3 `hashlittle`, the 32-bit engine name/path hash.
//! - [`oodle`] — runtime binding to the game's Oodle DLL (codec 4).
//! - [`tab`]   — TAB/ARC v2 archive: parse the ToC, decode/extract payloads (raw/zlib/Oodle).
//! - [`adf`]   — ADF reflection-based typed container: decode instances to a typed (serde_json) tree.

pub mod hash;
pub mod oodle;
pub mod tab;
pub mod adf;
pub mod avtx;
pub mod bundle;
pub mod sarc;
pub mod amf;

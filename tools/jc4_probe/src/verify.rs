//! Collision filter for cracked names. A lookup3 match against 120k targets over ~1e8 candidates yields
//! birthday collisions: a generated string hashing to a REAL entry whose true name differs. Since we only
//! generate plausible strings, false positives look real — so we need a CONTENT oracle. Each cracked path
//! claims an extension (→ expected file magic); decode the entry it hashes to and confirm the magic class
//! matches. Mismatch = proven-false (collision). No known magic for that ext = unverifiable (kept, flagged).
use std::collections::{HashMap, HashSet};
use std::fs::File;

use jc4_formats::hash::hashlittle;
use jc4_formats::oodle::Oodle;
use jc4_formats::tab::{decode_entry, magic_ext, oodle_dll_from_env, parse_tab, Entry};

/// Expected magic (from `magic_ext`) for a claimed extension. `None` ⇒ no strong magic to check.
fn expected_magic(ext: &str) -> Option<&'static [&'static str]> {
    match ext {
        "modelc" | "meshc" | "hrmeshc" | "mdic" | "navmeshc" | "environc" | "aisystunec"
        | "blo_adf" | "epe_adf" | "rawc" | "onlinec" | "obc" => Some(&["adf"]),
        "ddsc" => Some(&["avtx", "dds"]),
        "ee" => Some(&["sarc"]),
        "epe" => Some(&["rtpc"]),
        _ => None, // bl/fl/nl/blo/ban/pfx_*/gfx… — no cheap magic check
    }
}

pub fn run(game_dir: &str, cracked_path: &str) {
    let cracked = crate::read_lines(cracked_path);
    // hash -> claimed path (first wins; collisions inside our own set are themselves suspicious)
    let mut want: HashMap<u32, String> = HashMap::new();
    for p in &cracked { want.entry(hashlittle(p.as_bytes(), 0)).or_insert_with(|| p.clone()); }

    // locate each wanted hash's entry across the archives
    let root = std::path::Path::new(game_dir).join("archives_win64");
    let mut located: HashMap<u32, (std::path::PathBuf, Entry)> = HashMap::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() { stack.push(p); continue; }
                if p.extension().map_or(true, |x| x != "tab") { continue; }
                if let Ok(b) = std::fs::read(&p) {
                    if let Ok(t) = parse_tab(&b) {
                        let arc = p.with_extension("arc");
                        for en in &t.entries {
                            if want.contains_key(&en.name_hash) { located.entry(en.name_hash).or_insert_with(|| (arc.clone(), *en)); }
                        }
                    }
                }
            }
        }
    }

    let dll = oodle_dll_from_env();
    let (mut confirmed, mut mismatch, mut unverifiable, mut undecodable) = (0u32, 0u32, 0u32, 0u32);
    let mut examples: Vec<String> = Vec::new();
    let mut magic_ok: HashSet<u32> = HashSet::new();   // magic matched the claimed ext
    let mut proven_false: HashSet<u32> = HashSet::new(); // magic contradicted the ext
    // group by arc so we open each once, decoding only the wanted entries
    let mut by_arc: HashMap<std::path::PathBuf, Vec<(u32, Entry)>> = HashMap::new();
    for (h, (arc, en)) in &located { by_arc.entry(arc.clone()).or_default().push((*h, *en)); }
    for (arc, ents) in &by_arc {
        let tabp = arc.with_extension("tab");
        let tb = match std::fs::read(&tabp) { Ok(b) => b, Err(_) => continue };
        let t = match parse_tab(&tb) { Ok(t) => t, Err(_) => continue };
        let mut f = match File::open(arc) { Ok(f) => f, Err(_) => continue };
        let mut oodle: Option<Oodle> = None;
        for (h, en) in ents {
            let path = &want[h];
            let ext = path.rsplit('.').next().unwrap_or("");
            let exp = match expected_magic(ext) { Some(e) => e, None => { unverifiable += 1; continue; } };
            match decode_entry(&mut f, &t, en, &dll, &mut oodle) {
                Ok(data) => {
                    let m = magic_ext(&data);
                    if exp.contains(&m) { confirmed += 1; magic_ok.insert(*h); }
                    else { mismatch += 1; proven_false.insert(*h); if examples.len() < 8 { examples.push(format!("  {m:>5} != .{ext}  {path}")); } }
                }
                Err(_) => undecodable += 1,
            }
        }
    }

    // sibling co-confirmation: count NON-false cracked extensions per stem (dir+base). ≥2 ⇒ a real family
    // (two independent 32-bit matches on one stem ≈ 10^-9). Covers exts with no magic (bl/fl/nl/…).
    let mut fam: HashMap<String, u32> = HashMap::new();
    for p in &cracked {
        if proven_false.contains(&hashlittle(p.as_bytes(), 0)) { continue; }
        let file = p.rsplit('/').next().unwrap_or(p);
        let base = file.split('.').next().unwrap_or(file);
        let stem = format!("{}{base}", &p[..p.len() - file.len()]);
        *fam.entry(stem).or_default() += 1;
    }

    // confident set: magic-confirmed OR in a ≥2 sibling family; never proven-false
    let mut confident: Vec<&String> = Vec::new();
    for p in &cracked {
        let h = hashlittle(p.as_bytes(), 0);
        if proven_false.contains(&h) { continue; }
        let file = p.rsplit('/').next().unwrap_or(p);
        let base = file.split('.').next().unwrap_or(file);
        let stem = format!("{}{base}", &p[..p.len() - file.len()]);
        if magic_ok.contains(&h) || fam.get(&stem).copied().unwrap_or(0) >= 2 { confident.push(p); }
    }
    confident.sort();
    let out = format!("{}.confident.filelist", cracked_path.trim_end_matches(".filelist"));
    let mut s = String::new();
    for p in &confident { s.push_str(p); s.push('\n'); }
    let _ = std::fs::write(&out, &s);

    let checkable = confirmed + mismatch;
    println!("cracked paths: {}", cracked.len());
    println!("  located in archives: {}", located.len());
    println!("  magic CONFIRMED: {confirmed}   magic PROVEN-FALSE: {mismatch}   (no-magic ext: {unverifiable}, undecodable: {undecodable})");
    if checkable > 0 {
        println!("  raw false-positive rate on checkable set: {:.1}%  ({mismatch}/{checkable})",
            100.0 * mismatch as f64 / checkable as f64);
    }
    println!("  CONFIDENT set (magic-ok OR ≥2-sibling family, minus proven-false): {}  -> {out}", confident.len());
    if !examples.is_empty() { println!("  example collisions:\n{}", examples.join("\n")); }
}

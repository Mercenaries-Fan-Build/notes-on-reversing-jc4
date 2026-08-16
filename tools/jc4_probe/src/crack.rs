//! Grammar-guided name recovery — NOT blind brute force. Humans named JC4's assets with a bounded,
//! friendly vocabulary and recurring templates (mined from known paths: ~21 roots, a fixed extension
//! alphabet, `token_token_NNN` basenames, sibling families). This engine reproduces those CONVENTIONS:
//! generate plausible paths from patterns/vocabulary seen in already-known names, hash each with lookup3,
//! keep the ones matching a real archive entry hash. Every hit feeds back as a seed, so coverage
//! compounds. Hot loop writes into a reused buffer and only allocates on a hit; fans out via rayon.
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use jc4_formats::hash::hashlittle;
use jc4_formats::tab::parse_tab;
use rayon::prelude::*;

/// JC4 cooked-extension alphabet (mined) — a stem usually ships as several of these.
const EXTS: &[&str] = &[
    "modelc", "meshc", "hrmeshc", "mdic", "ddsc", "hmddsc", "resourcebundle", "bl", "blo", "blo_adf",
    "fl", "nl", "ban", "navmeshc", "obc", "ee", "epe", "epe_adf", "environc", "gfx", "bin",
    "pfx_breakablecompoundc", "pfx_staticcompoundc", "pfx_charactercompoundc",
];
/// Convention suffix families: LOD levels, collision/state, texture channels — deterministic "creative"
/// transforms: one known name implies its whole variant set.
const SUFFIX_SETS: &[&[&str]] = &[
    &["_lod0", "_lod1", "_lod2", "_lod3", "_lod4", "_lod5"],
    &["_col", "_dst", "_destroyed", "_damaged", "_clean", "_broken", "_open", "_closed"],
    &["_dif", "_diff", "_albedo", "_nrm", "_normal", "_spec", "_gloss", "_mask", "_ao", "_emissive", "_atlas"],
];
const VOCAB_CAP: usize = 240; // top-N tokens per context for the substitution combinator (CPU bound)

fn discover_tabs(game_dir: &str) -> Vec<PathBuf> {
    let root = Path::new(game_dir).join("archives_win64");
    let (mut out, mut stack) = (Vec::new(), vec![root]);
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() { stack.push(p); }
                else if p.extension().map_or(false, |x| x == "tab") { out.push(p); }
            }
        }
    }
    out
}

/// (dir-with-trailing-slash, base = basename before FIRST '.').
fn split_path(p: &str) -> (&str, &str) {
    let file = p.rsplit('/').next().unwrap_or(p);
    let base = file.split('.').next().unwrap_or(file);
    (&p[..p.len() - file.len()], base)
}

/// Thematic context for the vocabulary: first two path segments (`models/environments`,
/// `locations/solis`) — a large in-theme token pool, narrow enough that swaps stay on-theme.
fn context(p: &str) -> &str {
    let b = p.as_bytes();
    let mut slashes = 0;
    for (i, &c) in b.iter().enumerate() {
        if c == b'/' { slashes += 1; if slashes == 2 { return &p[..i]; } }
    }
    p.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

pub fn run(game_dir: &str, dict_path: &str, out_path: &str, thesaurus: Option<&str>, rounds: usize) {
    // 1. target: every real entry hash across all archives
    let mut target: HashSet<u32> = HashSet::new();
    for tab in discover_tabs(game_dir) {
        if let Ok(b) = std::fs::read(&tab) {
            if let Ok(t) = parse_tab(&b) { for e in &t.entries { target.insert(e.name_hash); } }
        }
    }
    eprintln!("target: {} unique entry hashes", target.len());

    // 2. seed = known paths that resolve a target hash
    let known = crate::read_lines(dict_path);
    let syn = crate::load_thesaurus(thesaurus);
    if thesaurus.is_some() { eprintln!("thesaurus: {} head words", syn.len()); }
    let mut resolved: HashSet<u32> = HashSet::new();
    let mut seed: Vec<String> = Vec::new();
    for p in &known {
        let h = hashlittle(p.as_bytes(), 0);
        if target.contains(&h) && resolved.insert(h) { seed.push(p.clone()); }
    }
    let start = resolved.len();
    eprintln!("seed: {start} known paths resolve a target ({:.1}%)", 100.0 * start as f64 / target.len().max(1) as f64);

    // 3. theme vocabulary: top-N basename tokens per context, + extensions used there (from full dict)
    let mut freq: HashMap<&str, HashMap<&str, u32>> = HashMap::new();
    let mut ctx_exts: HashMap<&str, BTreeSet<&str>> = HashMap::new();
    for p in &known {
        let (_, base) = split_path(p);
        let c = context(p);
        let m = freq.entry(c).or_default();
        for t in base.split(['_', '-']) { if t.len() >= 3 && !t.bytes().all(|b| b.is_ascii_digit()) { *m.entry(t).or_default() += 1; } }
        let file = p.rsplit('/').next().unwrap_or(p);
        if let Some((_, ext)) = file.split_once('.') { ctx_exts.entry(c).or_default().insert(ext); }
    }
    let vocab: HashMap<String, Vec<String>> = freq.iter().map(|(c, m)| {
        let mut v: Vec<(&str, u32)> = m.iter().map(|(k, n)| (*k, *n)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        (c.to_string(), v.into_iter().take(VOCAB_CAP).map(|(k, _)| k.to_string()).collect())
    }).collect();
    let ctx_exts: HashMap<String, Vec<String>> = ctx_exts.iter().map(|(c, s)| (c.to_string(), s.iter().map(|x| x.to_string()).collect())).collect();
    eprintln!("vocabulary: {} contexts, {} tokens (cap {VOCAB_CAP}/ctx)", vocab.len(), vocab.values().map(|v| v.len()).sum::<usize>());

    let mut cracked: BTreeSet<String> = BTreeSet::new();
    let mut frontier = seed;

    // 4. feedback loop. Gen A/B/D run every round on the frontier; the heavy Gen C combinator runs only
    //    on the ORIGINAL seed (round 1) so it doesn't explode over its own output.
    for round in 1..=rounds {
        let before = resolved.len();
        let use_c = round == 1;
        let hits: Vec<(u32, String)> = frontier.par_iter().flat_map_iter(|p| {
            let (dir, base) = split_path(p);
            let mut out: Vec<(u32, String)> = Vec::new();
            let mut buf = String::with_capacity(192);
            macro_rules! chk { () => {{ let h = hashlittle(buf.as_bytes(), 0); if target.contains(&h) { out.push((h, buf.clone())); } }}; }

            // Gen A — sibling extensions
            for e in EXTS { buf.clear(); let _ = write!(buf, "{dir}{base}.{e}"); chk!(); }
            // Gen D — convention suffixes (strip a known one, append each variant)
            for fam in SUFFIX_SETS {
                let stem = fam.iter().find(|s| base.ends_with(**s)).map(|s| &base[..base.len() - s.len()]).unwrap_or(base);
                for s in *fam {
                    for e in EXTS { buf.clear(); let _ = write!(buf, "{dir}{stem}{s}.{e}"); chk!(); }
                }
            }
            // Gen B — numeric templates: vary each digit run over a bounded range (one at a time)
            let bb = base.as_bytes();
            let mut i = 0;
            while i < bb.len() {
                if !bb[i].is_ascii_digit() { i += 1; continue; }
                let s = i;
                while i < bb.len() && bb[i].is_ascii_digit() { i += 1; }
                let (pre, post, w) = (&base[..s], &base[i..], i - s);
                if w > 3 { continue; }
                let hi = if w <= 2 { 99 } else { 255 };
                for v in 0..=hi {
                    for e in EXTS {
                        buf.clear();
                        if w <= 2 { let _ = write!(buf, "{dir}{pre}{v:0w$}{post}.{e}"); }
                        else { let _ = write!(buf, "{dir}{pre}{v}{post}.{e}"); }
                        chk!();
                    }
                }
            }
            // Gen C — theme-vocabulary substitution (+ thesaurus). Only round 1; bounded exts per context.
            if use_c {
                let c = context(p);
                if let Some(vv) = vocab.get(c) {
                    let exs = ctx_exts.get(c).map(|v| v.as_slice()).unwrap_or(&[]);
                    for tok in base.split(['_', '-']).filter(|t| t.len() >= 3) {
                        for r in vv.iter().map(|s| s.as_str()).chain(syn.get(tok).into_iter().flatten().map(|s| s.as_str())) {
                            if r == tok { continue; }
                            for e in exs { buf.clear(); let _ = write!(buf, "{dir}{}.{e}", base.replacen(tok, r, 1)); chk!(); }
                        }
                    }
                }
            }
            out.into_iter()
        }).collect();

        let mut newfront: Vec<String> = Vec::new();
        for (h, c) in hits {
            if resolved.insert(h) { newfront.push(c.clone()); cracked.insert(c); }
        }
        let gained = resolved.len() - before;
        eprintln!("round {round}{}: +{gained} (resolved {}/{}, {:.1}%)",
            if use_c { " [A/B/C/D]" } else { " [A/B/D]" }, resolved.len(), target.len(),
            100.0 * resolved.len() as f64 / target.len().max(1) as f64);
        if gained == 0 { break; }
        frontier = newfront;
    }

    let mut s = String::with_capacity(cracked.len() * 48);
    for p in &cracked { s.push_str(p); s.push('\n'); }
    let _ = std::fs::write(out_path, &s);
    println!("cracked {} NEW paths (resolved {start} -> {}, +{:.1} pts); wrote {out_path}",
        cracked.len(), resolved.len(), 100.0 * (resolved.len() - start) as f64 / target.len().max(1) as f64);
}

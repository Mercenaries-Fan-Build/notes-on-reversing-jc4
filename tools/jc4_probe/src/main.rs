// jc4_probe — research sandbox for the MESSY, fast-changing experiments (name-cracking, heuristic
// probes). Deliberately separate from the clean, possibly-public jc4_formats/jc4_arc crates: nothing
// here is API-stable and the logic churns. Add subcommands freely.
use std::collections::HashMap;

mod crack;
mod gpu;
mod verify;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 2 {
        eprintln!("jc4_probe — JC4 research sandbox");
        eprintln!("  jc4_probe crack  <game_dir> <dict.filelist> <out.filelist> [--thesaurus F] [--rounds N]");
        eprintln!("        recover names via grammar/vocabulary generation + lookup3 match (feedback loop)");
        eprintln!("  jc4_probe verify <game_dir> <cracked.filelist>");
        eprintln!("        content oracle: decode each cracked entry, confirm magic matches ext (collision filter)");
        eprintln!("  jc4_probe gpu    <game_dir> <dict.filelist> <out.filelist> --context CTX");
        eprintln!("        GPU combinator: on-device generate CTX/<tok>_<tok>.<ext>, lookup3, match (no vocab cap)");
        return;
    }
    match a[1].as_str() {
        "crack" => {
            if a.len() < 5 {
                eprintln!("usage: jc4_probe crack <game_dir> <dict.filelist> <out.filelist> [--thesaurus F] [--rounds N]");
                return;
            }
            let flag = |name: &str| a.iter().position(|s| s == name).and_then(|i| a.get(i + 1));
            let thes = flag("--thesaurus").map(|s| s.as_str());
            let rounds = flag("--rounds").and_then(|s| s.parse().ok()).unwrap_or(6);
            crack::run(&a[2], &a[3], &a[4], thes, rounds);
        }
        "verify" => {
            if a.len() < 4 { eprintln!("usage: jc4_probe verify <game_dir> <cracked.filelist>"); return; }
            verify::run(&a[2], &a[3]);
        }
        "gpu" => {
            if a.len() < 6 { eprintln!("usage: jc4_probe gpu <game_dir> <dict.filelist> <out.filelist> --context CTX"); return; }
            let ctx = a.iter().position(|s| s == "--context").and_then(|i| a.get(i + 1));
            match ctx { Some(c) => gpu::run(&a[2], &a[3], &a[4], c), None => eprintln!("--context CTX required (e.g. models/environments)") }
        }
        other => eprintln!("unknown command {other}"),
    }
}

// small shared helper used across probes
pub fn read_lines(path: &str) -> Vec<String> {
    std::fs::read_to_string(path).unwrap_or_default().lines()
        .map(|l| l.trim().to_ascii_lowercase().replace('\\', "/"))
        .filter(|l| !l.is_empty() && !l.starts_with(';') && l.contains('/'))
        .collect()
}

pub fn load_thesaurus(path: Option<&str>) -> HashMap<String, Vec<String>> {
    let mut syn = HashMap::new();
    if let Some(tp) = path {
        if let Ok(s) = std::fs::read_to_string(tp) {
            for line in s.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    let vs: Vec<String> = v.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
                    if !vs.is_empty() { syn.insert(k.trim().to_ascii_lowercase(), vs); }
                }
            }
        }
    }
    syn
}

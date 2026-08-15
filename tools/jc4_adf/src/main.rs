// jc4_adf — Just Cause 4 ADF typed decoder CLI. Thin wrapper over `jc4_formats::adf`.
// See ../../docs/formats/adf.md.
use std::panic::AssertUnwindSafe;
use jc4_formats::adf::parse;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("jc4_adf — ADF typed decoder");
        eprintln!("  jc4_adf info <adf>          header + type/instance summary");
        eprintln!("  jc4_adf dump <adf>          decode instances -> JSON (stdout)");
        eprintln!("  jc4_adf verify <dir>        parse+decode every *.adf under <dir>, report pass/fail");
        return;
    }
    match a[1].as_str() {
        "info" => {
            let adf = parse(std::fs::read(&a[2]).unwrap()).unwrap_or_else(|e| { eprintln!("{e}"); std::process::exit(1) });
            println!("total_size={} (file={})  types={} instances={} names={} strhash={}",
                adf.total_size(), adf.data_len(), adf.num_types(), adf.num_instances(), adf.num_names(), adf.num_strhash());
            for &(nh, th, off, size, ni) in adf.instances() {
                println!("  instance {} : type 0x{th:08x} @0x{off:x} size {size} (name_hash 0x{nh:08x}, name_idx {ni})",
                    adf.name(ni));
            }
        }
        "dump" => {
            let adf = parse(std::fs::read(&a[2]).unwrap()).unwrap_or_else(|e| { eprintln!("{e}"); std::process::exit(1) });
            println!("{}", serde_json::to_string_pretty(&adf.decode_instances()).unwrap());
        }
        "verify" => {
            let dir = &a[2];
            let (mut ok, mut bad) = (0u32, 0u32);
            let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().map_or(false, |x| x == "adf")).collect();
            files.sort();
            for f in &files {
                let res = std::panic::catch_unwind(AssertUnwindSafe(|| -> Result<(), String> {
                    let adf = parse(std::fs::read(f).unwrap())?;
                    if adf.total_size() as usize != adf.data_len() { return Err("total_size!=len".to_string()); }
                    let _ = adf.decode_instances(); // must not panic / OOB
                    Ok(())
                }));
                match res {
                    Ok(Ok(())) => ok += 1,
                    Ok(Err(e)) => { bad += 1; eprintln!("FAIL {}: {e}", f.display()); }
                    Err(_) => { bad += 1; eprintln!("PANIC {}", f.display()); }
                }
            }
            println!("ADF verify: PASS={ok} FAIL={bad} / {} files", files.len());
        }
        other => eprintln!("unknown command {other}"),
    }
}

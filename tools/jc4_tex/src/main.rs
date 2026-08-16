// jc4_tex — Just Cause 4 AVTX texture CLI. Thin wrapper over `jc4_formats::avtx`.
// See ../../docs/formats/avtx.md.
use std::collections::BTreeMap;
use jc4_formats::avtx;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("jc4_tex — AVTX texture tool");
        eprintln!("  jc4_tex info   <avtx>            header + best-inline-mip summary");
        eprintln!("  jc4_tex dds    <avtx> <out.dds>  export best inline mip as a DDS (DX10)");
        eprintln!("  jc4_tex rgba   <avtx> [out.ppm]  decode best inline mip to RGBA (same path as the workshop view)");
        eprintln!("  jc4_tex verify <dir>             oracle over every *.avtx (size + format census)");
        return;
    }
    match a[1].as_str() {
        "info" => {
            let b = std::fs::read(&a[2]).unwrap();
            let h = avtx::parse_header(&b).unwrap_or_else(|e| { eprintln!("{e}"); std::process::exit(1) });
            println!("AVTX v{}  {}x{}x{}  {} ({:#x})  mips {}/{} inline  flags {:#06x}{}{}",
                h.version, h.width, h.height, h.depth, avtx::dxgi_name(h.dxgi_format), h.dxgi_format,
                h.mip_inline, h.mip_total, h.flags,
                if h.is_cubemap() { "  [cubemap]" } else { "" },
                if h.has_external_mips() { "  [has external hi-res mips]" } else { "" });
            match avtx::best_inline_mip(&b) {
                Ok(m) => println!("best inline mip: {}x{}  {} bytes", m.width, m.height, m.data.len()),
                Err(e) => println!("best inline mip: {e}"),
            }
        }
        "dds" => {
            if a.len() < 4 { eprintln!("usage: jc4_tex dds <avtx> <out.dds>"); return; }
            let b = std::fs::read(&a[2]).unwrap();
            match avtx::best_inline_mip(&b) {
                Ok(m) => {
                    let dds = avtx::to_dds(&m);
                    std::fs::write(&a[3], &dds).unwrap();
                    println!("wrote {} ({}x{} {}, {} bytes)", a[3], m.width, m.height, avtx::dxgi_name(m.dxgi_format), dds.len());
                }
                Err(e) => { eprintln!("{e}"); std::process::exit(1) }
            }
        }
        "rgba" => {
            let b = std::fs::read(&a[2]).unwrap();
            let m = avtx::best_inline_mip(&b).unwrap_or_else(|e| { eprintln!("{e}"); std::process::exit(1) });
            let rgba = avtx::decode_rgba(&m).unwrap_or_else(|e| { eprintln!("{e}"); std::process::exit(1) });
            let nonzero = rgba.chunks_exact(4).filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0).count();
            println!("decoded {}x{} {} -> {} RGBA bytes; {}/{} texels non-black",
                m.width, m.height, avtx::dxgi_name(m.dxgi_format), rgba.len(), nonzero, (m.width * m.height));
            if let Some(out) = a.get(3) {
                // PPM P6 (opaque RGB) — quick eyeball in any image viewer
                let mut ppm = format!("P6\n{} {}\n255\n", m.width, m.height).into_bytes();
                for px in rgba.chunks_exact(4) { ppm.extend_from_slice(&px[..3]); }
                std::fs::write(out, &ppm).unwrap();
                println!("wrote {out}");
            }
        }
        "png" => {
            if a.len() < 4 { eprintln!("usage: jc4_tex png <ddsc> <out.png> [hmddsc]"); return; }
            let ddsc = std::fs::read(&a[2]).unwrap();
            let hm = a.get(4).map(|p| std::fs::read(p).unwrap());
            match avtx::decode_rgba_from(&ddsc, hm.as_deref()) {
                Ok((w, h, rgba)) => {
                    std::fs::write(&a[3], jc4_formats::amf::to_png_rgba(w as usize, h as usize, &rgba)).unwrap();
                    let nonblack = rgba.chunks_exact(4).filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0).count();
                    println!("{w}x{h} {} -> {}  ({}/{} non-black)", if hm.is_some() { "hi-res" } else { "inline" }, a[3], nonblack, w * h);
                }
                Err(e) => eprintln!("{e}"),
            }
        }
        "verify" => {
            let (mut ok, mut bad) = (0u32, 0u32);
            let mut census: BTreeMap<u32, u32> = BTreeMap::new();
            let (mut cubemaps, mut external) = (0u32, 0u32);
            let mut files: Vec<_> = std::fs::read_dir(&a[2]).unwrap().filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().map_or(false, |x| x == "avtx")).collect();
            files.sort();
            for f in &files {
                let b = match std::fs::read(f) { Ok(b) => b, Err(_) => { bad += 1; continue } };
                let h = match avtx::parse_header(&b) { Ok(h) => h, Err(e) => { bad += 1; eprintln!("FAIL {}: {e}", f.display()); continue } };
                *census.entry(h.dxgi_format).or_default() += 1;
                if h.is_cubemap() { cubemaps += 1; }
                if h.has_external_mips() { external += 1; }
                let body = b.len() - avtx::HEADER_LEN;
                match avtx::expected_inline_size(&h) {
                    Some(sz) if sz == body && h.size_body as usize == body && avtx::best_inline_mip(&b).is_ok() => ok += 1,
                    Some(sz) => { bad += 1; eprintln!("FAIL {}: inline {sz} != body {body} (size_body={})", f.display(), h.size_body); }
                    None => { bad += 1; eprintln!("FAIL {}: unknown dxgi {:#x}", f.display(), h.dxgi_format); }
                }
            }
            println!("AVTX verify: PASS={ok} FAIL={bad} / {} files  (cubemaps {cubemaps}, external-mips {external})", files.len());
            println!("format census:");
            let mut v: Vec<_> = census.into_iter().collect();
            v.sort_by(|x, y| y.1.cmp(&x.1));
            for (fmt, n) in v { println!("  {:>5}  {:<20} {}", fmt, avtx::dxgi_name(fmt), n); }
        }
        other => eprintln!("unknown command {other}"),
    }
}

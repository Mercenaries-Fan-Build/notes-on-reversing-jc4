//! AMF geometry decode — turn a `.meshc`/`.hrmeshc` (Avalanche Model Format, ADF-wrapped) into raw
//! positions + indices for rendering. The mesh header's `StreamAttributes[]` describe the interleaved
//! vertex layout: for each attribute, `Usage` (Position/Normal/TexCoord/…), `Format`, `StreamIndex`
//! (which vertex buffer/stride), `StreamOffset` (within the vertex), and `PackingData` (dequant). We
//! walk the Position stream at (VertexStreamOffsets[si] + i*stride + StreamOffset), dequantize per format
//! (SNORM positions rescale by the PackingData float), and read u16/u32 indices. Mirrors DECA
//! `ff_adf_amf_gltf`. Positions are validated against the header BoundingBox (see decode_mesh). See
//! [[model-amf]] and ../../docs/formats/adf.md (the container).
use serde_json::Value;
use crate::adf;

pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
}

fn bytes_of(v: Option<&Value>) -> Vec<u8> {
    v.and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|n| n.as_u64().map(|u| u as u8)).collect())
        .unwrap_or_default()
}
fn u64_at(v: &Value, k: &str) -> u64 { v.get(k).and_then(|x| x.as_u64()).unwrap_or(0) }
fn arr3(v: Option<&Value>) -> [f32; 3] {
    let a = v.and_then(|x| x.as_array());
    let g = |i: usize| a.and_then(|a| a.get(i)).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
    [g(0), g(1), g(2)]
}
fn u32s(v: &Value, k: &str) -> Vec<u64> {
    v.get(k).and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|n| n.as_u64()).collect()).unwrap_or_default()
}

fn i16le(d: &[u8], o: usize) -> i16 { i16::from_le_bytes([d[o], d[o + 1]]) }
fn f32le(d: &[u8], o: usize) -> f32 { f32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]) }

/// Vertex/index buffer byte arrays from an AmfMeshBuffers instance (`buffers` in a meshc or hrmeshc).
fn buffers_of(v: &Value) -> Result<(Vec<Vec<u8>>, Vec<Vec<u8>>), String> {
    let b = v.get("buffers").ok_or("no buffers instance")?;
    let vb = b.get("VertexBuffers").and_then(|x| x.as_array()).ok_or("no VertexBuffers")?
        .iter().map(|b| bytes_of(b.get("Data"))).collect();
    let ib = b.get("IndexBuffers").and_then(|x| x.as_array()).ok_or("no IndexBuffers")?
        .iter().map(|b| bytes_of(b.get("Data"))).collect();
    Ok((vb, ib))
}

/// Decode the highest-detail LOD present inline in a meshc (low LOD only).
pub fn decode_mesh(meshc: &[u8]) -> Result<Mesh, String> {
    let v = adf::parse(meshc.to_vec())?.decode_instances();
    let header = v.get("header").ok_or("meshc: no header instance")?.clone();
    let (vb, ib) = buffers_of(&v)?;
    decode_from(&header, &vb, &ib)
}

/// Decode the HIGH LOD: the meshc supplies the header (StreamAttributes/LodGroups); the hrmeshc supplies
/// the high-detail buffers. LodGroup buffer indices address the combined list `meshc ++ hrmeshc`.
pub fn decode_mesh_hr(meshc: &[u8], hrmeshc: &[u8]) -> Result<Mesh, String> {
    let mv = adf::parse(meshc.to_vec())?.decode_instances();
    let header = mv.get("header").ok_or("meshc: no header instance")?.clone();
    let (mut vb, mut ib) = buffers_of(&mv)?;
    let hv = adf::parse(hrmeshc.to_vec())?.decode_instances();
    let (hvb, hib) = buffers_of(&hv)?;
    vb.extend(hvb);
    ib.extend(hib);
    decode_from(&header, &vb, &ib)
}

fn decode_from(header: &Value, vb: &[Vec<u8>], ib: &[Vec<u8>]) -> Result<Mesh, String> {
    let groups = header.get("LodGroups").and_then(|x| x.as_array()).ok_or("no LodGroups")?;
    let empty = Vec::new();
    let meshes_of = |g: &Value| g.get("Meshes").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    // A meshc inlines only the low-detail LOD's buffers (higher LODs reference the .hrmeshc). Pick the
    // most-detailed LOD group whose referenced buffers are all present in THIS file.
    let inline = |g: &Value| {
        let ms = meshes_of(g);
        !ms.is_empty() && ms.iter().all(|m| {
            (u64_at(m, "IndexBufferIndex") as usize) < ib.len()
                && u32s(m, "VertexBufferIndices").iter().all(|&x| (x as usize) < vb.len())
        })
    };
    let group = groups.iter().filter(|g| inline(g))
        .max_by_key(|g| meshes_of(g).iter().map(|m| u64_at(m, "VertexCount")).sum::<u64>())
        .ok_or("no inline-decodable LOD group (buffers may be in the .hrmeshc)")?;

    let (mut positions, mut indices) = (Vec::new(), Vec::new());
    for mesh in group.get("Meshes").and_then(|x| x.as_array()).unwrap_or(&empty) {
        let base = positions.len() as u32;
        let (p, idx) = decode_one_mesh(mesh, vb, ib)?;
        positions.extend(p);
        indices.extend(idx.into_iter().map(|i| i + base));
    }
    if positions.is_empty() { return Err("no vertices decoded".into()); }

    let bb = header.get("BoundingBox");
    let (bmin, bmax) = (arr3(bb.and_then(|b| b.get("Min"))), arr3(bb.and_then(|b| b.get("Max"))));
    // ORACLE: every decoded position must lie within the header BoundingBox (small epsilon)
    let eps = 1e-3 + 0.02 * (0..3).map(|i| (bmax[i] - bmin[i]).abs()).fold(0.0f32, f32::max);
    let inside = positions.iter().all(|p| (0..3).all(|i| p[i] >= bmin[i] - eps && p[i] <= bmax[i] + eps));
    if !inside { return Err("decoded positions fall outside the mesh BoundingBox (bad dequant/layout)".into()); }
    Ok(Mesh { positions, indices, bbox_min: bmin, bbox_max: bmax })
}

fn decode_one_mesh(mesh: &Value, vb: &[Vec<u8>], ib: &[Vec<u8>]) -> Result<(Vec<[f32; 3]>, Vec<u32>), String> {
    let vcount = u64_at(mesh, "VertexCount") as usize;
    let attrs = mesh.get("StreamAttributes").and_then(|x| x.as_array()).ok_or("no StreamAttributes")?;
    let pos = attrs.iter().find(|a| a.get("Usage").and_then(|u| u.as_str()) == Some("AmfUsage_Position"))
        .ok_or("no Position attribute")?;

    let si = u64_at(pos, "StreamIndex") as usize;
    let vbi = *u32s(mesh, "VertexBufferIndices").get(si).ok_or("StreamIndex oob")? as usize;
    let voff = *u32s(mesh, "VertexStreamOffsets").get(si).unwrap_or(&0) as usize;
    let stride = u64_at(pos, "StreamStride") as usize;
    let soff = u64_at(pos, "StreamOffset") as usize;
    let fmt = pos.get("Format").and_then(|f| f.as_str()).unwrap_or("");
    let pack = bytes_of(pos.get("PackingData"));
    let scale = if pack.len() >= 4 { f32le(&pack, 0) } else { 1.0 };
    let data = vb.get(vbi).ok_or("vertex buffer index oob")?;

    let mut positions = Vec::with_capacity(vcount);
    for i in 0..vcount {
        let o = voff + i * stride + soff;
        let p = match fmt {
            "AmfFormat_R16G16B16_SNORM" | "AmfFormat_R16G16B16A16_SNORM" => {
                if o + 6 > data.len() { return Err("position stream overruns vertex buffer".into()); }
                let s = scale / 32767.0;
                [i16le(data, o) as f32 * s, i16le(data, o + 2) as f32 * s, i16le(data, o + 4) as f32 * s]
            }
            "AmfFormat_R32G32B32_FLOAT" => {
                if o + 12 > data.len() { return Err("position stream overruns vertex buffer".into()); }
                [f32le(data, o), f32le(data, o + 4), f32le(data, o + 8)]
            }
            other => return Err(format!("position format {other} not handled yet")),
        };
        positions.push(p);
    }

    let ibi = u64_at(mesh, "IndexBufferIndex") as usize;
    let ibo = u64_at(mesh, "IndexBufferOffset") as usize;
    let istride = u64_at(mesh, "IndexBufferStride").max(2) as usize;
    let icount = u64_at(mesh, "IndexCount") as usize;
    let idata = ib.get(ibi).ok_or("index buffer index oob")?;
    let mut indices = Vec::with_capacity(icount);
    for j in 0..icount {
        let o = ibo + j * istride;
        if o + istride > idata.len() { return Err("index stream overruns index buffer".into()); }
        let idx = if istride == 4 { u32::from_le_bytes([idata[o], idata[o + 1], idata[o + 2], idata[o + 3]]) }
                  else { u16::from_le_bytes([idata[o], idata[o + 1]]) as u32 };
        if idx as usize >= vcount { return Err(format!("index {idx} >= vertex count {vcount}")); }
        indices.push(idx);
    }
    Ok((positions, indices))
}

// ── headless z-buffered rasterizer (render oracle) ───────────────────────────────────────────────
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 { crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB88320 } else { crc >> 1 }; }
    }
    !crc
}
/// Minimal RGB PNG (filter 0, zlib via flate2) so a render can be viewed directly.
fn png(w: usize, h: usize, rgb: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut raw = Vec::with_capacity(h * (1 + w * 3));
    for y in 0..h { raw.push(0); raw.extend_from_slice(&rgb[y * w * 3..(y + 1) * w * 3]); }
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    z.write_all(&raw).unwrap();
    let idat = z.finish().unwrap();
    let mut out = vec![0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    let mut chunk = |typ: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut cd = typ.to_vec(); cd.extend_from_slice(data);
        out.extend_from_slice(&cd);
        out.extend_from_slice(&crc32(&cd).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit, RGB
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &idat);
    chunk(b"IEND", &[]);
    out
}

/// Software-rasterize the mesh (orbit yaw/pitch, orthographic, flat shading, PER-PIXEL z-buffer) to RGBA
/// with a TRANSPARENT background (alpha 0), so it composites over any UI. Exact depth — no painter's-algo
/// ordering error. `base` is the model's RGB tint. This is the workshop viewport's renderer.
pub fn rasterize_rgba(m: &Mesh, w: usize, h: usize, yaw: f32, pitch: f32, base: [u8; 3]) -> Vec<u8> {
    let mut color = vec![0u8; w * h * 4]; // transparent
    let mut zbuf = vec![f32::NEG_INFINITY; w * h];
    let (cy, sy, cp, sp) = (yaw.cos(), yaw.sin(), pitch.cos(), pitch.sin());
    let rot = |v: [f32; 3]| { let (x, z) = (v[0] * cy + v[2] * sy, -v[0] * sy + v[2] * cy); [x, v[1] * cp - z * sp, v[1] * sp + z * cp] };
    let c = [0, 1, 2].map(|i| (m.bbox_min[i] + m.bbox_max[i]) * 0.5);
    let radius = (0..3).map(|i| (m.bbox_max[i] - m.bbox_min[i]).abs()).fold(1e-6f32, f32::max) * 0.5;
    let scale = w.min(h) as f32 / (2.0 * radius) * 0.8;
    let proj: Vec<[f32; 3]> = m.positions.iter().map(|v| {
        let r = rot([v[0] - c[0], v[1] - c[1], v[2] - c[2]]);
        [w as f32 * 0.5 + r[0] * scale, h as f32 * 0.5 - r[1] * scale, r[2]]
    }).collect();
    let light = { let l = [0.35f32, 0.5, 0.79]; let n = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt(); [l[0] / n, l[1] / n, l[2] / n] };
    let edge = |ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32| (bx - ax) * (py - ay) - (by - ay) * (px - ax);
    for t in m.indices.chunks_exact(3) {
        let (pa, pb, pc) = (proj[t[0] as usize], proj[t[1] as usize], proj[t[2] as usize]);
        let n = { let u = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]]; let ww = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
                  [u[1] * ww[2] - u[2] * ww[1], u[2] * ww[0] - u[0] * ww[2], u[0] * ww[1] - u[1] * ww[0]] };
        let nl = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-9);
        let sh = ((n[0] * light[0] + n[1] * light[1] + n[2] * light[2]) / nl).abs() * 0.8 + 0.2;
        let col = [(base[0] as f32 * sh) as u8, (base[1] as f32 * sh) as u8, (base[2] as f32 * sh) as u8, 255];
        let area = edge(pa[0], pa[1], pb[0], pb[1], pc[0], pc[1]);
        if area.abs() < 1e-6 { continue; }
        let (minx, maxx) = (pa[0].min(pb[0]).min(pc[0]).floor().max(0.0) as usize, pa[0].max(pb[0]).max(pc[0]).ceil().min(w as f32 - 1.0) as usize);
        let (miny, maxy) = (pa[1].min(pb[1]).min(pc[1]).floor().max(0.0) as usize, pa[1].max(pb[1]).max(pc[1]).ceil().min(h as f32 - 1.0) as usize);
        for py in miny..=maxy {
            for px in minx..=maxx {
                let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
                let (w0, w1, w2) = (edge(pb[0], pb[1], pc[0], pc[1], fx, fy) / area,
                                    edge(pc[0], pc[1], pa[0], pa[1], fx, fy) / area,
                                    edge(pa[0], pa[1], pb[0], pb[1], fx, fy) / area);
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 { continue; }
                let depth = w0 * pa[2] + w1 * pb[2] + w2 * pc[2];
                let i = py * w + px;
                if depth > zbuf[i] { zbuf[i] = depth; color[i * 4..i * 4 + 4].copy_from_slice(&col); }
            }
        }
    }
    color
}

/// Render oracle: rasterize and composite over a dark ground → RGB PNG (`jc4_arc mesh/model --png`).
pub fn render_png(m: &Mesh, size: usize, yaw: f32, pitch: f32) -> Vec<u8> {
    let (w, h) = (size, size);
    let rgba = rasterize_rgba(m, w, h, yaw, pitch, [255, 122, 51]);
    let bg = [16u8, 22, 28];
    let mut rgb = vec![0u8; w * h * 3];
    for (o, px) in rgba.chunks_exact(4).enumerate() {
        let a = px[3] as u32;
        for c in 0..3 { rgb[o * 3 + c] = ((px[c] as u32 * a + bg[c] as u32 * (255 - a)) / 255) as u8; }
    }
    png(w, h, &rgb)
}

/// Decode and MERGE every mesh part in a model SARC (`.ee`) into one Mesh — each `.meshc` paired with
/// its `.hrmeshc` for high LOD. NOTE: parts are merged in whatever space their own bbox implies; if the
/// result looks jumbled, parts are in per-part local space and need the entity (.epe RTPC) transforms.
pub fn decode_model(sarc_bytes: &[u8]) -> Result<Mesh, String> {
    let members = crate::sarc::parse(sarc_bytes)?;
    let hr: std::collections::HashMap<&str, &[u8]> = members.iter()
        .filter(|m| m.stored && m.name.ends_with(".hrmeshc"))
        .filter_map(|m| m.data(sarc_bytes).map(|d| (m.name.trim_end_matches(".hrmeshc"), d)))
        .collect();
    let mut out = Mesh { positions: Vec::new(), indices: Vec::new(), bbox_min: [f32::MAX; 3], bbox_max: [f32::MIN; 3] };
    let mut parts = 0;
    for m in &members {
        if !(m.stored && m.name.ends_with(".meshc")) { continue; }
        let d = match m.data(sarc_bytes) { Some(d) => d, None => continue };
        let mesh = match hr.get(m.name.trim_end_matches(".meshc")) {
            Some(h) => decode_mesh_hr(d, h), None => decode_mesh(d),
        };
        let mesh = match mesh { Ok(x) => x, Err(_) => continue };
        let base = out.positions.len() as u32;
        out.positions.extend_from_slice(&mesh.positions);
        out.indices.extend(mesh.indices.iter().map(|i| i + base));
        for i in 0..3 { out.bbox_min[i] = out.bbox_min[i].min(mesh.bbox_min[i]); out.bbox_max[i] = out.bbox_max[i].max(mesh.bbox_max[i]); }
        parts += 1;
    }
    if parts == 0 { return Err("no mesh parts in SARC".into()); }
    Ok(out)
}

/// Wavefront OBJ (positions + triangles) for eyeballing the decode in any 3D viewer.
pub fn to_obj(m: &Mesh) -> String {
    let mut s = String::with_capacity(m.positions.len() * 24 + m.indices.len() * 12);
    s.push_str("# jc4 amf mesh\n");
    for p in &m.positions { s.push_str(&format!("v {} {} {}\n", p[0], p[1], p[2])); }
    for t in m.indices.chunks_exact(3) { s.push_str(&format!("f {} {} {}\n", t[0] + 1, t[1] + 1, t[2] + 1)); }
    s
}

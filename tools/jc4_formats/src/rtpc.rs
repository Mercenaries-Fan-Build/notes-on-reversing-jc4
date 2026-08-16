//! RTPC v3 — Avalanche "Runtime Property Container": the entity blueprint (`.epe`). A tree of nodes,
//! each with typed properties and child nodes. We use it for FAITHFUL model assembly: the entity declares
//! exactly which mesh parts make up its render model (nodes of the render class) and each part's `world`
//! transform — so a bundled-but-separate payload (a different class) is correctly excluded, and attached
//! parts land at their real offset. Layout: "RTPC"+u32 ver; root node header @0x08; node header = 12B
//! {name_hash u32, data_off u32, prop_count u16, child_count u16}; at data_off: prop_count × 9B
//! {name_hash u32, data u32, type u8}, then 4-align, then child_count × 12B node headers (recursive).
//! Property `data` is inline for scalars, else a file offset (string/vec/mat/array). See [[composite-assets]].

fn u16(b: &[u8], o: usize) -> u16 { u16::from_le_bytes([b[o], b[o + 1]]) }
fn u32(b: &[u8], o: usize) -> u32 { u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) }
fn f32a(b: &[u8], o: usize) -> f32 { f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]) }

// Well-known property name hashes (lookup3).
pub const P_NAME: u32 = 0xd31ab684;        // "name" (string)
pub const P_CLASS_HASH: u32 = 0xd04059e6;  // "_class_hash" (u32)
pub const P_WORLD: u32 = 0x6ca6d4b9;       // "world" (mat4, row-major, translation in [12..15])
/// Class hash of a render mesh-part node (the parts that make up an entity's visual model).
pub const CLASS_RENDER_PART: u32 = 0xc1d8333a;

#[derive(Clone)]
pub enum Val { U32(u32), F32(f32), Str(String), Mat([f32; 16]), Other(u8, u32) }

pub struct Prop { pub name_hash: u32, pub val: Val }
pub struct Node { pub name_hash: u32, pub props: Vec<Prop>, pub children: Vec<Node> }

impl Node {
    pub fn prop(&self, name_hash: u32) -> Option<&Val> { self.props.iter().find(|p| p.name_hash == name_hash).map(|p| &p.val) }
    pub fn name(&self) -> Option<&str> { match self.prop(P_NAME) { Some(Val::Str(s)) => Some(s), _ => None } }
    pub fn class_hash(&self) -> Option<u32> { match self.prop(P_CLASS_HASH) { Some(Val::U32(v)) => Some(*v), _ => None } }
    pub fn world(&self) -> Option<[f32; 16]> { match self.prop(P_WORLD) { Some(Val::Mat(m)) => Some(*m), _ => None } }
}

fn read_prop(b: &[u8], o: usize) -> Prop {
    let (name_hash, data, ty) = (u32(b, o), u32(b, o + 4), b[o + 8]);
    let val = match ty {
        1 => Val::U32(data),
        2 => Val::F32(f32::from_bits(data)),
        3 => {
            let s = data as usize;
            let mut e = s;
            while e < b.len() && b[e] != 0 { e += 1; }
            Val::Str(String::from_utf8_lossy(&b[s..e]).into_owned())
        }
        8 => {
            let s = data as usize;
            if s + 64 <= b.len() { let mut m = [0f32; 16]; for i in 0..16 { m[i] = f32a(b, s + i * 4); } Val::Mat(m) }
            else { Val::Other(ty, data) }
        }
        _ => Val::Other(ty, data),
    };
    Prop { name_hash, val }
}

fn read_node(b: &[u8], hdr: usize) -> Node {
    let name_hash = u32(b, hdr);
    let data_off = u32(b, hdr + 4) as usize;
    let pc = u16(b, hdr + 8) as usize;
    let cc = u16(b, hdr + 10) as usize;
    let mut props = Vec::with_capacity(pc);
    for i in 0..pc { if data_off + i * 9 + 9 <= b.len() { props.push(read_prop(b, data_off + i * 9)); } }
    let child_off = (data_off + pc * 9 + 3) & !3;
    let mut children = Vec::with_capacity(cc);
    for i in 0..cc { let h = child_off + i * 12; if h + 12 <= b.len() { children.push(read_node(b, h)); } }
    Node { name_hash, props, children }
}

pub fn parse(b: &[u8]) -> Result<Node, String> {
    if b.len() < 20 || &b[0..4] != b"RTPC" { return Err("not an RTPC (bad magic)".into()); }
    if u32(b, 4) != 3 { return Err(format!("RTPC version {} unsupported (need 3)", u32(b, 4))); }
    Ok(read_node(b, 8))
}

/// (part name → world transform) for every render mesh-part node the entity declares. Names are lowercased.
/// A part with no explicit `world` gets identity (positioned by its own geometry / shared object space).
pub fn render_parts(b: &[u8]) -> Result<Vec<(String, [f32; 16])>, String> {
    const IDENT: [f32; 16] = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];
    let root = parse(b)?;
    let mut out = Vec::new();
    fn walk(n: &Node, out: &mut Vec<(String, [f32; 16])>) {
        if n.class_hash() == Some(CLASS_RENDER_PART) {
            if let Some(name) = n.name() { out.push((name.to_ascii_lowercase(), n.world().unwrap_or(IDENT))); }
        }
        for c in &n.children { walk(c, out); }
    }
    walk(&root, &mut out);
    Ok(out)
}

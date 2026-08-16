// jc4_workshop — JC4 asset browser & inspector (wgpu + eframe). "Storm-rig" identity.
//
// The app frame mirrors the Mercs2/Sab workshops: command bar · left icon rail · navigator ·
// viewport · inspector · status bar, with Inspect + Settings pages. Navigator/inspector are wired to
// the real jc4_formats backends (tab/oodle/adf/avtx/sarc/bundle) — archives browse as composite UNITS
// (Models / Structures / Textures / Data), not flat files. The render viewport is intentionally a
// STUB (drawn placeholder); the wgpu paint pass slots into `viewport()` next.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, RichText, Stroke};
use jc4_formats::{adf, amf, avtx, bundle, hash::hashlittle, oodle::Oodle, sarc, tab};

// ── storm-rig palette ──────────────────────────────────────────────────────────────────────────
const G0: Color32 = Color32::from_rgb(0x0b, 0x10, 0x15); // app ground
const G1: Color32 = Color32::from_rgb(0x0f, 0x16, 0x1c); // panels
const G2: Color32 = Color32::from_rgb(0x16, 0x1f, 0x27); // cards / inputs
const G3: Color32 = Color32::from_rgb(0x1e, 0x2a, 0x34); // raised / hover
const LINE: Color32 = Color32::from_rgb(0x24, 0x34, 0x40);
const LINE2: Color32 = Color32::from_rgb(0x37, 0x48, 0x5a);
const TX: Color32 = Color32::from_rgb(0xd6, 0xde, 0xe4);
const DIM: Color32 = Color32::from_rgb(0x8a, 0x97, 0xa3);
const FAINT: Color32 = Color32::from_rgb(0x58, 0x66, 0x74);
const ACC: Color32 = Color32::from_rgb(0xff, 0x7a, 0x33); // explosive orange — live/selected
const ACC_SOFT: Color32 = Color32::from_rgb(0x2a, 0x17, 0x10);
const VOLT: Color32 = Color32::from_rgb(0x3f, 0xd0, 0xd8); // lightning teal — verified/good
const INFO: Color32 = Color32::from_rgb(0x5a, 0xa0, 0xd8);

fn install_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.override_text_color = Some(TX);
    v.panel_fill = G1;
    v.window_fill = G1;
    v.extreme_bg_color = G0;
    v.faint_bg_color = G2;
    v.hyperlink_color = ACC;
    v.selection.bg_fill = ACC_SOFT;
    v.selection.stroke = Stroke::new(1.0, ACC);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = G1;
    w.noninteractive.fg_stroke = Stroke::new(1.0, DIM);
    w.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    w.inactive.bg_fill = G2;
    w.inactive.weak_bg_fill = G2;
    w.inactive.fg_stroke = Stroke::new(1.0, TX);
    w.inactive.bg_stroke = Stroke::new(1.0, LINE);
    w.hovered.bg_fill = G3;
    w.hovered.weak_bg_fill = G3;
    w.hovered.bg_stroke = Stroke::new(1.0, LINE2);
    w.active.bg_fill = G3;
    w.active.bg_stroke = Stroke::new(1.0, ACC);
    ctx.set_visuals(v);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(7.0, 6.0);
    ctx.set_style(style);
}

fn main() -> eframe::Result<()> {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]).with_title("JC4 Workshop"),
        ..Default::default()
    };
    eframe::run_native("jc4_workshop", opts, Box::new(|cc| {
        install_theme(&cc.egui_ctx);
        let mut app = Workshop::default();
        app.config = load_config();
        app.highlights = load_highlights();
        app.apply_config();
        Ok(Box::new(app))
    }))
}

// ── unit taxonomy: classify an entry by its resolved path extension ──────────────────────────────
#[derive(Clone, Copy, PartialEq)]
enum Kind { Model, Structure, Entity, Texture, Ui, Data, Other }
impl Kind {
    fn of_path(p: &str) -> Kind {
        let e = p.rsplit('.').next().unwrap_or("");
        match e {
            "modelc" | "meshc" | "hrmeshc" | "mdic" => Kind::Model,
            "resourcebundle" => Kind::Structure,
            "ee" | "epe" | "epe_adf" => Kind::Entity,
            "ddsc" | "hmddsc" => Kind::Texture,
            "gfx" => Kind::Ui,
            "environc" | "adf" | "aisystunec" | "bin" => Kind::Data,
            _ => Kind::Other,
        }
    }
    fn tag(self) -> (&'static str, Color32) {
        match self {
            Kind::Model => ("MODEL", ACC), Kind::Structure => ("STRUCT", VOLT),
            Kind::Entity => ("ENTITY", INFO), Kind::Texture => ("TEX", VOLT),
            Kind::Ui => ("UI", INFO), Kind::Data => ("DATA", DIM), Kind::Other => ("···", FAINT),
        }
    }
}

// ── decoded inspector views ──────────────────────────────────────────────────────────────────────
struct ModelPart { name: String, lods: usize, lod_factor: f64, materials: Vec<Material>, mesh: Option<MeshStat> }
struct Material { name: String, render_block: String, textures: Vec<String> }
struct MeshStat { groups: usize, meshes: u64, verts: u64, indices: u64, hires: String }
struct BundleView { members: Vec<(u32, u32, usize, String, Vec<(String, String)>)> } // name_hash,type_hash,size,ext,fields
struct SarcView { stored: usize, refs: usize, members: Vec<(String, usize, bool)> }
struct TexView { info: String, dims: Option<(u32, u32, String)> }

enum Preview {
    Model { parts: Vec<ModelPart> },
    Structure(BundleView),
    Sarc(SarcView),
    Texture(TexView),
    Data(String),
    Raw(String),
    Error(String),
}

struct Archive { tab_path: PathBuf, arc_path: PathBuf, tab: tab::Tab }

#[derive(Default, PartialEq, Clone, Copy)]
enum Page { #[default] Inspect, Settings }

#[derive(Default, Clone)]
struct Config { game_dir: String, oodle_dll: String, filelist: String }

/// A curated navigator shortcut (see data/highlights.txt) — jumps to a notable unit across archives.
struct Highlight { tag: String, label: String, archive: String, path: String }
fn load_highlights() -> Vec<Highlight> {
    include_str!("../data/highlights.txt").lines().filter_map(|l| {
        let l = l.trim();
        if l.is_empty() || l.starts_with('#') { return None; }
        let f: Vec<&str> = l.split('|').map(|s| s.trim()).collect();
        if f.len() < 4 { return None; }
        Some(Highlight { tag: f[0].to_string(), label: f[1].to_string(), archive: f[2].to_string(), path: f[3].to_string() })
    }).collect()
}
fn tag_color(tag: &str) -> Color32 {
    match tag { "MODEL" => ACC, "TEX" => VOLT, "STRUCT" => VOLT, "UI" => INFO, _ => DIM }
}

/// Config lives in a text file (no hardcoded paths in the binary). Windows: `%APPDATA%\jc4_workshop\config.txt`.
fn config_path() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("jc4_workshop").join("config.txt")
}
fn load_config() -> Config {
    let mut c = Config::default();
    if let Ok(s) = std::fs::read_to_string(config_path()) {
        for line in s.lines() {
            if let Some((k, v)) = line.split_once('=') {
                let v = v.trim().to_string();
                match k.trim() { "game_dir" => c.game_dir = v, "oodle_dll" => c.oodle_dll = v, "filelist" => c.filelist = v, _ => {} }
            }
        }
    }
    c
}
fn save_config(c: &Config) {
    let p = config_path();
    if let Some(d) = p.parent() { let _ = std::fs::create_dir_all(d); }
    let _ = std::fs::write(&p, format!("game_dir = {}\noodle_dll = {}\nfilelist = {}\n", c.game_dir, c.oodle_dll, c.filelist));
}
/// Discover the game's `.tab` archives under `<game_dir>/archives_win64`, labelled by relative path.
fn discover_archives(game_dir: &str) -> Vec<(String, PathBuf)> {
    let root = Path::new(game_dir).join("archives_win64");
    let mut out = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() { stack.push(p); }
                else if p.extension().map_or(false, |x| x == "tab") {
                    let label = p.strip_prefix(&root).ok()
                        .and_then(|r| r.with_extension("").to_str().map(|s| s.replace('\\', "/")))
                        .unwrap_or_else(|| name_of(&p));
                    out.push((label, p));
                }
            }
        }
    }
    out.sort();
    out
}

#[derive(Default)]
struct Workshop {
    page: Page,
    config: Config,
    archives: Vec<(String, PathBuf)>, // (label, .tab path) discovered under the game dir
    archive: Option<Archive>,
    names: BTreeMap<u32, String>,
    filter: String,
    selected: Option<usize>,
    sel_kind: Option<Kind>,
    preview: Option<Preview>,
    status: String,
    oodle_dll: String,
    oodle: Option<Oodle>,
    tex: Option<egui::TextureHandle>,          // uploaded preview texture (current selection)
    tex_pending: Option<egui::ColorImage>,     // decoded RGBA awaiting upload (needs ctx, done in update)
    mesh: Option<amf::Mesh>,                   // decoded model geometry (current selection)
    orbit: (f32, f32),                         // model viewport camera (yaw, pitch)
    highlights: Vec<Highlight>,                // curated navigator shortcuts
}

fn basename(s: &str) -> &str { s.rsplit(['/', '\\']).next().unwrap_or(s) }
fn name_of(p: &Path) -> String { p.file_name().and_then(|s| s.to_str()).unwrap_or("?").to_string() }

impl Workshop {
    fn open_archive(&mut self, tab_path: PathBuf) {
        let arc_path = tab_path.with_extension("arc");
        match std::fs::read(&tab_path).map_err(|e| e.to_string()).and_then(|b| tab::parse_tab(&b)) {
            Ok(t) => {
                self.status = format!("loaded {} entries from {}", t.entries.len(), name_of(&tab_path));
                self.archive = Some(Archive { tab_path, arc_path, tab: t });
                self.selected = None; self.preview = None;
            }
            Err(e) => self.status = format!("open failed: {e}"),
        }
    }

    fn load_filelist(&mut self, path: PathBuf) {
        if let Ok(txt) = std::fs::read_to_string(&path) {
            for line in txt.lines() {
                let p = line.trim();
                if !p.is_empty() && !p.starts_with(';') { self.names.insert(hashlittle(p.as_bytes(), 0), p.to_string()); }
            }
            self.status = format!("{} known names", self.names.len());
        }
    }

    // Apply the config: derive the Oodle DLL path, discover archives, load the filelist, auto-open a
    // default archive. Called at startup and whenever a Settings path changes.
    fn apply_config(&mut self) {
        self.oodle_dll = if !self.config.oodle_dll.is_empty() { self.config.oodle_dll.clone() }
            else if !self.config.game_dir.is_empty() { format!("{}/oo2core_7_win64.dll", self.config.game_dir) }
            else { String::new() };
        self.oodle = None;
        self.names.clear();
        if !self.config.filelist.is_empty() { self.load_filelist(PathBuf::from(self.config.filelist.clone())); }
        self.archives = if self.config.game_dir.is_empty() { Vec::new() } else { discover_archives(&self.config.game_dir) };
        self.archive = None; self.selected = None; self.preview = None;
        if let Some((_, p)) = self.archives.iter().find(|(l, _)| l == "main/game0").cloned()
            .or_else(|| self.archives.first().cloned()) { self.open_archive(p); }
        self.status = if self.config.game_dir.is_empty() {
            "Set the game folder in Settings to auto-load assets.".into()
        } else {
            format!("{} archives · {} names", self.archives.len(), self.names.len())
        };
    }

    fn entry_kind(&self, e: &tab::Entry) -> Kind {
        self.names.get(&e.name_hash).map(|p| Kind::of_path(p)).unwrap_or(Kind::Other)
    }
    fn entry_label(&self, e: &tab::Entry) -> String {
        self.names.get(&e.name_hash).map(|p| basename(p).to_string()).unwrap_or_else(|| format!("{:08x}", e.name_hash))
    }

    /// Jump to a curated highlight: open its archive (if needed), then select the entry by name hash.
    fn jump(&mut self, archive_label: &str, path: &str) {
        if let Some((_, p)) = self.archives.iter().find(|(l, _)| l == archive_label).cloned() {
            if self.archive.as_ref().map_or(true, |a| a.tab_path != p) { self.open_archive(p); }
        }
        let h = hashlittle(path.as_bytes(), 0);
        match self.archive.as_ref().and_then(|a| a.tab.entries.iter().position(|e| e.name_hash == h)) {
            Some(i) => self.select(i),
            None => self.status = format!("highlight not found in archive: {path}"),
        }
    }

    fn select(&mut self, i: usize) {
        self.selected = Some(i);
        let arc = match &self.archive { Some(a) => a, None => return };
        let entry = arc.tab.entries[i];
        self.sel_kind = Some(self.entry_kind(&entry));
        let path_ext = self.names.get(&entry.name_hash).map(|p| Kind::of_path(p));
        let mut f = match File::open(&arc.arc_path) { Ok(f) => f, Err(e) => { self.preview = Some(Preview::Error(e.to_string())); return; } };
        let data = match tab::decode_entry(&mut f, &arc.tab, &entry, &self.oodle_dll, &mut self.oodle) {
            Ok(d) => d, Err(e) => { self.preview = Some(Preview::Error(format!("decode: {e}"))); return; }
        };
        self.preview = Some(build_preview(&data, path_ext));
        // for a texture, decode the best inline mip to RGBA now (upload happens in update, needs ctx)
        self.tex = None;
        self.tex_pending = None;
        if matches!(self.preview, Some(Preview::Texture(_))) {
            if let Ok(mip) = avtx::best_inline_mip(&data) {
                match avtx::decode_rgba(&mip) {
                    Ok(rgba) => self.tex_pending = Some(egui::ColorImage::from_rgba_unmultiplied([mip.width as usize, mip.height as usize], &rgba)),
                    Err(e) => self.status = format!("texture decode: {e}"),
                }
            }
        }
        // for a model, decode the most-detailed inline mesh part for the viewport
        self.mesh = None;
        if matches!(self.preview, Some(Preview::Model { .. })) {
            if let Ok(members) = sarc::parse(&data) {
                // pair each .meshc with its sibling .hrmeshc (high-detail buffers) by base name
                let hr: BTreeMap<&str, &[u8]> = members.iter()
                    .filter(|m| m.stored && m.name.ends_with(".hrmeshc"))
                    .filter_map(|m| m.data(&data).map(|d| (m.name.trim_end_matches(".hrmeshc"), d)))
                    .collect();
                for mem in &members {
                    if mem.stored && mem.name.ends_with(".meshc") {
                        if let Some(d) = mem.data(&data) {
                            let decoded = match hr.get(mem.name.trim_end_matches(".meshc")) {
                                Some(h) => amf::decode_mesh_hr(d, h),
                                None => amf::decode_mesh(d),
                            };
                            if let Ok(mesh) = decoded {
                                if self.mesh.as_ref().map_or(true, |b| mesh.positions.len() > b.positions.len()) { self.mesh = Some(mesh); }
                            }
                        }
                    }
                }
            }
            if self.mesh.is_some() { self.orbit = (0.6, 0.35); }
        }
    }
}

fn build_preview(data: &[u8], path_ext: Option<Kind>) -> Preview {
    // resourcebundle has no magic — recognized only by path extension
    if path_ext == Some(Kind::Structure) {
        return Preview::Structure(build_bundle(data));
    }
    match tab::magic_ext(data) {
        "sarc" => match sarc::parse(data) {
            Ok(members) => {
                if members.iter().any(|m| m.stored && m.name.ends_with(".modelc")) {
                    Preview::Model { parts: build_model(data, &members) }
                } else {
                    let stored = members.iter().filter(|m| m.stored).count();
                    Preview::Sarc(SarcView { stored, refs: members.len() - stored,
                        members: members.iter().map(|m| (m.name.clone(), m.size, m.stored)).collect() })
                }
            }
            Err(e) => Preview::Error(e),
        },
        "avtx" => match avtx::parse_header(data) {
            Ok(h) => {
                let mip = avtx::best_inline_mip(data).ok();
                let dims = mip.map(|m| (m.width, m.height, avtx::dxgi_name(m.dxgi_format).to_string()));
                let info = format!("{}x{}x{}  {} ({:#x})\nmips {}/{} inline{}{}",
                    h.width, h.height, h.depth, avtx::dxgi_name(h.dxgi_format), h.dxgi_format,
                    h.mip_inline, h.mip_total, if h.is_cubemap() { "  [cubemap]" } else { "" },
                    if h.has_external_mips() { "  [external hi-res]" } else { "" });
                Preview::Texture(TexView { info, dims })
            }
            Err(e) => Preview::Error(e),
        },
        "adf" => match adf::parse(data.to_vec()) {
            Ok(a) => Preview::Data(serde_json::to_string_pretty(&a.decode_instances()).unwrap_or_default()),
            Err(e) => Preview::Error(e),
        },
        _ => Preview::Raw(hexdump(data, 768)),
    }
}

fn build_bundle(data: &[u8]) -> BundleView {
    let mut members = Vec::new();
    for m in bundle::parse(data) {
        let d = m.data(data);
        let ext = tab::magic_ext(d).to_string();
        let mut fields = Vec::new();
        if ext == "adf" {
            if let Ok(a) = adf::parse(d.to_vec()) {
                let v = a.decode_instances();
                if let Some(root) = v.as_object() {
                    for (name, val) in root {
                        fields.push((name.clone(), String::new()));
                        if let Some(o) = val.as_object() {
                            for (k, fv) in o {
                                let s = if let Some(arr) = fv.as_array() { format!("[{}]", arr.len()) }
                                    else if let Some(o2) = fv.as_object() { format!("{{{}}}", o2.len()) }
                                    else { let t = fv.to_string(); if t.len() > 40 { t[..40].to_string() } else { t } };
                                fields.push((k.clone(), s));
                            }
                        }
                    }
                }
            }
        }
        members.push((m.name_hash, m.type_hash, m.size, ext, fields));
    }
    BundleView { members }
}

fn build_model(data: &[u8], members: &[sarc::SarcEntry]) -> Vec<ModelPart> {
    let by_name: std::collections::HashMap<&str, &sarc::SarcEntry> = members.iter().map(|m| (m.name.as_str(), m)).collect();
    let mut parts = Vec::new();
    for mc in members.iter().filter(|m| m.stored && m.name.ends_with(".modelc")) {
        let bytes = match mc.data(data) { Some(b) => b, None => continue };
        let a = match adf::parse(bytes.to_vec()) { Ok(a) => a, Err(_) => continue };
        let v = a.decode_instances();
        let model = match v.as_object().and_then(|o| o.values().next()) { Some(m) => m, None => continue };
        let lods = model.get("LodSlots").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
        let lod_factor = model.get("LodFactor").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let mut materials = Vec::new();
        if let Some(mats) = model.get("Materials").and_then(|v| v.as_array()) {
            for mat in mats {
                materials.push(Material {
                    name: mat.get("Name").and_then(|v| v.as_str()).unwrap_or("?").to_string(),
                    render_block: mat.get("RenderBlockId").and_then(|v| v.as_str()).unwrap_or("?").to_string(),
                    textures: mat.get("Textures").and_then(|v| v.as_array()).map(|ts| ts.iter()
                        .filter_map(|v| v.as_str()).filter(|t| !t.is_empty() && !t.contains("dummies/"))
                        .map(|t| basename(t).to_string()).collect()).unwrap_or_default(),
                });
            }
        }
        let mesh = model.get("Mesh").and_then(|v| v.as_str())
            .and_then(|mp| by_name.get(mp).and_then(|m| m.data(data)))
            .and_then(mesh_stat);
        parts.push(ModelPart { name: basename(&mc.name).to_string(), lods, lod_factor, materials, mesh });
    }
    parts
}

fn mesh_stat(data: &[u8]) -> Option<MeshStat> {
    let a = adf::parse(data.to_vec()).ok()?;
    let v = a.decode_instances();
    let hdr = v.as_object()?.values().find(|iv| iv.get("LodGroups").is_some())?;
    let groups = hdr.get("LodGroups").and_then(|v| v.as_array())?;
    let (mut meshes, mut verts, mut indices) = (0u64, 0u64, 0u64);
    for g in groups {
        if let Some(ms) = g.get("Meshes").and_then(|v| v.as_array()) {
            for m in ms {
                meshes += 1;
                verts += m.get("VertexCount").and_then(|v| v.as_u64()).unwrap_or(0);
                indices += m.get("IndexCount").and_then(|v| v.as_u64()).unwrap_or(0);
            }
        }
    }
    Some(MeshStat { groups: groups.len(), meshes, verts, indices,
        hires: hdr.get("HighLodPath").and_then(|v| v.as_str()).map(|s| basename(s).to_string()).unwrap_or_default() })
}

fn hexdump(b: &[u8], max: usize) -> String {
    let mut s = String::new();
    for (i, row) in b[..max.min(b.len())].chunks(16).enumerate() {
        let hex: String = row.iter().map(|x| format!("{:02x} ", x)).collect();
        let asc: String = row.iter().map(|&x| if (0x20..0x7f).contains(&x) { x as char } else { '.' }).collect();
        s.push_str(&format!("{:08x}  {:<48}{}\n", i * 16, hex, asc));
    }
    s
}

// ── small UI helpers ─────────────────────────────────────────────────────────────────────────────
fn eyebrow(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(2.0, 11.0), egui::Sense::hover());
        ui.painter().rect_filled(r, 0.0, ACC);
        ui.add_space(3.0);
        ui.label(RichText::new(text.to_uppercase()).color(DIM).size(10.0).strong());
    });
}

fn card(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none().fill(G2).stroke(Stroke::new(1.0, LINE)).rounding(6.0).inner_margin(egui::Margin::same(11.0))
        .show(ui, |ui| {
            eyebrow(ui, title);
            ui.add_space(6.0);
            body(ui);
        });
    ui.add_space(9.0);
}

// Vector rail icons drawn with the painter (no font/emoji dependency → no tofu).
fn draw_icon(p: &egui::Painter, c: egui::Pos2, s: f32, col: Color32, id: u8) {
    use egui::pos2;
    let st = Stroke::new(1.6, col);
    let seg = |a: egui::Pos2, b: egui::Pos2| p.line_segment([a, b], st);
    match id {
        0 => { // inspect — magnifier
            p.circle_stroke(pos2(c.x - s * 0.18, c.y - s * 0.18), s * 0.55, st);
            seg(pos2(c.x + s * 0.22, c.y + s * 0.22), pos2(c.x + s * 0.72, c.y + s * 0.72));
        }
        1 => { // settings — sliders
            for (i, ky) in [-0.55f32, 0.05, 0.65].iter().enumerate() {
                let y = c.y + s * ky;
                seg(pos2(c.x - s * 0.75, y), pos2(c.x + s * 0.75, y));
                p.circle_filled(pos2(c.x + s * 0.45 * if i == 1 { -1.0 } else { 1.0 }, y), 2.4, col);
            }
        }
        2 => { // models — isometric cube
            let pts = vec![pos2(c.x, c.y - s), pos2(c.x + s * 0.87, c.y - s * 0.5),
                pos2(c.x + s * 0.87, c.y + s * 0.5), pos2(c.x, c.y + s),
                pos2(c.x - s * 0.87, c.y + s * 0.5), pos2(c.x - s * 0.87, c.y - s * 0.5)];
            p.add(egui::Shape::closed_line(pts, st));
            seg(c, pos2(c.x, c.y - s));
            seg(c, pos2(c.x + s * 0.87, c.y + s * 0.5));
            seg(c, pos2(c.x - s * 0.87, c.y + s * 0.5));
        }
        _ => { // pack — box with band
            let r = egui::Rect::from_center_size(c, egui::vec2(s * 1.6, s * 1.7));
            p.rect_stroke(r, 1.0, st);
            let y = r.top() + r.height() * 0.34;
            seg(pos2(r.left(), y), pos2(r.right(), y));
            seg(pos2(c.x, r.top()), pos2(c.x, y));
        }
    }
}

fn rail_btn(ui: &mut egui::Ui, icon: u8, label: &str, on: bool, soon: bool) -> bool {
    let color = if on { ACC } else if soon { FAINT } else { DIM };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 54.0),
        if soon { egui::Sense::hover() } else { egui::Sense::click() });
    if on { ui.painter().rect_filled(egui::Rect::from_min_size(rect.left_top(), egui::vec2(3.0, rect.height())), 0.0, ACC); }
    if resp.hovered() && !soon { ui.painter().rect_filled(rect, 0.0, Color32::from_white_alpha(4)); }
    draw_icon(ui.painter(), rect.center() - egui::vec2(0.0, 7.0), 9.0, color, icon);
    ui.painter().text(rect.center() + egui::vec2(0.0, 14.0), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(8.5), color);
    resp.clicked()
}

impl eframe::App for Workshop {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // command bar
        egui::TopBottomPanel::top("cmd").exact_height(46.0)
            .frame(egui::Frame::none().fill(G0).inner_margin(egui::Margin::symmetric(14.0, 8.0)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("JUST CAUSE 4").color(TX).size(14.0).strong());
                    ui.label(RichText::new("WORKSHOP").color(FAINT).size(9.0).strong());
                    ui.add_space(8.0);
                    ui.separator();
                    if let Some(a) = &self.archive {
                        ui.label(RichText::new(name_of(&a.tab_path)).color(DIM).monospace());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Filelist…").clicked() {
                            if let Some(p) = rfd::FileDialog::new().add_filter("filelist", &["filelist", "txt"]).pick_file() { self.load_filelist(p); }
                        }
                        if ui.button("Open .tab…").clicked() {
                            if let Some(p) = rfd::FileDialog::new().add_filter("TAB", &["tab"]).pick_file() { self.open_archive(p); }
                        }
                    });
                });
            });
        // status bar
        egui::TopBottomPanel::bottom("status").exact_height(26.0)
            .frame(egui::Frame::none().fill(G0).inner_margin(egui::Margin::symmetric(13.0, 5.0)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("●").color(VOLT).size(9.0));
                    ui.label(RichText::new(&self.status).color(DIM).size(11.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(if self.oodle.is_some() { "Oodle: loaded" } else { "Oodle: idle" }).color(FAINT).monospace().size(11.0));
                        ui.label(RichText::new(format!("{} names", self.names.len())).color(FAINT).monospace().size(11.0));
                    });
                });
            });
        // rail
        egui::SidePanel::left("rail").exact_width(62.0).resizable(false)
            .frame(egui::Frame::none().fill(G0)).show(ctx, |ui| {
                ui.add_space(6.0);
                if rail_btn(ui, 0, "INSPECT", self.page == Page::Inspect, false) { self.page = Page::Inspect; }
                if rail_btn(ui, 1, "SETTINGS", self.page == Page::Settings, false) { self.page = Page::Settings; }
                ui.add_space(ui.available_height() - 108.0);
                rail_btn(ui, 2, "MODELS", false, true);
                rail_btn(ui, 3, "PACK", false, true);
            });

        match self.page {
            Page::Inspect => self.ui_inspect(ctx),
            Page::Settings => self.ui_settings(ctx),
        }
    }
}

impl Workshop {
    fn ui_inspect(&mut self, ctx: &egui::Context) {
        let archives = self.archives.clone();
        let cur_label = self.archive.as_ref()
            .and_then(|a| archives.iter().find(|(_, p)| *p == a.tab_path).map(|(l, _)| l.clone()))
            .unwrap_or_else(|| "—".to_string());
        let mut switch: Option<PathBuf> = None;
        let mut jump: Option<(String, String)> = None;
        egui::SidePanel::left("nav").default_width(310.0).frame(egui::Frame::none().fill(G1).inner_margin(12.0))
            .show(ctx, |ui| {
                ui.label(RichText::new("ARCHIVE").color(TX).size(15.0).strong());
                ui.add_space(5.0);
                if archives.is_empty() {
                    ui.label(RichText::new("no game folder set — see Settings").color(FAINT).size(10.5));
                } else {
                    egui::ComboBox::from_id_salt("arch")
                        .selected_text(RichText::new(&cur_label).monospace().size(12.0).color(ACC))
                        .width(ui.available_width() - 4.0)
                        .show_ui(ui, |ui| {
                            for (label, path) in &archives {
                                if ui.selectable_label(*label == cur_label, RichText::new(label).monospace().size(11.5)).clicked() {
                                    switch = Some(path.clone());
                                }
                            }
                        });
                }
                // curated highlights — jump straight to notable units, across archives
                if !self.highlights.is_empty() {
                    ui.add_space(8.0);
                    egui::CollapsingHeader::new(RichText::new("★ HIGHLIGHTS").color(VOLT).size(11.0).strong())
                        .default_open(true).show(ui, |ui| {
                            for hl in &self.highlights {
                                let resp = ui.horizontal(|ui| {
                                    ui.label(RichText::new(&hl.tag).color(tag_color(&hl.tag)).size(8.0).strong());
                                    ui.add(egui::Label::new(RichText::new(&hl.label).size(11.0).color(TX)).truncate());
                                }).response;
                                if ui.interact(resp.rect, egui::Id::new(("hl", &hl.path)), egui::Sense::click()).clicked() {
                                    jump = Some((hl.archive.clone(), hl.path.clone()));
                                }
                            }
                        });
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("filter").color(FAINT).size(10.0));
                    ui.add(egui::TextEdit::singleline(&mut self.filter).desired_width(f32::INFINITY).hint_text("name or hash"));
                });
                ui.add_space(6.0);
                let mut pick = None;
                match &self.archive {
                    None => { ui.add_space(12.0); ui.label(RichText::new("No archive open — pick one above, or set the game folder in Settings.").color(FAINT)); }
                    Some(a) => {
                        let f = self.filter.to_lowercase();
                        let visible: Vec<usize> = a.tab.entries.iter().enumerate().filter(|(_, e)| {
                            if f.is_empty() { return true; }
                            self.names.get(&e.name_hash).map_or(false, |n| n.to_lowercase().contains(&f))
                                || format!("{:08x}", e.name_hash).contains(&f)
                        }).map(|(i, _)| i).collect();
                        eyebrow(ui, &format!("entries · {}", visible.len()));
                        ui.add_space(4.0);
                        let row_h = 20.0;
                        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, row_h, visible.len(), |ui, range| {
                            for row in range {
                                let i = visible[row];
                                let e = &a.tab.entries[i];
                                let (tag, tcol) = self.entry_kind(e).tag();
                                let sel = self.selected == Some(i);
                                let resp = ui.horizontal(|ui| {
                                    ui.label(RichText::new(tag).color(tcol).size(8.5).strong());
                                    let lbl = RichText::new(self.entry_label(e)).monospace().size(11.5)
                                        .color(if sel { ACC } else { TX });
                                    ui.add(egui::Label::new(lbl).truncate())
                                }).response;
                                if ui.interact(resp.rect, egui::Id::new(("row", i)), egui::Sense::click()).clicked() { pick = Some(i); }
                            }
                        });
                    }
                }
                if let Some(i) = pick { self.select(i); }
            });
        if let Some(p) = switch { self.open_archive(p); }
        if let Some((arch, path)) = jump { self.jump(&arch, &path); }
        // upload a freshly-decoded texture (ctx available here)
        if let Some(img) = self.tex_pending.take() {
            self.tex = Some(ctx.load_texture("preview_tex", img, egui::TextureOptions::LINEAR));
        }

        // inspector
        egui::SidePanel::right("insp").default_width(372.0).frame(egui::Frame::none().fill(G1).inner_margin(13.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.ui_inspector(ui));
            });

        // viewport
        egui::CentralPanel::default().frame(egui::Frame::none().fill(G0)).show(ctx, |ui| {
            // key off a decoded mesh, not the path-derived Kind: model geometry rides inside `.ee`
            // (Entity) and SARC entries, so sel_kind is rarely Model even when a mesh is present.
            if self.mesh.is_some() {
                self.model_viewport(ui);
            } else {
                let dims = match &self.preview {
                    Some(Preview::Texture(t)) => t.dims.clone(),
                    _ => None,
                };
                viewport(ui, dims.as_ref(), self.sel_kind, self.tex.as_ref());
            }
        });
    }

    /// Software-rendered model preview: orbit camera (drag), flat shading, painter's-algorithm depth
    /// sort, drawn as a single egui mesh. No wgpu pipeline — fast enough for the inline LOD meshes.
    fn model_viewport(&mut self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        let resp = ui.interact(rect, egui::Id::new("model_vp"), egui::Sense::drag());
        if resp.dragged() {
            let d = resp.drag_delta();
            self.orbit.0 += d.x * 0.01;
            self.orbit.1 = (self.orbit.1 + d.y * 0.01).clamp(-1.5, 1.5);
        }
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 0.0, G0);
        let inner = rect.shrink(14.0);
        p.rect(inner, 6.0, Color32::from_rgb(0x10, 0x18, 0x1e), Stroke::new(1.5, LINE));
        let mesh = self.mesh.as_ref().unwrap();

        let (yaw, pitch) = self.orbit;
        let (cy, sy, cp, sp) = (yaw.cos(), yaw.sin(), pitch.cos(), pitch.sin());
        let rot = |v: [f32; 3]| -> [f32; 3] {
            let (x, z) = (v[0] * cy + v[2] * sy, -v[0] * sy + v[2] * cy);
            let (y, z2) = (v[1] * cp - z * sp, v[1] * sp + z * cp);
            [x, y, z2]
        };
        let c = [0, 1, 2].map(|i| (mesh.bbox_min[i] + mesh.bbox_max[i]) * 0.5);
        let radius = (0..3).map(|i| (mesh.bbox_max[i] - mesh.bbox_min[i]).abs()).fold(1e-6f32, f32::max) * 0.5;
        let avail = inner.shrink(20.0);
        let scale = avail.width().min(avail.height()) / (2.0 * radius) * 0.85;
        let center = inner.center();
        let proj: Vec<([f32; 3], egui::Pos2)> = mesh.positions.iter().map(|v| {
            let r = rot([v[0] - c[0], v[1] - c[1], v[2] - c[2]]);
            (r, egui::pos2(center.x + r[0] * scale, center.y - r[1] * scale))
        }).collect();

        let light = { let l = [0.35f32, 0.5, 0.79]; let n = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt(); [l[0] / n, l[1] / n, l[2] / n] };
        let mut tris: Vec<(f32, [usize; 3], f32)> = Vec::with_capacity(mesh.indices.len() / 3);
        for t in mesh.indices.chunks_exact(3) {
            let (a, b, cc) = (t[0] as usize, t[1] as usize, t[2] as usize);
            let (pa, pb, pc) = (proj[a].0, proj[b].0, proj[cc].0);
            let u = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
            let w = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
            let n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
            let nl = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-9);
            let shade = ((n[0] * light[0] + n[1] * light[1] + n[2] * light[2]) / nl).abs() * 0.8 + 0.2;
            tris.push(((pa[2] + pb[2] + pc[2]) / 3.0, [a, b, cc], shade));
        }
        tris.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal)); // far -> near

        let mut m = egui::epaint::Mesh::default();
        for (_, [a, b, cc], sh) in &tris {
            let col = Color32::from_rgb((ACC.r() as f32 * sh) as u8, (ACC.g() as f32 * sh) as u8, (ACC.b() as f32 * sh) as u8);
            let i0 = m.vertices.len() as u32;
            for &vi in &[*a, *b, *cc] {
                m.vertices.push(egui::epaint::Vertex { pos: proj[vi].1, uv: egui::epaint::WHITE_UV, color: col });
            }
            m.indices.extend([i0, i0 + 1, i0 + 2]);
        }
        p.add(egui::Shape::mesh(m));
        p.text(inner.left_top() + egui::vec2(11.0, 13.0), egui::Align2::LEFT_CENTER, "MODEL", egui::FontId::proportional(10.0), ACC);
        p.text(inner.right_top() + egui::vec2(-11.0, 13.0), egui::Align2::RIGHT_CENTER,
            format!("{} verts · {} tris · drag to orbit", mesh.positions.len(), mesh.indices.len() / 3),
            egui::FontId::monospace(10.0), DIM);
    }

    fn ui_inspector(&self, ui: &mut egui::Ui) {
        match &self.preview {
            None => { ui.add_space(16.0); ui.label(RichText::new("Pick an entry to inspect.").color(FAINT)); }
            Some(Preview::Model { parts }) => {
                ui.label(RichText::new("MODEL").color(ACC).size(9.5).strong());
                ui.label(RichText::new(format!("{} part(s)", parts.len())).color(TX).size(18.0).strong());
                ui.add_space(8.0);
                for p in parts {
                    card(ui, &p.name, |ui| {
                        ui.label(RichText::new(format!("{} LOD(s) · factor {:.2}", p.lods, p.lod_factor)).monospace().size(11.5).color(DIM));
                        if let Some(m) = &p.mesh {
                            ui.label(RichText::new(format!("{} LOD grp · {} mesh · {} verts · {} idx", m.groups, m.meshes, m.verts, m.indices)).monospace().size(11.5).color(TX));
                            if !m.hires.is_empty() { ui.label(RichText::new(format!("hi-res: {}", m.hires)).monospace().size(10.5).color(FAINT)); }
                        }
                        for mat in &p.materials {
                            ui.add_space(4.0);
                            ui.label(RichText::new(format!("{}   [{}]", mat.name, mat.render_block)).size(12.0).color(VOLT));
                            for t in &mat.textures { ui.label(RichText::new(format!("    {}", t)).monospace().size(10.5).color(DIM)); }
                        }
                    });
                }
            }
            Some(Preview::Structure(bv)) => {
                ui.label(RichText::new("STRUCTURE").color(VOLT).size(9.5).strong());
                ui.label(RichText::new(format!("{} member(s)", bv.members.len())).color(TX).size(18.0).strong());
                ui.add_space(8.0);
                for (nh, th, size, ext, fields) in &bv.members {
                    card(ui, &format!("{ext} · {size} B"), |ui| {
                        ui.label(RichText::new(format!("name {nh:08x}  type {th:08x}")).monospace().size(10.0).color(FAINT));
                        ui.add_space(3.0);
                        for (k, val) in fields {
                            if val.is_empty() { ui.label(RichText::new(k).size(11.5).color(VOLT).strong()); }
                            else { ui.horizontal(|ui| {
                                ui.label(RichText::new(k).monospace().size(11.0).color(DIM));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(RichText::new(val).monospace().size(11.0).color(TX)));
                            }); }
                        }
                    });
                }
            }
            Some(Preview::Sarc(sv)) => {
                ui.label(RichText::new("BUNDLE (SARC)").color(INFO).size(9.5).strong());
                ui.label(RichText::new(format!("{} stored · {} ref", sv.stored, sv.refs)).color(TX).size(16.0).strong());
                ui.add_space(6.0);
                egui::Frame::none().fill(G2).stroke(Stroke::new(1.0, LINE)).rounding(6.0).inner_margin(10.0).show(ui, |ui| {
                    for (name, size, stored) in &sv.members {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(if *stored { "+" } else { "-" }).color(if *stored { VOLT } else { FAINT }));
                            ui.add(egui::Label::new(RichText::new(basename(name)).monospace().size(10.5).color(if *stored { TX } else { FAINT })).truncate());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(RichText::new(format!("{size}")).monospace().size(10.0).color(FAINT)));
                        });
                    }
                });
            }
            Some(Preview::Texture(t)) => {
                ui.label(RichText::new("TEXTURE (AVTX)").color(VOLT).size(9.5).strong());
                ui.add_space(6.0);
                card(ui, "format", |ui| { ui.label(RichText::new(&t.info).monospace().size(12.0).color(TX)); });
            }
            Some(Preview::Data(json)) => {
                ui.label(RichText::new("DATA (ADF → JSON)").color(DIM).size(9.5).strong());
                ui.add_space(6.0);
                ui.label(RichText::new(json).monospace().size(11.0).color(TX));
            }
            Some(Preview::Raw(hex)) => {
                ui.label(RichText::new("RAW").color(DIM).size(9.5).strong());
                ui.add_space(6.0);
                ui.label(RichText::new(hex).monospace().size(10.5).color(DIM));
            }
            Some(Preview::Error(e)) => { ui.colored_label(Color32::from_rgb(0xe0, 0x43, 0x2b), e); }
        }
    }

    fn ui_settings(&mut self, ctx: &egui::Context) {
        let reload = std::cell::Cell::new(false);
        egui::CentralPanel::default().frame(egui::Frame::none().fill(G1).inner_margin(24.0)).show(ctx, |ui| {
            ui.label(RichText::new("SETTINGS").color(TX).size(20.0).strong());
            ui.add_space(4.0);
            ui.label(RichText::new(format!("Saved to {}. No paths are baked into the app.", config_path().display())).color(DIM).size(12.0));
            ui.add_space(14.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                card(ui, "paths", |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("GAME FOLDER").color(DIM).size(9.5).strong());
                        if ui.button("Browse…").clicked() {
                            if let Some(d) = rfd::FileDialog::new().pick_folder() {
                                self.config.game_dir = d.to_string_lossy().replace('\\', "/");
                                save_config(&self.config); reload.set(true);
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !self.config.game_dir.is_empty() { ok_check(ui); }
                            let g = if self.config.game_dir.is_empty() { "(not set — Browse to the Just Cause 4 folder)".to_string() } else { self.config.game_dir.clone() };
                            ui.add(egui::Label::new(RichText::new(g).monospace().size(11.0).color(TX)).truncate());
                        });
                    });
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("FILELIST").color(DIM).size(9.5).strong());
                        if ui.button("Browse…").clicked() {
                            if let Some(f) = rfd::FileDialog::new().add_filter("filelist", &["filelist", "txt"]).pick_file() {
                                self.config.filelist = f.to_string_lossy().replace('\\', "/");
                                save_config(&self.config); reload.set(true);
                            }
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !self.names.is_empty() { ok_check(ui); }
                            let fl = if self.config.filelist.is_empty() { "(optional — a gibbed filelist un-hashes names)".to_string() }
                                else { format!("{} names · {}", self.names.len(), basename(&self.config.filelist)) };
                            ui.add(egui::Label::new(RichText::new(fl).monospace().size(11.0).color(TX)).truncate());
                        });
                    });
                    ui.add_space(5.0);
                    let oodle_ok = std::path::Path::new(&self.oodle_dll).exists();
                    setting_row(ui, "Oodle DLL", if self.oodle_dll.is_empty() { "(derived from game folder)" } else { &self.oodle_dll }, oodle_ok);
                });
                card(ui, "rendering", |ui| {
                    setting_row(ui, "Backend", "wgpu", true);
                    setting_row(ui, "Viewport", "render pass — pending (stubbed)", false);
                });
                card(ui, "appearance", |ui| {
                    setting_row(ui, "Theme", "Storm dark", true);
                });
                card(ui, "about · v0.1.0", |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for f in ["TAB/ARC v2", "Oodle", "ADF", "AVTX", "SARC", "resourcebundle", "name hash"] {
                            egui::Frame::none().fill(Color32::from_rgb(0x0e, 0x24, 0x29)).stroke(Stroke::new(1.0, Color32::from_rgb(0x2b, 0x8f, 0x96)))
                                .rounding(999.0).inner_margin(egui::Margin::symmetric(9.0, 3.0)).show(ui, |ui| {
                                    ui.label(RichText::new(f).monospace().size(10.5).color(VOLT));
                                });
                        }
                    });
                });
            });
        });
        if reload.get() { self.apply_config(); }
    }
}

fn ok_check(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(13.0, 12.0), egui::Sense::hover());
    let c = r.center();
    let st = Stroke::new(1.7, VOLT);
    ui.painter().line_segment([c + egui::vec2(-3.5, 0.0), c + egui::vec2(-1.0, 2.6)], st);
    ui.painter().line_segment([c + egui::vec2(-1.0, 2.6), c + egui::vec2(3.6, -3.2)], st);
}

fn setting_row(ui: &mut egui::Ui, label: &str, value: &str, ok: bool) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label.to_uppercase()).color(DIM).size(9.5).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ok { ok_check(ui); }
            ui.add(egui::Label::new(RichText::new(value).monospace().size(11.0).color(TX)).truncate());
        });
    });
    ui.add_space(3.0);
}

/// Render surface — STUB. The wgpu paint callback that uploads a texture mip / draws a model goes here.
fn viewport(ui: &mut egui::Ui, dims: Option<&(u32, u32, String)>, kind: Option<Kind>, tex: Option<&egui::TextureHandle>) {
    let rect = ui.available_rect_before_wrap();
    let p = ui.painter();
    p.rect_filled(rect, 0.0, G0);
    let inner = rect.shrink(14.0);
    p.rect(inner, 6.0, Color32::from_rgb(0x10, 0x18, 0x1e), Stroke::new(1.5, LINE));

    // textured: paint the mip fit-to-frame over an alpha checkerboard
    if let (Some(Kind::Texture), Some(t)) = (kind, tex) {
        let sz = t.size();
        let (iw, ih) = (sz[0] as f32, sz[1] as f32);
        let avail = inner.shrink(16.0);
        let scale = (avail.width() / iw).min(avail.height() / ih); // fit, preserve aspect (may upscale small mips)
        let draw = egui::vec2(iw * scale, ih * scale);
        let img_rect = egui::Rect::from_center_size(inner.center(), draw);
        checkerboard(&p, img_rect);
        p.image(t.id(), img_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
        p.rect_stroke(img_rect, 0.0, Stroke::new(1.0, LINE2));
        p.text(inner.left_top() + egui::vec2(11.0, 13.0), egui::Align2::LEFT_CENTER, "TEXTURE", egui::FontId::proportional(10.0), VOLT);
        if let Some((w, h, f)) = dims {
            p.text(inner.right_top() + egui::vec2(-11.0, 13.0), egui::Align2::RIGHT_CENTER, format!("{w}×{h} · {f}"), egui::FontId::monospace(10.0), DIM);
        }
        return;
    }

    let msg = match (kind, dims) {
        (Some(Kind::Texture), _) => "◍  texture — no renderable inline mip\n(base mip may be external hi-res, or an HDR format)".to_string(),
        (Some(Kind::Model), _) => "◍  model viewport — not wired yet\n(next: AMF geometry decode → mesh render)".to_string(),
        _ => "◍  render viewport — not wired yet".to_string(),
    };
    p.text(inner.center(), egui::Align2::CENTER_CENTER, msg, egui::FontId::monospace(13.0), FAINT);
    p.text(inner.left_top() + egui::vec2(11.0, 13.0), egui::Align2::LEFT_CENTER, "RENDER · PENDING", egui::FontId::proportional(10.0), INFO);
}

/// Alpha checkerboard behind a (possibly transparent) texture, clipped to `rect`.
fn checkerboard(p: &egui::Painter, rect: egui::Rect) {
    const C: f32 = 9.0;
    let (a, b) = (Color32::from_rgb(0x1a, 0x22, 0x29), Color32::from_rgb(0x12, 0x18, 0x1e));
    p.rect_filled(rect, 0.0, b);
    let (cols, rows) = ((rect.width() / C).ceil() as i32, (rect.height() / C).ceil() as i32);
    for y in 0..rows {
        for x in 0..cols {
            if (x + y) % 2 != 0 { continue; }
            let min = rect.min + egui::vec2(x as f32 * C, y as f32 * C);
            let cell = egui::Rect::from_min_size(min, egui::vec2(C, C)).intersect(rect);
            p.rect_filled(cell, 0.0, a);
        }
    }
}

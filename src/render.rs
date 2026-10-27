//! Rasterisation of the 3D and flat views.
//!
//! All lengths are fractions of the image size, so every size shows the same
//! picture. Changing any constant here changes every published agenticon.

use std::error::Error;
use std::f64::consts::{PI, TAU};
use std::fmt;

use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap,
    Stroke, Transform,
};

use crate::color::{Palette, Rgb};
use crate::genome::{EdgeStyle, RING_SEGMENTS, VisualGenes};
use crate::geometry::{Mesh, Role, centroid, dot, normalize};

/// Projected radius of a unit-sphere solid at depth 0.
const SOLID_SCALE: f64 = 0.27;
/// Camera distance from the origin in the 3D view; smaller means stronger
/// perspective.
const CAMERA_DIST: f64 = 4.0;
/// Camera tilt of pitch step 0, in degrees.
const TILT_MIN_DEG: f64 = 15.0;
/// Tilt range covered by pitch steps 0..=7, in degrees.
const TILT_RANGE_DEG: f64 = 60.0;
/// Light direction for Lambert shading (view space, normalised before use).
const LIGHT_DIR: [f64; 3] = [-0.45, 0.6, 0.65];
/// Brightness of a face facing away from the light.
const AMBIENT: f64 = 0.4;
/// Extra brightness of a face facing the light.
const DIFFUSE: f64 = 0.75;

/// Outline colour of the dark edge styles.
const EDGE_DARK: Rgb = Rgb::new(20, 20, 28);
/// Outline colour of the light edge style.
const EDGE_LIGHT: Rgb = Rgb::new(245, 245, 245);
const EDGE_THIN: f32 = 0.006;
const EDGE_THICK: f32 = 0.014;

/// Ring radius.
const RING_RADIUS: f64 = 0.43;
/// Width of the always-drawn ring track.
const RING_TRACK_WIDTH: f64 = 0.008;
/// Width of a lit ring segment.
const RING_LIT_WIDTH: f64 = 0.035;
/// Angular gap at each end of a lit segment, in degrees.
const RING_GAP_DEG: f64 = 1.5;
/// Polyline points per full circle when approximating arcs.
const ARC_STEPS_PER_TURN: f64 = 180.0;
/// Half-width of the anchor notch.
const ANCHOR_HALF_WIDTH: f64 = 0.025;
/// How far the anchor notch reaches outside and inside the ring.
const ANCHOR_OUTER: f64 = 0.045;
const ANCHOR_INNER: f64 = 0.02;

/// Which view of an agenticon to render.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum View {
    /// Shaded perspective rendering of the 3D solid.
    Solid3d,
    /// The solid flattened along its axis: orthographic and unlit.
    Flat,
}

/// Rendering an agenticon failed.
#[derive(Debug)]
pub struct RenderError {
    kind: RenderErrorKind,
}

#[derive(Debug)]
enum RenderErrorKind {
    InvalidSize(u32),
    DegenerateFace,
    Encode(Box<dyn Error + Send + Sync>),
}

impl RenderError {
    /// Whether the requested image size was zero or too large.
    #[must_use]
    pub fn is_invalid_size(&self) -> bool {
        matches!(self.kind, RenderErrorKind::InvalidSize(_))
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            RenderErrorKind::InvalidSize(size) => {
                write!(f, "invalid image size {size}")
            }
            RenderErrorKind::DegenerateFace => f.write_str("degenerate face"),
            RenderErrorKind::Encode(_) => f.write_str("png encoding failed"),
        }
    }
}

impl Error for RenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            RenderErrorKind::Encode(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

fn paint(c: Rgb) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(c.r, c.g, c.b, 255);
    p.anti_alias = true;
    p
}

fn role_color(pal: &Palette, role: Role) -> Rgb {
    match role {
        Role::Cap => pal.base,
        Role::SideA => pal.accent1,
        Role::SideB => pal.accent2,
    }
}

fn edge_stroke(edge: EdgeStyle, size: f32) -> Option<(Rgb, Stroke)> {
    let (color, width) = match edge {
        EdgeStyle::None => return None,
        EdgeStyle::ThinDark => (EDGE_DARK, EDGE_THIN),
        EdgeStyle::ThinLight => (EDGE_LIGHT, EDGE_THIN),
        EdgeStyle::ThickDark => (EDGE_DARK, EDGE_THICK),
    };
    let stroke = Stroke {
        width: width * size,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    Some((color, stroke))
}

/// Renders a view into a square pixmap of `size` pixels.
pub(crate) fn render(
    g: &VisualGenes,
    view: View,
    size: u32,
) -> Result<Pixmap, RenderError> {
    let mut pm = Pixmap::new(size, size)
        .ok_or(RenderError { kind: RenderErrorKind::InvalidSize(size) })?;
    let pal = Palette::from_genes(g);
    let bg = pal.background;
    pm.fill(Color::from_rgba8(bg.r, bg.g, bg.b, 255));

    let s = f64::from(size);
    let (cx, cy) = (s / 2.0, s / 2.0);
    let k = SOLID_SCALE * s;

    let mut mesh = Mesh::from_genes(g);
    let tilt = match view {
        View::Solid3d => (TILT_MIN_DEG
            + f64::from(g.pitch) * TILT_RANGE_DEG / 7.0)
            .to_radians(),
        View::Flat => PI / 2.0,
    };
    mesh.tilt(tilt);
    let mesh = mesh;

    let project = |v: [f64; 3]| -> (f32, f32) {
        let p = match view {
            View::Solid3d => CAMERA_DIST / (CAMERA_DIST - v[2]),
            View::Flat => 1.0,
        };
        ((cx + v[0] * p * k) as f32, (cy - v[1] * p * k) as f32)
    };

    // Back-face culling, then painter's order (far to near).
    let mut visible: Vec<(f64, usize)> = mesh
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            let n = mesh.face_normal(f);
            match view {
                View::Solid3d => {
                    let v0 = mesh.verts[f.indices[0]];
                    dot(n, [-v0[0], -v0[1], CAMERA_DIST - v0[2]]) > 1e-9
                }
                View::Flat => n[2] > 1e-6,
            }
        })
        .map(|(i, f)| (centroid(&mesh.verts, &f.indices)[2], i))
        .collect();
    visible.sort_by(|a, b| a.0.total_cmp(&b.0));

    let light = normalize(LIGHT_DIR);
    let edge = edge_stroke(g.edge, size as f32);
    for (_, fi) in visible {
        let f = &mesh.faces[fi];
        let mut pb = PathBuilder::new();
        for (j, &vi) in f.indices.iter().enumerate() {
            let (x, y) = project(mesh.verts[vi]);
            if j == 0 { pb.move_to(x, y) } else { pb.line_to(x, y) }
        }
        pb.close();
        let path = pb
            .finish()
            .ok_or(RenderError { kind: RenderErrorKind::DegenerateFace })?;
        let base = role_color(&pal, f.role);
        let color = match view {
            View::Solid3d => {
                let lambert = dot(mesh.face_normal(f), light).max(0.0);
                base.scale(AMBIENT + DIFFUSE * lambert)
            }
            View::Flat => base,
        };
        pm.fill_path(
            &path,
            &paint(color),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
        if let Some((ec, stroke)) = &edge {
            pm.stroke_path(
                &path,
                &paint(*ec),
                stroke,
                Transform::identity(),
                None,
            );
        }
    }

    draw_ring(&mut pm, g, &pal, s);
    Ok(pm)
}

/// Encodes a pixmap as PNG.
pub(crate) fn encode_png(pm: &Pixmap) -> Result<Vec<u8>, RenderError> {
    pm.encode_png().map_err(|e| RenderError {
        kind: RenderErrorKind::Encode(Box::new(e)),
    })
}

fn arc(cx: f64, cy: f64, r: f64, a0: f64, a1: f64) -> Option<Path> {
    let steps =
        ((a1 - a0).abs() / TAU * ARC_STEPS_PER_TURN).ceil().max(4.0) as usize;
    let mut pb = PathBuilder::new();
    for i in 0..=steps {
        let a = a0 + (a1 - a0) * i as f64 / steps as f64;
        let (x, y) = ((cx + r * a.cos()) as f32, (cy + r * a.sin()) as f32);
        if i == 0 { pb.move_to(x, y) } else { pb.line_to(x, y) }
    }
    pb.finish()
}

/// The orbit ring: a thin track with lit segments for set bits, and an
/// anchor notch at 12 o'clock that fixes where reading starts.
fn draw_ring(pm: &mut Pixmap, g: &VisualGenes, pal: &Palette, s: f64) {
    let (cx, cy, r) = (s / 2.0, s / 2.0, RING_RADIUS * s);
    let track =
        Stroke { width: (RING_TRACK_WIDTH * s) as f32, ..Stroke::default() };
    if let Some(p) = arc(cx, cy, r, 0.0, TAU) {
        pm.stroke_path(
            &p,
            &paint(pal.accent2),
            &track,
            Transform::identity(),
            None,
        );
    }

    let seg = TAU / f64::from(RING_SEGMENTS);
    let gap = RING_GAP_DEG.to_radians();
    let lit = Stroke {
        width: (RING_LIT_WIDTH * s) as f32,
        line_cap: LineCap::Butt,
        ..Stroke::default()
    };
    let pattern = g.ring_pattern();
    for i in 0..RING_SEGMENTS {
        if pattern >> i & 1 == 1 {
            let a0 = -PI / 2.0 + f64::from(i) * seg + gap;
            if let Some(p) = arc(cx, cy, r, a0, a0 + seg - 2.0 * gap) {
                pm.stroke_path(
                    &p,
                    &paint(pal.accent1),
                    &lit,
                    Transform::identity(),
                    None,
                );
            }
        }
    }

    let w = (ANCHOR_HALF_WIDTH * s) as f32;
    let top = (cy - r - ANCHOR_OUTER * s) as f32;
    let mut pb = PathBuilder::new();
    pb.move_to(cx as f32 - w, top);
    pb.line_to(cx as f32 + w, top);
    pb.line_to(cx as f32, (cy - r + ANCHOR_INNER * s) as f32);
    pb.close();
    if let Some(p) = pb.finish() {
        pm.fill_path(
            &p,
            &paint(pal.base),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

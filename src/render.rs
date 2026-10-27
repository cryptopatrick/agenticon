//! Rasterisation of the 3D and flat views.

use std::f64::consts::{PI, TAU};

use tiny_skia::{
    FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform,
};

use crate::Error;
use crate::genome::{Background, EdgeStyle, RING_SEGMENTS, VisualGenes};
use crate::geometry::{Mesh, Role, centroid, dot, normalize};

/// An sRGB colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    fn scale(self, k: f64) -> Rgb {
        let f = |c: u8| (c as f64 * k).round().clamp(0.0, 255.0) as u8;
        Rgb(f(self.0), f(self.1), f(self.2))
    }
}

/// Converts HSL (hue in degrees, s and l in 0..1) to RGB.
pub fn hsl(h: f64, s: f64, l: f64) -> Rgb {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to8 = |v: f64| ((v + m) * 255.0).round() as u8;
    Rgb(to8(r), to8(g), to8(b))
}

/// Colours derived from the visual genes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// Base colour (caps, ring anchor).
    pub base: Rgb,
    /// First accent (side faces, lit ring segments).
    pub accent1: Rgb,
    /// Second accent (side faces, ring track).
    pub accent2: Rgb,
    /// Background colour.
    pub background: Rgb,
}

impl Palette {
    /// Builds the palette for the given genes.
    pub fn from_genes(g: &VisualGenes) -> Palette {
        let h = g.hue as f64 * 360.0 / 32.0;
        let (o1, o2) = g.scheme.offsets();
        let background = match g.background {
            Background::Dark => Rgb(18, 18, 24),
            Background::Light => Rgb(240, 240, 236),
            Background::DarkTint => hsl(h, 0.35, 0.12),
            Background::LightTint => hsl(h, 0.45, 0.92),
        };
        Palette {
            base: hsl(h, 0.65, 0.55),
            accent1: hsl(h + o1, 0.6, 0.62),
            accent2: hsl(h + o2, 0.55, 0.44),
            background,
        }
    }

    fn role(&self, r: Role) -> Rgb {
        match r {
            Role::Cap => self.base,
            Role::SideA => self.accent1,
            Role::SideB => self.accent2,
        }
    }
}

fn paint(c: Rgb) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(c.0, c.1, c.2, 255);
    p.anti_alias = true;
    p
}

fn edge_stroke(edge: EdgeStyle, size: f32) -> Option<(Rgb, Stroke)> {
    let (color, width) = match edge {
        EdgeStyle::None => return None,
        EdgeStyle::ThinDark => (Rgb(20, 20, 28), 0.006),
        EdgeStyle::ThinLight => (Rgb(245, 245, 245), 0.006),
        EdgeStyle::ThickDark => (Rgb(20, 20, 28), 0.014),
    };
    let stroke = Stroke {
        width: width * size,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    Some((color, stroke))
}

/// Which view to render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    /// Shaded perspective rendering of the 3D solid.
    Solid3d,
    /// The solid flattened along its axis: orthographic, unlit.
    Flat,
}

/// Renders a view into a square RGBA pixmap of `size` pixels.
pub(crate) fn render(
    g: &VisualGenes,
    view: View,
    size: u32,
) -> Result<Pixmap, Error> {
    let mut pm =
        Pixmap::new(size, size).ok_or(Error::Render("invalid size".into()))?;
    let pal = Palette::from_genes(g);
    let bg = pal.background;
    pm.fill(tiny_skia::Color::from_rgba8(bg.0, bg.1, bg.2, 255));

    let s = size as f64;
    let (cx, cy) = (s / 2.0, s / 2.0);
    let k = 0.27 * s;

    let mut mesh = Mesh::from_genes(g);
    let camera_dist = 4.0;
    let tilt = match view {
        View::Solid3d => (15.0 + g.pitch as f64 * 60.0 / 7.0).to_radians(),
        View::Flat => PI / 2.0,
    };
    mesh.tilt(tilt);

    let project = |v: [f64; 3]| -> (f32, f32) {
        let p = match view {
            View::Solid3d => camera_dist / (camera_dist - v[2]),
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
                    dot(n, [-v0[0], -v0[1], camera_dist - v0[2]]) > 1e-9
                }
                View::Flat => n[2] > 1e-6,
            }
        })
        .map(|(i, f)| (centroid(&mesh.verts, &f.indices)[2], i))
        .collect();
    visible.sort_by(|a, b| a.0.total_cmp(&b.0));

    let light = normalize([-0.45, 0.6, 0.65]);
    let edge = edge_stroke(g.edge, size as f32);
    for (_, fi) in visible {
        let f = &mesh.faces[fi];
        let mut pb = PathBuilder::new();
        for (j, &vi) in f.indices.iter().enumerate() {
            let (x, y) = project(mesh.verts[vi]);
            if j == 0 { pb.move_to(x, y) } else { pb.line_to(x, y) }
        }
        pb.close();
        let path = pb.finish().ok_or(Error::Render("empty face".into()))?;
        let base = pal.role(f.role);
        let color = match view {
            View::Solid3d => {
                let lambert = dot(mesh.face_normal(f), light).max(0.0);
                base.scale(0.4 + 0.75 * lambert)
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

fn arc(cx: f64, cy: f64, r: f64, a0: f64, a1: f64) -> Option<tiny_skia::Path> {
    let steps = ((a1 - a0).abs() / TAU * 180.0).ceil().max(4.0) as usize;
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
    let (cx, cy, r) = (s / 2.0, s / 2.0, 0.43 * s);
    let track = Stroke { width: (0.008 * s) as f32, ..Stroke::default() };
    if let Some(p) = arc(cx, cy, r, 0.0, TAU) {
        pm.stroke_path(
            &p,
            &paint(pal.accent2),
            &track,
            Transform::identity(),
            None,
        );
    }

    let seg = TAU / RING_SEGMENTS as f64;
    let gap = 1.5f64.to_radians();
    let lit = Stroke {
        width: (0.035 * s) as f32,
        line_cap: LineCap::Butt,
        ..Stroke::default()
    };
    let pattern = g.ring_pattern();
    for i in 0..RING_SEGMENTS {
        if pattern >> i & 1 == 1 {
            let a0 = -PI / 2.0 + i as f64 * seg + gap;
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

    let (w, top) = ((0.025 * s) as f32, (cy - r - 0.045 * s) as f32);
    let mut pb = PathBuilder::new();
    pb.move_to(cx as f32 - w, top);
    pb.line_to(cx as f32 + w, top);
    pb.line_to(cx as f32, (cy - r + 0.02 * s) as f32);
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

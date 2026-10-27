//! Construction of convex solids as polygon meshes.

use std::f64::consts::TAU;

use crate::genome::{Family, VisualGenes};

pub(crate) type V3 = [f64; 3];

/// Colour role of a face; resolved against the palette when rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    Cap,
    SideA,
    SideB,
}

pub(crate) struct Face {
    pub indices: Vec<usize>,
    pub role: Role,
}

pub(crate) struct Mesh {
    pub verts: Vec<V3>,
    pub faces: Vec<Face>,
}

pub(crate) fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(crate) fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(crate) fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(crate) fn normalize(a: V3) -> V3 {
    let l = dot(a, a).sqrt();
    [a[0] / l, a[1] / l, a[2] / l]
}

impl Mesh {
    /// Builds the solid described by the genes, centred on the origin with
    /// its axis along +y, scaled to fit the unit sphere, and rotated by yaw.
    pub fn from_genes(g: &VisualGenes) -> Mesh {
        let n = usize::from(g.sides);
        let h = [0.45, 0.7, 0.95, 1.2][usize::from(g.stretch)];
        let ring = |y: f64, r: f64, offset: f64| -> Vec<V3> {
            (0..n)
                .map(|k| {
                    let t = offset + TAU * k as f64 / n as f64;
                    [r * t.cos(), y, r * t.sin()]
                })
                .collect()
        };
        let side_role = |i: usize| {
            if i.is_multiple_of(2) { Role::SideA } else { Role::SideB }
        };

        let mut verts = Vec::new();
        let mut faces = Vec::new();
        match g.family {
            Family::Prism | Family::Frustum => {
                let top_r = if g.family == Family::Prism { 1.0 } else { 0.5 };
                verts.extend(ring(-h, 1.0, 0.0));
                verts.extend(ring(h, top_r, 0.0));
                faces
                    .push(Face { indices: (0..n).collect(), role: Role::Cap });
                faces.push(Face {
                    indices: (n..2 * n).collect(),
                    role: Role::Cap,
                });
                for i in 0..n {
                    let j = (i + 1) % n;
                    faces.push(Face {
                        indices: vec![i, j, n + j, n + i],
                        role: side_role(i),
                    });
                }
            }
            Family::Antiprism => {
                verts.extend(ring(-h, 1.0, 0.0));
                verts.extend(ring(h, 1.0, TAU / (2 * n) as f64));
                faces
                    .push(Face { indices: (0..n).collect(), role: Role::Cap });
                faces.push(Face {
                    indices: (n..2 * n).collect(),
                    role: Role::Cap,
                });
                for i in 0..n {
                    let j = (i + 1) % n;
                    faces.push(Face {
                        indices: vec![i, j, n + i],
                        role: Role::SideA,
                    });
                    faces.push(Face {
                        indices: vec![n + i, n + j, j],
                        role: Role::SideB,
                    });
                }
            }
            Family::Bipyramid => {
                verts.extend(ring(0.0, 1.0, 0.0));
                verts.push([0.0, h * 1.3, 0.0]);
                verts.push([0.0, -h * 1.3, 0.0]);
                for i in 0..n {
                    let j = (i + 1) % n;
                    faces.push(Face {
                        indices: vec![i, j, n],
                        role: side_role(i),
                    });
                    faces.push(Face {
                        indices: vec![i, j, n + 1],
                        role: side_role(i + 1),
                    });
                }
            }
        }

        // Fit to the unit sphere.
        let max_r =
            verts.iter().map(|v| dot(*v, *v).sqrt()).fold(0.0, f64::max);
        for v in &mut verts {
            *v = [v[0] / max_r, v[1] / max_r, v[2] / max_r];
        }

        // Yaw steps cover exactly one rotational symmetry period, so every
        // step gives a distinct orientation.
        let yaw = 0.15 + (f64::from(g.yaw) / 16.0) * TAU / n as f64;
        let (s, c) = yaw.sin_cos();
        for v in &mut verts {
            *v = [c * v[0] + s * v[2], v[1], -s * v[0] + c * v[2]];
        }

        let mut mesh = Mesh { verts, faces };
        mesh.orient_outward();
        mesh
    }

    /// Reverses faces whose winding points inward (the solid is convex and
    /// contains the origin).
    fn orient_outward(&mut self) {
        let verts = &self.verts;
        for f in &mut self.faces {
            let v = |i: usize| verts[f.indices[i]];
            let n = cross(sub(v(1), v(0)), sub(v(2), v(0)));
            let c = centroid(verts, &f.indices);
            if dot(n, c) < 0.0 {
                f.indices.reverse();
            }
        }
    }

    fn face_normal_raw(&self, idx: &[usize]) -> V3 {
        let v = |i: usize| self.verts[idx[i]];
        cross(sub(v(1), v(0)), sub(v(2), v(0)))
    }

    pub fn face_normal(&self, f: &Face) -> V3 {
        normalize(self.face_normal_raw(&f.indices))
    }

    /// Applies a rotation about the x axis (camera tilt).
    pub fn tilt(&mut self, angle: f64) {
        let (s, c) = angle.sin_cos();
        for v in &mut self.verts {
            *v = [v[0], c * v[1] - s * v[2], s * v[1] + c * v[2]];
        }
    }
}

pub(crate) fn centroid(verts: &[V3], idx: &[usize]) -> V3 {
    let mut c = [0.0; 3];
    for &i in idx {
        for k in 0..3 {
            c[k] += verts[i][k];
        }
    }
    let n = idx.len() as f64;
    [c[0] / n, c[1] / n, c[2] / n]
}

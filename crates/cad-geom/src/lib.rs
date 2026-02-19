//! Geometry layer backed by Truck.

use cad_core::{Model, ObjectId, Transform};
use glam::{Mat4, Quat, Vec3};
use thiserror::Error;
use truck_meshalgo::{filters::*, tessellation::*};
use truck_modeling::{builder, InnerSpace, Point3, Rad, Solid, Vector3};
use truck_polymesh::{PolygonMesh, StandardAttributes, StandardVertex, TOLERANCE};

#[derive(Debug, Error)]
pub enum GeomError {
    #[error("no solids in scene")]
    EmptyScene,
    #[error("operation not implemented: {0}")]
    NotImplemented(&'static str),
}

#[derive(Debug, Clone, Default)]
pub struct TriMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Debug, Clone, Copy)]
pub struct SurfaceHit {
    pub object_id: ObjectId,
    pub point: [f32; 3],
    pub normal: [f32; 3],
    pub distance: f32,
}

impl TriMesh {
    pub fn append(&mut self, other: TriMesh) {
        let base = self.positions.len() as u32;
        self.positions.extend(other.positions);
        self.normals.extend(other.normals);
        self.indices
            .extend(other.indices.into_iter().map(|idx| idx + base));
    }

    pub fn append_transformed(&mut self, other: &TriMesh, transform: Mat4) {
        let base = self.positions.len() as u32;
        self.positions.extend(other.positions.iter().map(|p| {
            let p = Vec3::from_array(*p);
            let p = transform.transform_point3(p);
            p.to_array()
        }));
        self.normals.extend(other.normals.iter().map(|n| {
            let n = Vec3::from_array(*n);
            let n = transform.transform_vector3(n);
            if n.length_squared() > 1.0e-12 {
                n.normalize().to_array()
            } else {
                [0.0, 1.0, 0.0]
            }
        }));
        self.indices
            .extend(other.indices.iter().copied().map(|idx| idx + base));
    }
}

/// Scene that keeps model data separate from render meshes.
#[derive(Default)]
pub struct GeomScene {
    model: Model,
    solids: Vec<Solid>,
    local_meshes: Vec<TriMesh>,
    bounds_radius: Vec<f32>,
    local_aabbs: Vec<Aabb>,
    mesh_cache: Option<TriMesh>,
    tolerance: f64,
}

impl GeomScene {
    pub fn new() -> Self {
        Self {
            model: Model::default(),
            solids: Vec::new(),
            local_meshes: Vec::new(),
            bounds_radius: Vec::new(),
            local_aabbs: Vec::new(),
            mesh_cache: None,
            tolerance: 0.01,
        }
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn object_transform(&self, id: ObjectId) -> Option<Transform> {
        self.model.object(id).map(|obj| obj.transform)
    }

    pub fn bounds_radius(&self, id: ObjectId) -> Option<f32> {
        self.model
            .objects()
            .iter()
            .position(|obj| obj.id == id)
            .and_then(|idx| self.bounds_radius.get(idx).copied())
    }

    pub fn local_aabb(&self, id: ObjectId) -> Option<Aabb> {
        self.model
            .objects()
            .iter()
            .position(|obj| obj.id == id)
            .and_then(|idx| self.local_aabbs.get(idx).copied())
    }

    pub fn set_object_transform(&mut self, id: ObjectId, transform: Transform) -> bool {
        if self.model.set_transform(id, transform) {
            self.mesh_cache = None;
            true
        } else {
            false
        }
    }

    pub fn add_box(&mut self, w: f32, h: f32, d: f32) -> ObjectId {
        let id = self.model.add_box(w, h, d);
        let solid = make_box(w as f64, h as f64, d as f64);
        let mesh = tessellate_solid(&solid, self.tolerance);
        let radius = mesh_bounds_radius(&mesh);
        let aabb = mesh_bounds_aabb(&mesh);
        self.solids.push(solid);
        self.local_meshes.push(mesh);
        self.bounds_radius.push(radius);
        self.local_aabbs.push(aabb);
        self.mesh_cache = None;
        id
    }

    pub fn add_cylinder(&mut self, r: f32, h: f32) -> ObjectId {
        let id = self.model.add_cylinder(r, h);
        let solid = make_cylinder(r as f64, h as f64);
        let mesh = tessellate_solid(&solid, self.tolerance);
        let radius = mesh_bounds_radius(&mesh);
        let aabb = mesh_bounds_aabb(&mesh);
        self.solids.push(solid);
        self.local_meshes.push(mesh);
        self.bounds_radius.push(radius);
        self.local_aabbs.push(aabb);
        self.mesh_cache = None;
        id
    }

    pub fn mesh(&mut self) -> Result<TriMesh, GeomError> {
        if self.solids.is_empty() {
            return Err(GeomError::EmptyScene);
        }
        if let Some(mesh) = self.mesh_cache.clone() {
            return Ok(mesh);
        }
        let mut combined = TriMesh::default();
        for (idx, obj) in self.model.objects().iter().enumerate() {
            if let Some(mesh) = self.local_meshes.get(idx) {
                let transform = transform_mat(obj.transform);
                combined.append_transformed(mesh, transform);
            }
        }
        self.mesh_cache = Some(combined.clone());
        Ok(combined)
    }

    pub fn pick_surface(&self, ray_origin: [f32; 3], ray_dir: [f32; 3]) -> Option<SurfaceHit> {
        let ray_o = Vec3::from_array(ray_origin);
        let ray_d = Vec3::from_array(ray_dir).normalize_or_zero();
        if ray_d.length_squared() < 1.0e-12 {
            return None;
        }

        let mut best: Option<SurfaceHit> = None;
        let mut best_t = f32::INFINITY;

        for (idx, obj) in self.model.objects().iter().enumerate() {
            let Some(mesh) = self.local_meshes.get(idx) else {
                continue;
            };
            let transform = transform_mat(obj.transform);
            let rotation = Quat::from_xyzw(
                obj.transform.rotation[0],
                obj.transform.rotation[1],
                obj.transform.rotation[2],
                obj.transform.rotation[3],
            )
            .normalize();

            for tri in mesh.indices.chunks_exact(3) {
                let i0 = tri[0] as usize;
                let i1 = tri[1] as usize;
                let i2 = tri[2] as usize;
                let (Some(p0), Some(p1), Some(p2)) = (
                    mesh.positions.get(i0),
                    mesh.positions.get(i1),
                    mesh.positions.get(i2),
                ) else {
                    continue;
                };

                let p0 = transform.transform_point3(Vec3::from_array(*p0));
                let p1 = transform.transform_point3(Vec3::from_array(*p1));
                let p2 = transform.transform_point3(Vec3::from_array(*p2));

                let Some(t) = ray_triangle_intersect(ray_o, ray_d, p0, p1, p2) else {
                    continue;
                };
                if t >= best_t {
                    continue;
                }

                let n = if let (Some(n0), Some(n1), Some(n2)) = (
                    mesh.normals.get(i0),
                    mesh.normals.get(i1),
                    mesh.normals.get(i2),
                ) {
                    let n_local =
                        (Vec3::from_array(*n0) + Vec3::from_array(*n1) + Vec3::from_array(*n2))
                            / 3.0;
                    (rotation * n_local).normalize_or_zero()
                } else {
                    (p1 - p0).cross(p2 - p0).normalize_or_zero()
                };

                let hit_point = ray_o + ray_d * t;
                best_t = t;
                best = Some(SurfaceHit {
                    object_id: obj.id,
                    point: hit_point.to_array(),
                    normal: n.to_array(),
                    distance: t,
                });
            }
        }

        best
    }
}

pub fn make_box(w: f64, h: f64, d: f64) -> Solid {
    let v = builder::vertex(Point3::new(-w / 2.0, -h / 2.0, -d / 2.0));
    let e = builder::tsweep(&v, Vector3::unit_x() * w);
    let f = builder::tsweep(&e, Vector3::unit_y() * h);
    builder::tsweep(&f, Vector3::unit_z() * d)
}

pub fn make_cylinder(r: f64, h: f64) -> Solid {
    let vertex = builder::vertex(Point3::new(0.0, -h / 2.0, r));
    let circle = builder::rsweep(
        &vertex,
        Point3::new(0.0, 0.0, 0.0),
        Vector3::unit_y(),
        Rad(std::f64::consts::TAU),
    );
    let disk = builder::try_attach_plane(&[circle]).expect("attach disk");
    builder::tsweep(&disk, Vector3::new(0.0, h, 0.0))
}

pub fn tessellate_solid(solid: &Solid, tolerance: f64) -> TriMesh {
    let mut poly = solid.triangulation(tolerance).to_polygon();
    poly.put_together_same_attrs(TOLERANCE * 10.0)
        .remove_degenerate_faces()
        .remove_unused_attrs();
    polygon_to_trimesh(&poly)
}

/// TODO: boolean subtraction backend (A - B).
pub fn boolean_subtract(_a: &Solid, _b: &Solid) -> Result<Solid, GeomError> {
    Err(GeomError::NotImplemented("boolean_subtract"))
}

/// TODO: STEP export backend.
pub fn export_step(_solid: &Solid) -> Result<String, GeomError> {
    Err(GeomError::NotImplemented("export_step"))
}

fn polygon_to_trimesh(poly: &PolygonMesh<StandardVertex, StandardAttributes>) -> TriMesh {
    let attrs = poly.attributes();
    let mut mesh = TriMesh::default();
    let mut index = 0u32;

    for tri in poly.faces().triangle_iter() {
        let p0 = attrs.positions[tri[0].pos];
        let p1 = attrs.positions[tri[1].pos];
        let p2 = attrs.positions[tri[2].pos];
        let fallback = face_normal(p0, p1, p2);

        for v in tri {
            let p = attrs.positions[v.pos];
            let n = v
                .nor
                .and_then(|idx| attrs.normals.get(idx))
                .map(vector_to_array)
                .unwrap_or(fallback);
            mesh.positions.push(point_to_array(p));
            mesh.normals.push(n);
            mesh.indices.push(index);
            index += 1;
        }
    }

    mesh
}

fn point_to_array(p: Point3) -> [f32; 3] {
    [p.x as f32, p.y as f32, p.z as f32]
}

fn vector_to_array(v: &Vector3) -> [f32; 3] {
    [v.x as f32, v.y as f32, v.z as f32]
}

fn face_normal(p0: Point3, p1: Point3, p2: Point3) -> [f32; 3] {
    let u = p1 - p0;
    let v = p2 - p0;
    let n = u.cross(v);
    if n.magnitude2() > 1.0e-12 {
        let n = n.normalize();
        [n.x as f32, n.y as f32, n.z as f32]
    } else {
        [0.0, 1.0, 0.0]
    }
}

fn mesh_bounds_radius(mesh: &TriMesh) -> f32 {
    mesh.positions
        .iter()
        .map(|p| Vec3::from_array(*p).length())
        .fold(0.0, f32::max)
}

fn transform_mat(transform: Transform) -> Mat4 {
    let t = Vec3::from_array(transform.translation);
    let q = Quat::from_xyzw(
        transform.rotation[0],
        transform.rotation[1],
        transform.rotation[2],
        transform.rotation[3],
    )
    .normalize();
    Mat4::from_translation(t) * Mat4::from_quat(q)
}

fn mesh_bounds_aabb(mesh: &TriMesh) -> Aabb {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for p in &mesh.positions {
        let v = Vec3::from_array(*p);
        min = min.min(v);
        max = max.max(v);
    }
    if !min.is_finite() || !max.is_finite() {
        return Aabb::default();
    }
    Aabb {
        min: min.to_array(),
        max: max.to_array(),
    }
}

fn ray_triangle_intersect(ray_o: Vec3, ray_d: Vec3, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
    let eps = 1.0e-6;
    let e1 = v1 - v0;
    let e2 = v2 - v0;
    let pvec = ray_d.cross(e2);
    let det = e1.dot(pvec);
    if det.abs() < eps {
        return None;
    }
    let inv_det = 1.0 / det;
    let tvec = ray_o - v0;
    let u = tvec.dot(pvec) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let qvec = tvec.cross(e1);
    let v = ray_d.dot(qvec) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(qvec) * inv_det;
    if t > eps {
        Some(t)
    } else {
        None
    }
}

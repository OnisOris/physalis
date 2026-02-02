//! Geometry layer backed by Truck.

use cad_core::Model;
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

impl TriMesh {
    pub fn append(&mut self, other: TriMesh) {
        let base = self.positions.len() as u32;
        self.positions.extend(other.positions);
        self.normals.extend(other.normals);
        self.indices
            .extend(other.indices.into_iter().map(|idx| idx + base));
    }
}

/// Scene that keeps model data separate from render meshes.
#[derive(Default)]
pub struct GeomScene {
    model: Model,
    solids: Vec<Solid>,
    mesh_cache: Option<TriMesh>,
    tolerance: f64,
}

impl GeomScene {
    pub fn new() -> Self {
        Self {
            model: Model::default(),
            solids: Vec::new(),
            mesh_cache: None,
            tolerance: 0.01,
        }
    }

    pub fn model(&self) -> &Model {
        &self.model
    }

    pub fn add_box(&mut self, w: f32, h: f32, d: f32) {
        self.model.add_box(w, h, d);
        self.solids.push(make_box(w as f64, h as f64, d as f64));
        self.mesh_cache = None;
    }

    pub fn add_cylinder(&mut self, r: f32, h: f32) {
        self.model.add_cylinder(r, h);
        self.solids
            .push(make_cylinder(r as f64, h as f64));
        self.mesh_cache = None;
    }

    pub fn mesh(&mut self) -> Result<TriMesh, GeomError> {
        if self.solids.is_empty() {
            return Err(GeomError::EmptyScene);
        }
        if let Some(mesh) = self.mesh_cache.clone() {
            return Ok(mesh);
        }
        let mut combined = TriMesh::default();
        for solid in &self.solids {
            let mesh = tessellate_solid(solid, self.tolerance);
            combined.append(mesh);
        }
        self.mesh_cache = Some(combined.clone());
        Ok(combined)
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

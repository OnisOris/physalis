//! Core model types shared by client and server.

use serde::{Deserialize, Serialize};

pub type ObjectId = u64;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Transform {
    pub translation: [f32; 3],
    /// Quaternion `[x, y, z, w]`.
    pub rotation: [f32; 4],
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectKind {
    Box { w: f32, h: f32, d: f32 },
    Cylinder { r: f32, h: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelObject {
    pub id: ObjectId,
    pub kind: ObjectKind,
    pub transform: Transform,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Model {
    objects: Vec<ModelObject>,
    next_id: ObjectId,
}

impl Model {
    pub fn objects(&self) -> &[ModelObject] {
        &self.objects
    }

    pub fn object(&self, id: ObjectId) -> Option<&ModelObject> {
        self.objects.iter().find(|obj| obj.id == id)
    }

    pub fn set_transform(&mut self, id: ObjectId, transform: Transform) -> bool {
        if let Some(obj) = self.objects.iter_mut().find(|obj| obj.id == id) {
            obj.transform = transform;
            true
        } else {
            false
        }
    }

    pub fn add_box(&mut self, w: f32, h: f32, d: f32) -> ObjectId {
        self.add_object(ObjectKind::Box { w, h, d })
    }

    pub fn add_cylinder(&mut self, r: f32, h: f32) -> ObjectId {
        self.add_object(ObjectKind::Cylinder { r, h })
    }

    fn add_object(&mut self, kind: ObjectKind) -> ObjectId {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.objects.push(ModelObject {
            id,
            kind,
            transform: Transform::default(),
        });
        id
    }
}

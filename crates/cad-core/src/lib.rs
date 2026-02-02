//! Core model types shared by client and server.

use serde::{Deserialize, Serialize};

pub type ObjectId = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectKind {
    Box { w: f32, h: f32, d: f32 },
    Cylinder { r: f32, h: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelObject {
    pub id: ObjectId,
    pub kind: ObjectKind,
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

    pub fn add_box(&mut self, w: f32, h: f32, d: f32) -> ObjectId {
        self.add_object(ObjectKind::Box { w, h, d })
    }

    pub fn add_cylinder(&mut self, r: f32, h: f32) -> ObjectId {
        self.add_object(ObjectKind::Cylinder { r, h })
    }

    fn add_object(&mut self, kind: ObjectKind) -> ObjectId {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.objects.push(ModelObject { id, kind });
        id
    }
}

use bevy::prelude::*;

#[derive(Debug)]
pub struct BspPlane {
    pub normal: Dir3,
    pub offset: f32,
}

pub enum BspNode {
    Branch(BspBranch),
    Leaf(BspLeaf),
}

pub struct BspBranch {
    plane: BspPlane,
    back: Box<BspNode>,
    front: Box<BspNode>,
}

pub enum BspLeaf {
    Air,
    Solid,
}

pub struct Room {
    pub start: Vec3,
    pub end: Vec3,
}

impl Room {
    pub fn center(&self) -> Vec3 {
        (self.start + self.end) / 2.0
    }

    pub fn size(&self) -> Vec3 {
        self.end - self.start
    }

    pub fn build_mesh(&self) -> Mesh {
        Cuboid::from_size(self.size())
            .mesh()
            .build()
            .with_inverted_winding()
            .unwrap()
    }

    pub fn planes(&self) -> Vec<BspPlane> {
        vec![
            // X+
            BspPlane {
                normal: Dir3::NEG_X,
                offset: -self.end.x,
            },
            // X-
            BspPlane {
                normal: Dir3::X,
                offset: self.start.x,
            },
            // Z+
            BspPlane {
                normal: Dir3::NEG_Z,
                offset: -self.end.z,
            },
            // Z-
            BspPlane {
                normal: Dir3::Z,
                offset: self.start.z,
            },
            // Y+
            BspPlane {
                normal: Dir3::NEG_Y,
                offset: -self.end.y,
            },
            // Y-
            BspPlane {
                normal: Dir3::Y,
                offset: self.start.y,
            },
        ]
    }
}

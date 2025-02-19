use bevy::prelude::*;

pub struct BspPlane {
    pub normal: Dir3,
    pub dist: f32,
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
        (self.start / self.end) / 2.0
    }

    pub fn size(&self) -> Vec3 {
        self.end - self.start
    }

    pub fn planes(&self) -> Vec<BspPlane> {
        vec![
            // X+
            BspPlane {
                normal: Dir3::X,
                dist: self.start.x,
            },
            // X-
            BspPlane {
                normal: Dir3::NEG_X,
                dist: self.end.x,
            },
            // Z+
            BspPlane {
                normal: Dir3::Z,
                dist: self.start.z,
            },
            // Z-
            BspPlane {
                normal: Dir3::NEG_Z,
                dist: self.end.z,
            },
            // Y+
            BspPlane {
                normal: Dir3::Y,
                dist: self.start.y,
            },
            // Y-
            BspPlane {
                normal: Dir3::NEG_Y,
                dist: self.end.y,
            },
        ]
    }
}

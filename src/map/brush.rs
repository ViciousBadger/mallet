use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Brush {
    pub bounds: BrushBounds,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct BrushBounds {
    pub start: Vec3,
    pub end: Vec3,
}

impl BrushBounds {
    pub fn new(point_a: Vec3, point_b: Vec3) -> Self {
        Self {
            start: Vec3::min(point_a, point_b),
            end: Vec3::max(point_a, point_b),
        }
    }

    pub fn is_valid(&self) -> bool {
        let size = self.size();
        size.x > 0.01 && size.y > 0.01 && size.z > 0.01
    }

    pub fn center(&self) -> Vec3 {
        (self.start + self.end) / 2.0
    }

    pub fn size(&self) -> Vec3 {
        self.end - self.start
    }

    pub fn sides(&self) -> Vec<BrushSide> {
        let center = self.center();
        let size = self.size();
        let half_size = size / 2.0;
        vec![
            // X-
            BrushSide {
                pos: Vec3::new(self.start.x, center.y, center.z),
                plane: Plane3d::new(Vec3::NEG_X, Vec2::new(half_size.y, half_size.z)),
            },
            // X+
            BrushSide {
                pos: Vec3::new(self.end.x, center.y, center.z),
                plane: Plane3d::new(Vec3::X, Vec2::new(half_size.y, half_size.z)),
            },
            // Z-
            BrushSide {
                pos: Vec3::new(center.x, center.y, self.start.z),
                plane: Plane3d::new(Vec3::NEG_Z, Vec2::new(half_size.x, half_size.y)),
            },
            // Z+
            BrushSide {
                pos: Vec3::new(center.x, center.y, self.end.z),
                plane: Plane3d::new(Vec3::Z, Vec2::new(half_size.x, half_size.y)),
            },
            // Y-
            BrushSide {
                pos: Vec3::new(center.x, self.start.y, center.z),
                plane: Plane3d::new(Vec3::NEG_Y, Vec2::new(half_size.x, half_size.z)),
            },
            // Y+
            BrushSide {
                pos: Vec3::new(center.x, self.end.y, center.z),
                plane: Plane3d::new(Vec3::Y, Vec2::new(half_size.x, half_size.z)),
            },
        ]
    }
}

pub struct BrushSide {
    pub pos: Vec3,
    pub plane: Plane3d,
}

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::util::Facing3d;

#[derive(Component, Serialize, Deserialize, Clone, PartialEq)]
#[require(Visibility, Transform)]
pub struct Brush {
    pub bounds: BrushBounds,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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

    pub fn sides_local(&self) -> impl Iterator<Item = BrushSide> {
        // TODO: might as well pre-calculate sides on new()
        // and store them so they don't have to be created on every call.
        let half_size = self.size() / 2.0;
        vec![
            // X-
            BrushSide {
                facing: Facing3d::NegX,
                pos: Vec3::NEG_X * half_size.x,
                plane: Plane3d::new(Vec3::NEG_X, Vec2::new(half_size.y, half_size.z)),
            },
            // X+
            BrushSide {
                facing: Facing3d::X,
                pos: Vec3::X * half_size.x,
                plane: Plane3d::new(Vec3::X, Vec2::new(half_size.y, half_size.z)),
            },
            // Z-
            BrushSide {
                facing: Facing3d::NegZ,
                pos: Vec3::NEG_Z * half_size.z,
                plane: Plane3d::new(Vec3::NEG_Z, Vec2::new(half_size.x, half_size.y)),
            },
            // Z+
            BrushSide {
                facing: Facing3d::Z,
                pos: Vec3::Z * half_size.z,
                plane: Plane3d::new(Vec3::Z, Vec2::new(half_size.x, half_size.y)),
            },
            // Y-
            BrushSide {
                facing: Facing3d::NegY,
                pos: Vec3::NEG_Y * half_size.y,
                plane: Plane3d::new(Vec3::NEG_Y, Vec2::new(half_size.x, half_size.z)),
            },
            // Y+
            BrushSide {
                facing: Facing3d::Y,
                pos: Vec3::Y * half_size.y,
                plane: Plane3d::new(Vec3::Y, Vec2::new(half_size.x, half_size.z)),
            },
        ]
        .into_iter()
    }

    pub fn sides_world(&self) -> impl Iterator<Item = BrushSide> {
        let offset = self.center();
        self.sides_local().map(move |mut side| {
            side.pos += offset;
            side
        })
    }

    pub fn resized(&self, side: Facing3d, target_point: Vec3) -> Self {
        let (start, end) = match side {
            Facing3d::NegX => (self.start.with_x(target_point.x), self.end),
            Facing3d::NegY => (self.start.with_y(target_point.y), self.end),
            Facing3d::NegZ => (self.start.with_z(target_point.z), self.end),
            Facing3d::X => (self.start, self.end.with_x(target_point.x)),
            Facing3d::Y => (self.start, self.end.with_y(target_point.y)),
            Facing3d::Z => (self.start, self.end.with_z(target_point.z)),
        };
        // Run thru constructor to ensure the brush don't flip inside out.
        Self::new(start, end)
    }
}

pub struct BrushSide {
    pub facing: Facing3d,
    pub pos: Vec3,
    pub plane: Plane3d,
}

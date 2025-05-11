use avian3d::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{
        Indices,
        PrimitiveTopology::{self},
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    core::map::{
        changes::{Change, UpdateElemParams},
        ElementLookup, MapAssets,
    },
    util::Facing3d,
};

#[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
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
        let size = self.size();
        let half_size = size / 2.0;
        vec![
            // X-
            BrushSide {
                facing: Facing3d::NegX,
                pos: Vec3::NEG_X * half_size.x,
                size: Vec2::new(size.z, size.y),
            },
            // X+
            BrushSide {
                facing: Facing3d::X,
                pos: Vec3::X * half_size.x,
                size: Vec2::new(size.z, size.y),
            },
            // Z-
            BrushSide {
                facing: Facing3d::NegZ,
                pos: Vec3::NEG_Z * half_size.z,
                size: Vec2::new(size.x, size.y),
            },
            // Z+
            BrushSide {
                facing: Facing3d::Z,
                pos: Vec3::Z * half_size.z,
                size: Vec2::new(size.x, size.y),
            },
            // Y-
            BrushSide {
                facing: Facing3d::NegY,
                pos: Vec3::NEG_Y * half_size.y,
                size: Vec2::new(size.x, size.z),
            },
            // Y+
            BrushSide {
                facing: Facing3d::Y,
                pos: Vec3::Y * half_size.y,
                size: Vec2::new(size.x, size.z),
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

#[derive(Clone)]
pub struct BrushSide {
    pub facing: Facing3d,
    pub pos: Vec3,
    pub size: Vec2,
}

impl Meshable for BrushSide {
    type Output = BrushSideMeshBuilder;

    fn mesh(&self) -> Self::Output {
        BrushSideMeshBuilder(self.clone())
    }
}

pub struct BrushSideMeshBuilder(BrushSide);

impl MeshBuilder for BrushSideMeshBuilder {
    fn build(&self) -> Mesh {
        const VERTS: usize = 4;
        let mut positions: Vec<Vec3> = Vec::with_capacity(VERTS);
        let mut normals: Vec<[f32; 3]> = Vec::with_capacity(VERTS);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(VERTS);

        let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, *self.0.facing.as_dir());
        let size = self.0.size;

        // Vertices (and vertex data)
        for z in 0..=1 {
            for x in 0..=1 {
                let tx = x as f32;
                let tz = z as f32;
                let pos = rotation * Vec3::new((-0.5 + tx) * size.x, (-0.5 + tz) * size.y, 0.0);
                positions.push(pos);
                normals.push(self.0.facing.as_dir().to_array());
                uvs.push([tx * size.x * 0.1, tz * size.y * 0.1]);
            }
        }

        let indices = vec![3, 1, 2, 0, 2, 1];

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_indices(Indices::U32(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Change for UpdateElemParams<Brush> {
    fn apply_to_world(&self, world: &mut World) {
        world
            .run_system_cached_with(
                |change: In<Self>,
                 lookup: Res<ElementLookup>,
                 map_assets: Res<MapAssets>,
                 mut meshes: ResMut<Assets<Mesh>>,
                 mut commands: Commands|
                 -> Result {
                    let entity_id = lookup.find(&change.elem_id)?;
                    let mut entity = commands.entity(entity_id);
                    //let mut entity = get_elem_entity(world, &self.elem_id).unwrap();
                    let brush = change.new_params.clone();

                    info!("spawning/updating brush: {:?}", brush);

                    // Brush will use base entity as a container for sides.
                    let center = brush.bounds.center();
                    let size = brush.bounds.size();

                    entity.insert((
                        brush.clone(),
                        Transform::IDENTITY.with_translation(center),
                        RigidBody::Static,
                        Collider::cuboid(size.x, size.y, size.z),
                    ));
                    entity.despawn_related::<Children>();
                    entity.with_children(|cmds| {
                        for side in brush.bounds.sides_local() {
                            let mesh = meshes.add(side.mesh());
                            let material = map_assets.default_material.clone();
                            cmds.spawn((
                                Transform::IDENTITY.with_translation(side.pos),
                                Mesh3d(mesh),
                                MeshMaterial3d(material),
                            ));
                        }
                    });
                    Ok(())
                },
                self.clone(),
            )
            .expect("error running system")
            .expect("system returned an error");
    }
}

use bevy::prelude::*;

#[derive(Component)]
pub enum MapElement {
    Brush {
        start: IVec3,
        end: IVec3,
    },
    //LevelGeometry(LevelGeometry),
    Prop {
        transform: Transform,
        features: Vec<PropFeature>,
    },
}

pub enum PropFeature {
    PointLightSource,
}

// pub fn deploy_all_elements(q_elements: Query<(Entity, &MapElement)>, mut commands: Commands) {
//     for (entity_id, element) in q_elements.iter() {
//         commands.entity(entity_id).despawn_recursive();
//
//         let element_clone = (*element).clone();
//         let fresh_entity = commands.spawn(element_clone);
//         element_clone.deploy(fresh_entity);
//     }
// }

pub fn deploy_added_elements(
    q_added_elements: Query<(Entity, &MapElement), Added<MapElement>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity_id, element) in q_added_elements.iter() {
        match element {
            MapElement::Brush { start: _, end: _ } => {
                commands.entity(entity_id).insert(PbrBundle {
                    mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
                    material: materials.add(Color::rgb_u8(124, 144, 255)),
                    transform: Transform::from_xyz(0.0, 0.5, 0.0),
                    ..default()
                });
            }
            MapElement::Prop {
                transform,
                features,
            } => {
                for feature in features {
                    match feature {
                        PropFeature::PointLightSource => {
                            commands.entity(entity_id).insert(PointLightBundle {
                                point_light: PointLight {
                                    shadows_enabled: true,
                                    ..Default::default()
                                },
                                ..default()
                            });
                        }
                    }
                }

                // Insert transform last
                // (Some bundles add their own transform components but this will be overwritten here)
                commands
                    .entity(entity_id)
                    .insert(TransformBundle::from_transform(*transform));
            }
        };
    }
}

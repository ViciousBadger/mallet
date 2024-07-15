use bevy::prelude::*;

#[derive(Component)]
pub enum MapElement {
    Brush(Brush),
    // StaticModel(StaticModel),
    // Entity(Entity),
}

#[derive(Default)]
pub struct Brush {
    pub start: IVec2,
    pub end: IVec2,
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
        commands.entity(entity_id).insert(match element {
            MapElement::Brush(_) => PbrBundle {
                mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
                material: materials.add(Color::rgb_u8(124, 144, 255)),
                transform: Transform::from_xyz(0.0, 0.5, 0.0),
                ..default()
            },
        });
    }
}

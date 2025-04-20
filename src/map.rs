pub mod brush;

use bevy::{
    color::palettes::css, input::common_conditions::input_just_released, prelude::*, utils::HashMap,
};
use bevy_common_assets::ron::RonAssetPlugin;
use brush::Brush;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::File,
    hash::{Hash, Hasher},
    io::Write,
};
use ulid::Ulid;

use crate::util::IdGen;

#[derive(Serialize, Deserialize, Default, Resource, Asset, TypePath)]
pub struct Map {
    pub nodes: BTreeMap<Ulid, MapNode>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Component)]
pub struct MapNode {
    pub id: Ulid,
    pub name: String,
    pub kind: MapNodeKind,
}

#[derive(Resource, Default)]
pub struct MapNodeLookupTable(HashMap<Ulid, Entity>);

impl Hash for MapNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl MapNode {
    pub fn new(id_gen: &mut IdGen, kind: MapNodeKind) -> Self {
        let id = id_gen.generate();
        let name = format!("{}-{}", id, kind.name());
        Self { id, name, kind }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum MapNodeKind {
    Brush(Brush),
}

impl MapNodeKind {
    pub fn name(&self) -> &'static str {
        match self {
            MapNodeKind::Brush(_) => "Brush",
        }
    }
}

#[derive(Event)]
pub struct CreateNewMapNode(pub MapNodeKind);

#[derive(Event)]
pub struct DeployMapNode(Entity);

pub fn plugin(app: &mut App) {
    app.add_plugins(RonAssetPlugin::<Map>::new(&["mmap"]))
        .add_systems(Startup, start_loading_map)
        .add_systems(
            First,
            (
                init_map.run_if(resource_added::<Map>),
                cleanup_map.run_if(resource_removed::<Map>),
                (reflect_map_node_changes, despawn_deleted_nodes)
                    .run_if(resource_exists_and_changed::<Map>),
                reset_map.run_if(input_just_released(KeyCode::KeyR)),
            ),
        )
        .add_systems(PreUpdate, perform_node_deployment)
        .add_systems(PostUpdate, save_map.run_if(on_event::<AppExit>))
        .add_systems(
            Last,
            (
                create_new_map_nodes,
                wait_for_map_load.run_if(resource_exists::<LoadingMap>),
            ),
        )
        .add_event::<CreateNewMapNode>()
        .add_event::<DeployMapNode>()
        .init_resource::<Map>()
        .init_resource::<MapNodeLookupTable>();
}

#[derive(Resource)]
pub struct LoadingMap(Handle<Map>);

fn start_loading_map(asset_server: Res<AssetServer>, mut commands: Commands) {
    let handle = asset_server.load("map.mmap");
    commands.insert_resource(LoadingMap(handle));
}

fn wait_for_map_load(
    loading_map: Res<LoadingMap>,
    asset_server: Res<AssetServer>,
    mut map_assets: ResMut<Assets<Map>>,
    mut commands: Commands,
) {
    match asset_server.load_state(&loading_map.0) {
        bevy::asset::LoadState::Loaded => {
            commands.insert_resource(map_assets.remove(&loading_map.0).unwrap());
            commands.remove_resource::<LoadingMap>();
            info!("load map success");
        }
        bevy::asset::LoadState::Failed(asset_load_error) => {
            commands.remove_resource::<LoadingMap>();
            info!("failed to load map: {}", asset_load_error);
        }
        _ => {}
    };
}

fn save_map(map: Res<Map>) {
    let mut file = File::create("assets/map.mmap").unwrap();
    file.write_all(ron::ser::to_string(&*map).unwrap().as_bytes())
        .unwrap();
}

fn reset_map(mut commands: Commands) {
    commands.remove_resource::<Map>();
    commands.init_resource::<Map>();
}

fn init_map(_map: Res<Map>) {
    // things such as skybox color..?
    info!("a map was added (init)");
}

fn cleanup_map() {
    info!("a map was removed and cleaned");
}

fn create_new_map_nodes(
    mut id_gen: ResMut<IdGen>,
    mut create_node_events: EventReader<CreateNewMapNode>,
    mut map: ResMut<Map>,
) {
    for event in create_node_events.read() {
        info!("creating map node");
        let new_node = MapNode::new(&mut id_gen, event.0.clone());
        map.nodes.insert(new_node.id.clone(), new_node);
    }
}

fn reflect_map_node_changes(
    map: Res<Map>,
    mut map_node_lookup: ResMut<MapNodeLookupTable>,
    mut q_live_nodes: Query<&mut MapNode>,
    mut commands: Commands,
    mut deploy_events: EventWriter<DeployMapNode>,
) {
    info!("a map was added or changed");
    for node in map.nodes.values() {
        if let Some(entity) = map_node_lookup.0.get(&node.id) {
            let mut live_node = q_live_nodes.get_mut(*entity).unwrap();
            if &*live_node != node {
                info!("node changed, to be redeployed: {}", node.id);
                *live_node = node.clone();
                deploy_events.send(DeployMapNode(*entity));
            }
        } else {
            let node_entity = commands.spawn(node.clone()).id();
            map_node_lookup.0.insert(node.id.clone(), node_entity);
            deploy_events.send(DeployMapNode(node_entity));
            info!("new node added, to be deployed: {}", node.id);
        }
    }
}

fn perform_node_deployment(
    q_live_nodes: Query<&MapNode>,
    mut deploy_events: EventReader<DeployMapNode>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for event in deploy_events.read() {
        let node_entity_id = event.0;

        let live_node = q_live_nodes.get(node_entity_id).unwrap();
        info!("deployment of {}", live_node.id);

        let mut entity_commands = commands.entity(node_entity_id);
        entity_commands.despawn_descendants();

        match &live_node.kind {
            MapNodeKind::Brush(ref brush) => {
                // Brush will use base entity as a container for sides.
                entity_commands.insert((Transform::IDENTITY, Visibility::Visible));
                for side in brush.bounds.sides() {
                    commands
                        .spawn((
                            Transform::IDENTITY.with_translation(side.pos),
                            Mesh3d(meshes.add(side.plane.mesh())),
                            MeshMaterial3d(materials.add(Color::Srgba(css::PERU))),
                        ))
                        .set_parent(node_entity_id);
                }
            }
        }
    }
}

fn despawn_deleted_nodes(
    map: Res<Map>,
    mut map_node_lookup: ResMut<MapNodeLookupTable>,
    q_live_nodes: Query<(Entity, &MapNode)>,
    mut commands: Commands,
) {
    for (entity, live_node) in q_live_nodes.iter() {
        if !map.nodes.contains_key(&live_node.id) {
            commands.entity(entity).despawn_recursive();
            map_node_lookup.0.remove(&live_node.id);
        }
    }
}

pub mod brush;
pub mod light;

use crate::{app_data::AppDataPath, core::binds::InputBindingSystem, util::IdGen};
use avian3d::{
    math::Quaternion,
    parry::either::IntoEither,
    prelude::{Collider, RigidBody},
};
use bevy::{
    input::common_conditions::{input_just_pressed, input_just_released},
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use bimap::BiHashMap;
use brush::Brush;
use daggy::{Dag, NodeIndex, Walker};
use light::Light;
use rand::{RngCore, SeedableRng};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::File, path::PathBuf};
use ulid::{serde::ulid_as_u128, Ulid};
use wyrand::WyRand;

use super::{
    binds::Binding,
    view::{Gimbal, GimbalPos, TPCameraTo},
};

#[derive(Resource, Serialize, Deserialize, Clone)]
pub struct Map {
    state: BTreeMap<MapNodeId, MapNode>,
    cur_delta_idx: NodeIndex<u32>,
    delta_graph: Dag<MapDelta, ()>,
    pub editor_context: EditorContext,
}

/// Special persistent identifier for map nodes.
#[derive(
    Deref,
    Debug,
    PartialOrd,
    Ord,
    PartialEq,
    Eq,
    Hash,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Component,
)]
pub struct MapNodeId(#[serde(with = "ulid_as_u128")] Ulid);

impl std::fmt::Display for MapNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Component)]
pub enum MapNode {
    Brush(Brush),
    Light(Light),
}

/// Represents a proposed change to the map.
#[derive(Event)]
pub enum MapChange {
    Add(MapNode),
    Modify(MapNodeId, MapNode),
    Remove(MapNodeId),
}

/// Request to deploy a map node in the world. Entity id is expected to be an entity with a MapNodeId component.
/// MapNode can be different from the one stored in the Map, to make temporary node changes in the editor.
#[derive(Event)]
pub struct DeployMapNode {
    pub entity_id: Entity,
    pub node: MapNode,
}

/// A "symmetric" map change that stores enough data to be reversable.
#[derive(Serialize, Deserialize, Clone)]
pub enum MapDelta {
    Nop,
    AddNode {
        id: MapNodeId,
        node: MapNode,
    },
    ModifyNode {
        id: MapNodeId,
        before: MapNode,
        after: MapNode,
    },
    RemoveNode {
        id: MapNodeId,
        node: MapNode,
    },
}

/// Things that are part of map but not saved.
#[derive(Resource)]
pub struct MapContext {
    pub save_path: PathBuf, // Could be in EditorContext but shouldnt be saved.. hmm..
    pub node_lookup: BiHashMap<MapNodeId, Entity>,
}

#[derive(Resource)]
pub struct MapAssets {
    pub default_material: Handle<StandardMaterial>,
}

/// Relevant editor state stored in the map file.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct EditorContext {
    pub camera_pos: GimbalPos,
}

#[derive(Debug)]
enum MapDeltaApplyError {
    NodeExists,
    NodeNotFound,
}

impl Default for Map {
    fn default() -> Self {
        let mut graph: Dag<MapDelta, ()> = Dag::new();
        let root_node = graph.add_node(MapDelta::Nop);
        Self {
            state: BTreeMap::new(),
            cur_delta_idx: root_node,
            delta_graph: graph,
            editor_context: EditorContext::default(),
        }
    }
}

impl Map {
    pub fn nodes(&self) -> impl Iterator<Item = &MapNode> {
        self.state.values()
    }

    pub fn node_ids(&self) -> impl Iterator<Item = &MapNodeId> {
        self.state.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&MapNodeId, &MapNode)> {
        self.state.iter()
    }

    pub fn get_node(&self, id: &MapNodeId) -> Option<&MapNode> {
        self.state.get(id)
    }

    pub fn has_node(&self, id: &MapNodeId) -> bool {
        self.state.contains_key(id)
    }

    pub fn push(&mut self, new_delta: MapDelta) {
        self.apply(&new_delta).unwrap();
        let (_new_edge, new_node) =
            self.delta_graph
                .add_child(self.cur_delta_idx.into(), (), new_delta);
        self.cur_delta_idx = new_node;
    }

    pub fn can_undo(&self) -> bool {
        self.delta_graph
            .parents(self.cur_delta_idx)
            .walk_next(&self.delta_graph)
            .is_some()
    }

    pub fn undo(&mut self) {
        let (_, parent_node_idx) = self
            .delta_graph
            .parents(self.cur_delta_idx)
            .walk_next(&self.delta_graph)
            .expect("Use can_undo() to check first");

        let reverse_of_current = self
            .delta_graph
            .node_weight(self.cur_delta_idx)
            .unwrap()
            .reverse_of();

        self.apply(&reverse_of_current).unwrap();
        self.cur_delta_idx = parent_node_idx;
    }

    pub fn can_redo(&self) -> bool {
        self.delta_graph
            .children(self.cur_delta_idx)
            .walk_next(&self.delta_graph)
            .is_some()
    }

    pub fn redo(&mut self) {
        // Assume the last child node to be most relevant change tree.
        let (_, child_node_idx) = self
            .delta_graph
            .children(self.cur_delta_idx)
            .walk_next(&self.delta_graph)
            .expect("Use can_redo() to check first");

        let child_delta = self
            .delta_graph
            .node_weight(child_node_idx)
            .unwrap()
            .clone();
        self.apply(&child_delta).unwrap();
        self.cur_delta_idx = child_node_idx;
    }

    fn apply(&mut self, action: &MapDelta) -> Result<(), MapDeltaApplyError> {
        match action {
            MapDelta::Nop => Ok(()),
            MapDelta::AddNode { id, node } => {
                if self.state.insert(*id, node.clone()).is_none() {
                    Ok(())
                } else {
                    Err(MapDeltaApplyError::NodeExists)
                }
            }
            MapDelta::ModifyNode { id, after, .. } => {
                if let Some(node) = self.state.get_mut(id) {
                    *node = after.clone();
                    Ok(())
                } else {
                    Err(MapDeltaApplyError::NodeNotFound)
                }
            }
            MapDelta::RemoveNode { id, .. } => {
                if self.state.remove(id).is_some() {
                    Ok(())
                } else {
                    Err(MapDeltaApplyError::NodeNotFound)
                }
            }
        }
    }
}
impl MapDelta {
    pub fn reverse_of(&self) -> MapDelta {
        match self {
            MapDelta::Nop => MapDelta::Nop,
            MapDelta::AddNode { id, node } => MapDelta::RemoveNode {
                id: *id,
                node: node.clone(),
            },
            MapDelta::ModifyNode { id, before, after } => MapDelta::ModifyNode {
                id: *id,
                before: after.clone(),
                after: before.clone(),
            },
            MapDelta::RemoveNode { id, node } => MapDelta::AddNode {
                id: *id,
                node: node.clone(),
            },
        }
    }
}

impl MapNode {
    pub fn name(&self) -> &'static str {
        match self {
            MapNode::Brush(..) => "Brush",
            MapNode::Light(..) => "Light",
        }
    }
}

impl MapContext {
    pub fn node_to_entity(&self, node_id: &MapNodeId) -> Option<&Entity> {
        self.node_lookup.get_by_left(node_id)
    }

    pub fn entity_to_node(&self, entity_id: &Entity) -> Option<&MapNodeId> {
        self.node_lookup.get_by_right(entity_id)
    }
}

#[derive(Resource)]
struct LoadingMap {
    path: PathBuf,
    task: Task<MapLoadResult>,
}

pub type MapLoadResult = Result<Map, postcard::Error>;

pub const MAP_FILE_EXT: &str = "mmap";
pub const DEFAULT_MAP_NAME: &str = "map";

pub fn default_map_filename() -> String {
    format!("{}.{}", DEFAULT_MAP_NAME, MAP_FILE_EXT)
}

fn start_loading_map(data_path: Res<AppDataPath>, mut commands: Commands) {
    let path: PathBuf = [data_path.get(), &default_map_filename()].iter().collect();
    if path.exists() {
        let task_owned_path = path.clone();
        let task_pool = AsyncComputeTaskPool::get();
        commands.insert_resource(LoadingMap {
            path,
            task: task_pool.spawn(load_map_async(task_owned_path)),
        });
    } else {
        info!(
            "no map file found at {}. inserting empty map",
            path.display()
        );
        commands.insert_resource(Map::default());
        commands.insert_resource(MapContext {
            save_path: path.clone(),
            node_lookup: default(),
        });
    }
}

async fn load_map_async(path: PathBuf) -> MapLoadResult {
    let bytes = std::fs::read(path).unwrap();
    postcard::from_bytes::<Map>(&bytes)
}

fn insert_map_when_loaded(
    mut loading_map: ResMut<LoadingMap>,
    mut commands: Commands,
    mut tp_writer: EventWriter<TPCameraTo>,
) {
    if let Some(map_result) = block_on(future::poll_once(&mut loading_map.task)) {
        commands.remove_resource::<LoadingMap>();

        match map_result {
            Ok(map) => {
                tp_writer.send(TPCameraTo(map.editor_context.camera_pos));
                commands.insert_resource(map);
                commands.insert_resource(MapContext {
                    save_path: loading_map.path.clone(),
                    node_lookup: default(),
                });
            }
            Err(err) => {
                error!("Failed to load map: {}", err);
            }
        }
    }
}

fn update_editor_context(mut map: ResMut<Map>, q_camera: Query<(&GlobalTransform, &Gimbal)>) {
    let (cam_t, cam_g) = q_camera.single();
    let new_context = EditorContext {
        camera_pos: GimbalPos::new(cam_t.translation(), *cam_g),
    };
    map.editor_context = new_context;
}

fn save_map(map: Res<Map>, map_context: Res<MapContext>) {
    // TODO: async, of course this would mean it can't run on AppExit.

    let file = File::create(&map_context.save_path).unwrap();
    postcard::to_io(map.into_inner(), file).unwrap();
}

fn unload_map(q_live_nodes: Query<Entity, With<MapNodeId>>, mut commands: Commands) {
    info!("unload map with {} nodes", q_live_nodes.iter().count());
    for entity in q_live_nodes.iter() {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<Map>();
}

fn init_empty_map(
    data_path: Res<AppDataPath>,
    map_context: Option<Res<MapContext>>,
    mut commands: Commands,
) {
    commands.insert_resource(Map::default());
    if map_context.is_none() {
        commands.insert_resource(MapContext {
            save_path: [data_path.get(), &default_map_filename()].iter().collect(),
            node_lookup: default(),
        });
    }
}

fn load_map_assets(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let texture = asset_server.load("concrete.png");

    let material = materials.add(StandardMaterial {
        base_color_texture: Some(texture),
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..default()
    });

    commands.insert_resource(MapAssets {
        default_material: material,
    });
}

fn apply_changes_to_map(
    mut id_gen: ResMut<IdGen>,
    mut mod_events: EventReader<MapChange>,
    mut map: ResMut<Map>,
) {
    for mod_event in mod_events.read() {
        info!("mod event!");
        let delta = match mod_event {
            MapChange::Add(node) => MapDelta::AddNode {
                id: MapNodeId(id_gen.generate()),
                node: node.clone(),
            },
            MapChange::Modify(node_id, node) => {
                let prev = map.get_node(node_id).unwrap();
                MapDelta::ModifyNode {
                    id: *node_id,
                    before: prev.clone(),
                    after: node.clone(),
                }
            }
            MapChange::Remove(node_id) => {
                let prev = map.get_node(node_id).unwrap();
                MapDelta::RemoveNode {
                    id: *node_id,
                    node: prev.clone(),
                }
            }
        };
        map.push(delta);
    }
}

pub fn undo_map_change(mut map: ResMut<Map>) {
    if map.can_undo() {
        map.undo();
    } else {
        info!("no parent nodes");
    }
}

pub fn redo_map_change(mut map: ResMut<Map>) {
    if map.can_redo() {
        map.redo();
    } else {
        info!("no child nodes");
    }
}

fn reflect_map_changes_in_world(
    mut last_map: Local<Option<Map>>,
    new_map: Res<Map>,
    mut map_context: ResMut<MapContext>,
    mut deploy_events: EventWriter<DeployMapNode>,
    mut commands: Commands,
) {
    info!("reflecting map changes !!");

    if let Some(ref last_map) = *last_map {
        for (node_id, node) in new_map.iter() {
            if let Some(last_node) = last_map.get_node(node_id) {
                if node != last_node {
                    // Modify
                    info!("mod {}", node_id);
                    let entity_id = *map_context
                        .node_to_entity(node_id)
                        .expect("Modified node to already be instantiated in world");
                    deploy_events.send(DeployMapNode {
                        entity_id,
                        node: node.clone(),
                    });
                }
            } else {
                // Add
                let entity_id = commands.spawn(node_id.clone()).id();
                map_context.node_lookup.insert(*node_id, entity_id);
                deploy_events.send(DeployMapNode {
                    entity_id,
                    node: node.clone(),
                });
                info!("add {}", entity_id);
            }
        }

        let removed_node_entities: Vec<Entity> = last_map
            .node_ids()
            .filter(|id| !new_map.has_node(id))
            .filter_map(|id| map_context.node_to_entity(id))
            .cloned()
            .collect();

        for entity in removed_node_entities {
            info!("fuck off {}", entity);
            commands.entity(entity).despawn_recursive();
            map_context.node_lookup.remove_by_right(&entity);
        }
    } else {
        // Nothing to compare with, add all.
        for (node_id, node) in new_map.iter() {
            let entity_id = commands.spawn(node_id.clone()).id();
            map_context.node_lookup.insert(*node_id, entity_id);
            deploy_events.send(DeployMapNode {
                entity_id,
                node: node.clone(),
            });
            info!("add (nothing b4) {}", entity_id);
        }
    }

    *last_map = Some(new_map.clone());
}

fn deploy_nodes(
    map_assets: Res<MapAssets>,
    mut deploy_events: EventReader<DeployMapNode>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for event in deploy_events.read() {
        let mut entity_commands = commands.entity(event.entity_id);
        entity_commands.despawn_descendants();
        // NOTE: When this is applied, the Children component will be gone, so it's important to
        // despawn descendants BEFORE retaining.
        entity_commands.retain::<MapNodeId>();

        // Once this match is stupid large it should be split up. Perhaps using observer pattern,
        // fire an event using MapNodeKind generic. Register listeners for each kind.
        match &event.node {
            MapNode::Light(ref light) => {
                entity_commands.insert((
                    Transform::from_translation(light.position),
                    match light.light_type {
                        light::LightType::Point => PointLight {
                            color: light.color,
                            intensity: light.intensity,
                            range: light.range,
                            ..default()
                        },
                        light::LightType::Spot => PointLight {
                            color: light.color,
                            intensity: light.intensity,
                            range: light.range,
                            ..default()
                        },
                    },
                ));
            }
            MapNode::Brush(ref brush) => {
                // Brush will use base entity as a container for sides.
                let center = brush.bounds.center();
                let size = brush.bounds.size();

                entity_commands.insert((
                    brush.clone(),
                    Transform::IDENTITY.with_translation(center),
                    RigidBody::Static,
                    Collider::cuboid(size.x, size.y, size.z),
                ));

                for side in brush.bounds.sides_local() {
                    commands
                        .spawn((
                            Transform::IDENTITY.with_translation(side.pos),
                            Mesh3d(meshes.add(side.plane.mesh())),
                            //MeshMaterial3d(materials.add(color)),
                            MeshMaterial3d(map_assets.default_material.clone()),
                        ))
                        .set_parent(event.entity_id);
                }
            }
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_event::<MapChange>()
        .add_event::<DeployMapNode>()
        .add_systems(Startup, (init_empty_map, start_loading_map))
        .add_systems(
            PreUpdate,
            (
                (init_empty_map)
                    .chain()
                    .run_if(input_just_released(KeyCode::KeyR)),
                undo_map_change.run_if(input_just_pressed(Binding::Undo)),
                redo_map_change.run_if(input_just_pressed(Binding::Redo)),
                save_map.run_if(input_just_pressed(Binding::Save)),
            )
                .after(InputBindingSystem),
        )
        .add_systems(
            Update,
            (
                load_map_assets.run_if(resource_added::<Map>),
                (update_editor_context, save_map)
                    .chain()
                    .run_if(resource_exists::<Map>.and(on_event::<AppExit>)),
                insert_map_when_loaded.run_if(resource_exists::<LoadingMap>),
                apply_changes_to_map.run_if(resource_exists::<Map>),
                reflect_map_changes_in_world.run_if(resource_exists_and_changed::<Map>),
                deploy_nodes
                    .after(reflect_map_changes_in_world)
                    .after(load_map_assets),
            ),
        );
}

pub mod brush;

use crate::{app_data::AppDataPath, core::input_binding::InputBindingSystem, util::IdGen};
use avian3d::prelude::{Collider, RigidBody};
use bevy::{
    input::common_conditions::{input_just_pressed, input_just_released},
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use bimap::BiHashMap;
use brush::Brush;
use daggy::{Dag, NodeIndex, Walker};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::File, path::PathBuf};
use ulid::{serde::ulid_as_u128, Ulid};
use wyrand::WyRand;

use super::{
    input_binding::Binding,
    view::{Gimbal, TeleportGimbalCamera},
};

#[derive(Resource, Serialize, Deserialize, Clone)]
pub struct MMap {
    state: BTreeMap<MMapNodeId, MMapNode>,
    cur_delta_idx: NodeIndex<u32>,
    delta_graph: Dag<MMapDelta, ()>,
    pub editor_context: EditorContext,
}

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
pub struct MMapNodeId(#[serde(with = "ulid_as_u128")] Ulid);

#[derive(Serialize, Deserialize, Clone, PartialEq, Component)]
pub struct MMapNode {
    pub name: String,
    pub kind: MMapNodeKind,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum MMapNodeKind {
    Brush(Brush),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum MMapDelta {
    Nop,
    AddNode {
        id: MMapNodeId,
        node: MMapNode,
    },
    ModifyNode {
        id: MMapNodeId,
        before: MMapNode,
        after: MMapNode,
    },
    RemoveNode {
        id: MMapNodeId,
        node: MMapNode,
    },
}

#[derive(Debug)]
enum MMapOpErr {
    NodeExists,
    NodeNotFound,
}

impl Default for MMap {
    fn default() -> Self {
        let mut graph: Dag<MMapDelta, ()> = Dag::new();
        let root_node = graph.add_node(MMapDelta::Nop);
        Self {
            state: BTreeMap::new(),
            cur_delta_idx: root_node,
            delta_graph: graph,
            editor_context: EditorContext::default(),
        }
    }
}

impl MMap {
    pub fn nodes(&self) -> impl Iterator<Item = &MMapNode> {
        self.state.values()
    }

    pub fn node_ids(&self) -> impl Iterator<Item = &MMapNodeId> {
        self.state.keys()
    }

    pub fn nodes_with_id(&self) -> impl Iterator<Item = (&MMapNodeId, &MMapNode)> {
        self.state.iter()
    }

    pub fn get_node(&self, id: &MMapNodeId) -> Option<&MMapNode> {
        self.state.get(id)
    }

    pub fn has_node(&self, id: &MMapNodeId) -> bool {
        self.state.contains_key(id)
    }

    pub fn push(&mut self, new_delta: MMapDelta) {
        self.apply(&new_delta).unwrap();
        let (_new_edge, new_node) =
            self.delta_graph
                .add_child(self.cur_delta_idx.into(), (), new_delta);
        self.cur_delta_idx = new_node;
    }

    pub fn undo(&mut self) {
        if let Some((_, parent_node_idx)) = self
            .delta_graph
            .parents(self.cur_delta_idx)
            .walk_next(&self.delta_graph)
        {
            let reverse_of_current = self
                .delta_graph
                .node_weight(self.cur_delta_idx)
                .unwrap()
                .reverse_of();

            self.apply(&reverse_of_current).unwrap();
            self.cur_delta_idx = parent_node_idx;
        } else {
            warn!(
                "Did not undo - no parent node found for {}.",
                self.cur_delta_idx.index()
            );
        }
    }

    pub fn redo(&mut self) {
        // Assume the last child node to be most relevant change tree.
        if let Some((_, child_node_idx)) = self
            .delta_graph
            .children(self.cur_delta_idx)
            .iter(&self.delta_graph)
            .last()
        {
            let child_delta = self
                .delta_graph
                .node_weight(child_node_idx)
                .unwrap()
                .clone();
            self.apply(&child_delta).unwrap();
            self.cur_delta_idx = child_node_idx;
        } else {
            warn!(
                "Did not redo - no child nodes found for {}.",
                self.cur_delta_idx.index()
            );
        }
    }

    fn apply(&mut self, action: &MMapDelta) -> Result<(), MMapOpErr> {
        match action {
            MMapDelta::Nop => Ok(()),
            MMapDelta::AddNode { id, node } => {
                if self.state.insert(*id, node.clone()).is_some() {
                    Ok(())
                } else {
                    Err(MMapOpErr::NodeExists)
                }
            }
            MMapDelta::ModifyNode { id, after, .. } => {
                if let Some(node) = self.state.get_mut(id) {
                    *node = after.clone();
                    Ok(())
                } else {
                    Err(MMapOpErr::NodeNotFound)
                }
            }
            MMapDelta::RemoveNode { id, .. } => {
                if self.state.remove(id).is_some() {
                    Ok(())
                } else {
                    Err(MMapOpErr::NodeNotFound)
                }
            }
        }
    }
}

impl MMapDelta {
    pub fn reverse_of(&self) -> MMapDelta {
        match self {
            MMapDelta::Nop => MMapDelta::Nop,
            MMapDelta::AddNode { id, node } => MMapDelta::RemoveNode {
                id: *id,
                node: node.clone(),
            },
            MMapDelta::ModifyNode { id, before, after } => MMapDelta::ModifyNode {
                id: *id,
                before: after.clone(),
                after: before.clone(),
            },
            MMapDelta::RemoveNode { id, node } => MMapDelta::AddNode {
                id: *id,
                node: node.clone(),
            },
        }
    }
}

impl MMapNodeKind {
    pub fn name(&self) -> &'static str {
        match self {
            MMapNodeKind::Brush(_) => "Brush",
        }
    }
}

#[derive(Resource)]
pub struct MMapContext {
    pub save_path: PathBuf,
    pub node_lookup: BiHashMap<MMapNodeId, Entity>,
}

/// Relevant editor state stored in the map file.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct EditorContext {
    camera_pos: Vec3,
    camera_gimbal: Gimbal,
}

impl MMapContext {
    pub fn new(save_path: PathBuf) -> Self {
        Self {
            save_path,
            node_lookup: default(),
        }
    }
}

#[derive(Resource)]
struct LoadingMMap {
    path: PathBuf,
    task: Task<MMapLoadResult>,
}

pub type MMapLoadResult = Result<MMap, postcard::Error>;

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
        commands.insert_resource(LoadingMMap {
            path,
            task: task_pool.spawn(load_map_async(task_owned_path)),
        });
    } else {
        info!(
            "no map file found at {}. inserting empty map",
            path.display()
        );
        commands.insert_resource(MMap::default());
        commands.insert_resource(MMapContext::new(path.clone()));
    }
}

async fn load_map_async(path: PathBuf) -> MMapLoadResult {
    let bytes = std::fs::read(path).unwrap();
    postcard::from_bytes::<MMap>(&bytes)
}

fn insert_map_when_loaded(
    mut loading_map: ResMut<LoadingMMap>,
    mut commands: Commands,
    mut changed_events: EventWriter<MMapNodeDeploy>,
    mut tp_writer: EventWriter<TeleportGimbalCamera>,
) {
    if let Some(map_result) = block_on(future::poll_once(&mut loading_map.task)) {
        commands.remove_resource::<LoadingMMap>();

        match map_result {
            Ok(map) => {
                tp_writer.send(TeleportGimbalCamera {
                    new_pos: map.editor_context.camera_pos,
                    new_gimbal: map.editor_context.camera_gimbal.clone(),
                });

                commands.insert_resource(map);
                commands.insert_resource(MMapContext::new(loading_map.path.clone()));
            }
            Err(err) => {
                error!("Failed to load map: {}", err);
            }
        }
    }
}

fn update_editor_context(mut map: ResMut<MMap>, q_camera: Query<(&GlobalTransform, &Gimbal)>) {
    let (cam_t, cam_g) = q_camera.single();
    let new_context = EditorContext {
        camera_pos: cam_t.translation(),
        camera_gimbal: cam_g.clone(),
    };
    map.editor_context = new_context;
}

fn save_map(map: Res<MMap>, map_context: Res<MMapContext>) {
    // TODO: async, of course this would mean it can't run on AppExit.

    let file = File::create(&map_context.save_path).unwrap();
    postcard::to_io(map.into_inner(), file).unwrap();
}

fn unload_map(q_live_nodes: Query<Entity, With<MMapNodeId>>, mut commands: Commands) {
    info!("unload map with {} nodes", q_live_nodes.iter().count());
    for entity in q_live_nodes.iter() {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<MMap>();
    commands.remove_resource::<MMapContext>();
}

fn init_empty_map(data_path: Res<AppDataPath>, mut commands: Commands) {
    commands.init_resource::<MMap>();
    commands.insert_resource(MMapContext::new(
        [data_path.get(), &default_map_filename()].iter().collect(),
    ));
}

// fn create_new_map_nodes(
//     mut id_gen: ResMut<IdGen>,
//     mut create_node_events: EventReader<CreateNewMapNode>,
//     mut map: ResMut<LiveGameMap>,
//     mut deploy_events: EventWriter<LiveMapNodeChanged>,
//     mut commands: Commands,
// ) {
//     for event in create_node_events.read() {
//         info!("creating map node");
//
//         let id = id_gen.generate();
//         let kind = event.0.clone();
//         let name = kind.name().to_string();
//
//         let entity = commands.spawn(MapNode { id, name, kind }).id();
//         map.node_lookup_table.insert(id, entity);
//         deploy_events.send(LiveMapNodeChanged(entity));
//     }
// }

// fn remove_despawned_entites_from_lookup_table(
//     mut removals: RemovedComponents<MapNode>,
//     mut live_map: ResMut<LiveGameMap>,
// ) {
//     let removed_entities: HashSet<Entity> = removals.read().collect();
//     live_map
//         .node_lookup_table
//         .retain(|_, v| !removed_entities.contains(v));
// }

#[derive(Event)]
pub enum MMapMod {
    Add(MMapNodeKind),
    Modify(MMapNodeId, MMapNode),
    Remove(MMapNodeId),
}

#[derive(Event)]
struct MMapNodeDeploy(pub MMapNodeId);

fn map_mod_to_delta(
    mut id_gen: ResMut<IdGen>,
    mut mod_events: EventReader<MMapMod>,
    mut map: ResMut<MMap>,
) {
    for mod_event in mod_events.read() {
        info!("mod event!");
        let delta = match mod_event {
            MMapMod::Add(node_kind) => MMapDelta::AddNode {
                id: MMapNodeId(id_gen.generate()),
                node: MMapNode {
                    name: node_kind.name().to_string(),
                    kind: node_kind.clone(),
                },
            },
            MMapMod::Modify(node_id, node) => {
                let prev = map.get_node(node_id).unwrap();
                MMapDelta::ModifyNode {
                    id: *node_id,
                    before: prev.clone(),
                    after: node.clone(),
                }
            }
            MMapMod::Remove(node_id) => {
                let prev = map.get_node(node_id).unwrap();
                MMapDelta::RemoveNode {
                    id: *node_id,
                    node: prev.clone(),
                }
            }
        };
        map.push(delta);
    }
}

pub fn map_undo(mut map: ResMut<MMap>) {
    map.undo();
}

pub fn map_redo(mut map: ResMut<MMap>) {
    map.redo();
}

fn reflect_map_changes_in_world(
    mut last_map: Local<Option<MMap>>,
    new_map: Res<MMap>,
    mut map_context: ResMut<MMapContext>,
    mut deploy_events: EventWriter<MMapNodeDeploy>,
    mut commands: Commands,
) {
    info!("reflecting map changes !!");

    if let Some(ref last_map) = *last_map {
        for node_id in new_map.node_ids() {
            if last_map.has_node(node_id) {
                // Modify
                deploy_events.send(MMapNodeDeploy(*node_id));
            } else {
                // Add
                let entity_id = commands.spawn(node_id.clone()).id();
                map_context.node_lookup.insert(*node_id, entity_id);
                deploy_events.send(MMapNodeDeploy(*node_id));
            }
        }

        let removed_node_entities: Vec<Entity> = last_map
            .node_ids()
            .filter(|id| !new_map.has_node(id))
            .filter_map(|id| map_context.node_lookup.get_by_left(id))
            .cloned()
            .collect();
        for entity in removed_node_entities {
            commands.entity(entity).despawn_recursive();
            map_context.node_lookup.remove_by_right(&entity);
        }
    } else {
        // Nothing to compare with, add all.
        for node_id in new_map.node_ids() {
            let entity_id = commands.spawn(node_id.clone()).id();
            map_context.node_lookup.insert(*node_id, entity_id);
            deploy_events.send(MMapNodeDeploy(*node_id));
        }
    }

    *last_map = Some(new_map.clone());
}

fn deploy_nodes(
    map: Res<MMap>,
    map_context: Res<MMapContext>,
    mut changed_events: EventReader<MMapNodeDeploy>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("deplopy");
    for event in changed_events.read() {
        let node_id = event.0;
        let node_entity_id = *map_context.node_lookup.get_by_left(&event.0).unwrap();

        let mut entity_commands = commands.entity(node_entity_id);
        entity_commands.retain::<MMapNodeId>();
        entity_commands.despawn_descendants();

        let node = map.get_node(&event.0).unwrap();

        // Once this match is stupid large it should be split up. Perhaps using observer pattern,
        // fire an event using MapNodeKind generic. Register listeners for each kind.
        match &node.kind {
            MMapNodeKind::Brush(ref brush) => {
                // Brush will use base entity as a container for sides.
                let center = brush.bounds.center();
                let size = brush.bounds.size();

                entity_commands.insert((
                    Visibility::Visible,
                    Transform::IDENTITY.with_translation(center),
                    RigidBody::Static,
                    Collider::cuboid(size.x, size.y, size.z),
                ));

                let mut rng = WyRand::new(node_id.0 .0 as u64);
                let color = rng.next_u32();
                let r = (color & 0xFF) as u8;
                let g = ((color >> 8) & 0xFF) as u8;
                let b = ((color >> 16) & 0xFF) as u8;
                let color = Color::srgb_u8(r, g, b);

                for side in brush.bounds.sides() {
                    commands
                        .spawn((
                            Transform::IDENTITY.with_translation(side.pos),
                            Mesh3d(meshes.add(side.plane.mesh())),
                            MeshMaterial3d(materials.add(color)),
                        ))
                        .set_parent(node_entity_id);
                }
            }
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_event::<MMapMod>()
        .add_event::<MMapNodeDeploy>()
        .add_systems(Startup, start_loading_map)
        .add_systems(Startup, init_empty_map)
        .add_systems(
            PreUpdate,
            (
                (unload_map, init_empty_map).run_if(input_just_released(KeyCode::KeyR)),
                map_undo.run_if(input_just_pressed(Binding::Undo)),
                map_redo.run_if(input_just_pressed(Binding::Redo)),
            )
                .after(InputBindingSystem),
        )
        .add_systems(
            Update,
            (
                (update_editor_context, save_map)
                    .chain()
                    .run_if(resource_exists::<MMap>.and(on_event::<AppExit>)),
                insert_map_when_loaded.run_if(resource_exists::<LoadingMMap>),
                map_mod_to_delta.run_if(resource_exists::<MMap>),
                (reflect_map_changes_in_world, deploy_nodes)
                    .chain()
                    .run_if(resource_exists_and_changed::<MMap>),
            ),
        );
}

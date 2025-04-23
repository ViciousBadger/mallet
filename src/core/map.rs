pub mod brush;

use crate::{app_data::AppDataPath, core::input_binding::InputBindingSystem, util::IdGen};
use avian3d::prelude::{Collider, RigidBody};
use bevy::{
    input::common_conditions::input_just_released,
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
    utils::{HashMap, HashSet},
};
use brush::Brush;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::{fs::File, path::PathBuf};
use ulid::{serde::ulid_as_u128, Ulid};
use wyrand::WyRand;

#[derive(Serialize, Deserialize, Default)]
pub struct StoredGameMap {
    pub nodes: Vec<MapNode>,
}

#[derive(Resource)]
pub struct LiveGameMap {
    pub save_path: PathBuf,
    pub node_lookup_table: HashMap<Ulid, Entity>,
}

impl LiveGameMap {
    pub fn new(save_path: PathBuf) -> Self {
        Self {
            save_path,
            node_lookup_table: default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Component)]
pub struct MapNode {
    #[serde(with = "ulid_as_u128")]
    pub id: Ulid,
    pub name: String,
    pub kind: MapNodeKind,
}

#[derive(Resource)]
struct LoadingMap {
    path: PathBuf,
    task: Task<StoredGameMap>,
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
pub struct LiveMapNodeChanged(Entity);

pub const MAP_FILE_EXT: &str = "mmap";
pub const DEFAULT_MAP_NAME: &str = "map";

pub fn default_map_filename() -> String {
    format!("{}.{}", DEFAULT_MAP_NAME, MAP_FILE_EXT)
}

fn start_loading_map(data_path: Res<AppDataPath>, mut commands: Commands) {
    let path: PathBuf = [data_path.get(), &default_map_filename()].iter().collect();
    if path.exists() {
        let task_owned_path = path.clone();
        let task = async move {
            let bytes = std::fs::read(task_owned_path).unwrap();
            let map: StoredGameMap = postcard::from_bytes(&bytes).unwrap();
            map
        };
        let task_pool = AsyncComputeTaskPool::get();
        commands.insert_resource(LoadingMap {
            path,
            task: task_pool.spawn(task),
        });
    } else {
        info!(
            "no map file found at {}. inserting empty map",
            path.display()
        );
        commands.insert_resource(LiveGameMap::new(path.clone()));
    }
}

fn insert_map_when_loaded(
    mut loading_map: ResMut<LoadingMap>,
    mut commands: Commands,
    mut changed_events: EventWriter<LiveMapNodeChanged>,
) {
    if let Some(map) = block_on(future::poll_once(&mut loading_map.task)) {
        commands.remove_resource::<LoadingMap>();
        let mut live_map = LiveGameMap::new(loading_map.path.clone());
        for node in map.nodes {
            let node_id = node.id;
            let entity = commands.spawn(node).id();
            changed_events.send(LiveMapNodeChanged(entity));
            live_map.node_lookup_table.insert(node_id, entity);
        }
        commands.insert_resource(live_map);
    }
}

fn save_map(map: Res<LiveGameMap>, q_live_nodes: Query<&MapNode>) {
    // TODO: async, of course this would mean it can't run on AppExit.
    let file = File::create(&map.save_path).unwrap();

    let stored_map = StoredGameMap {
        nodes: q_live_nodes.iter().cloned().collect(),
    };
    postcard::to_io(&stored_map, file).unwrap();
}

fn unload_map(q_live_nodes: Query<Entity, With<MapNode>>, mut commands: Commands) {
    info!("unload map with {} nodes", q_live_nodes.iter().count());
    for entity in q_live_nodes.iter() {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<LiveGameMap>();
}

fn init_empty_map(data_path: Res<AppDataPath>, mut commands: Commands) {
    commands.insert_resource(LiveGameMap::new(
        [data_path.get(), &default_map_filename()].iter().collect(),
    ));
}

fn create_new_map_nodes(
    mut id_gen: ResMut<IdGen>,
    mut create_node_events: EventReader<CreateNewMapNode>,
    mut map: ResMut<LiveGameMap>,
    mut deploy_events: EventWriter<LiveMapNodeChanged>,
    mut commands: Commands,
) {
    for event in create_node_events.read() {
        info!("creating map node");

        let id = id_gen.generate();
        let kind = event.0.clone();
        let name = kind.name().to_string();

        let entity = commands.spawn(MapNode { id, name, kind }).id();
        map.node_lookup_table.insert(id, entity);
        deploy_events.send(LiveMapNodeChanged(entity));
    }
}

fn remove_despawned_entites_from_lookup_table(
    mut removals: RemovedComponents<MapNode>,
    mut live_map: ResMut<LiveGameMap>,
) {
    let removed_entities: HashSet<Entity> = removals.read().collect();
    live_map
        .node_lookup_table
        .retain(|_, v| !removed_entities.contains(v));
}

fn deploy_nodes(
    q_live_nodes: Query<&MapNode>,
    mut changed_events: EventReader<LiveMapNodeChanged>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for event in changed_events.read() {
        let node_entity_id = event.0;

        let live_node = q_live_nodes.get(node_entity_id).unwrap();

        let mut entity_commands = commands.entity(node_entity_id);
        entity_commands.retain::<MapNode>();
        entity_commands.despawn_descendants();

        // Once this match is stupid large it should be split up. Perhaps using observer pattern,
        // fire an event using MapNodeKind generic. Register listeners for each kind.
        match &live_node.kind {
            MapNodeKind::Brush(ref brush) => {
                // Brush will use base entity as a container for sides.
                let center = brush.bounds.center();
                let size = brush.bounds.size();

                entity_commands.insert((
                    Visibility::Visible,
                    Transform::IDENTITY.with_translation(center),
                    RigidBody::Static,
                    Collider::cuboid(size.x, size.y, size.z),
                ));

                let mut rng = WyRand::new(live_node.id.0 as u64);
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
    app.add_systems(Startup, start_loading_map)
        .add_systems(Startup, init_empty_map)
        .add_systems(
            PreUpdate,
            (
                deploy_nodes,
                (unload_map, init_empty_map)
                    .run_if(input_just_released(KeyCode::KeyR))
                    .after(InputBindingSystem),
            ),
        )
        .add_systems(
            PostUpdate,
            save_map.run_if(resource_exists::<LiveGameMap>.and(on_event::<AppExit>)),
        )
        .add_systems(
            Last,
            (
                insert_map_when_loaded.run_if(resource_exists::<LoadingMap>),
                (
                    create_new_map_nodes,
                    remove_despawned_entites_from_lookup_table,
                )
                    .run_if(resource_exists::<LiveGameMap>),
            ),
        )
        .add_event::<CreateNewMapNode>()
        .add_event::<LiveMapNodeChanged>();
}

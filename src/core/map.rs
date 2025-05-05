mod dto;
pub mod history;
pub mod nodes;

use bevy::{
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    input::common_conditions::{input_just_pressed, input_just_released},
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use bimap::BiHashMap;
use itertools::Itertools;
use std::{fs::File, io::Write, path::PathBuf};

use crate::{
    app_data::AppDataPath,
    core::{
        binds::{Binding, InputBindingSystem},
        map::{
            dto::{MapDe, MapNodeSer, MapSer},
            history::{MapDelta, MapHistory},
            nodes::{brush::Brush, light::Light, MapNodeMeta, TypedMapNode},
        },
        view::TPCameraTo,
    },
    editor::{update_editor_context, EditorContext},
    util::Id,
};

#[derive(Resource)]
pub struct MapSession {
    pub save_path: PathBuf,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct MapLookup(BiHashMap<Id, Entity>);

impl MapLookup {
    pub fn node_to_entity(&self, node_id: &Id) -> Option<&Entity> {
        self.0.get_by_left(node_id)
    }

    pub fn entity_to_node(&self, entity_id: &Entity) -> Option<&Id> {
        self.0.get_by_right(entity_id)
    }
}

#[derive(Resource)]
pub struct MapAssets {
    pub default_material: Handle<StandardMaterial>,
}

/// Combines node id with its instantiated entity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveMapNodeId {
    pub node_id: Id,
    pub entity: Entity,
}

/// Request to deploy a map node in the world.
/// Entity is expect to contain a map node component.
#[derive(Event)]
pub struct MapNodeDeploy {
    pub target: Entity,
}

#[derive(Resource)]
struct LoadingMap {
    path: PathBuf,
    task: Task<MapLoadResult>,
}

pub type MapLoadResult = Result<MapDe, ron::Error>;

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
        //commands.insert_resource(Map::default());
        commands.insert_resource(MapSession {
            save_path: path.clone(),
        });
    }
}

async fn load_map_async(path: PathBuf) -> MapLoadResult {
    let bytes = std::fs::read(path).unwrap();
    MapDe::from_bytes(&bytes)
}

fn insert_map_when_loaded(
    mut loading_map: ResMut<LoadingMap>,
    mut commands: Commands,
    mut tp_writer: EventWriter<TPCameraTo>,
    mut apply_evs: EventWriter<MapDeltaApply>,
) {
    if let Some(map_result) = block_on(future::poll_once(&mut loading_map.task)) {
        commands.remove_resource::<LoadingMap>();

        match map_result {
            Ok(loadedmap) => {
                tp_writer.send(TPCameraTo(loadedmap.editor_context.camera_pos));
                commands.insert_resource(MapSession {
                    save_path: loading_map.path.clone(),
                });

                commands.insert_resource(loadedmap.history);
                commands.insert_resource(loadedmap.editor_context);

                for de in loadedmap.brushes {
                    apply_evs.send(
                        MapDelta::AddNode {
                            id: de.meta.id,
                            name: de.meta.name,
                            node: TypedMapNode::Brush(de.node),
                        }
                        .into(),
                    );
                }
                //commands.spawn_batch(loadedmap.brushes.into_iter().map(|de| (de.meta, de.node)));
                //deploy_all_evs.send_default();
            }
            Err(err) => {
                error!("Failed to load map: {}", err);
            }
        }
    }
}

fn save_map(
    map_session: Res<MapSession>,
    map_history: Res<MapHistory>,
    editor_context: Res<EditorContext>,
    q_brushes: Query<(&MapNodeMeta, &Brush)>,
) {
    // TODO: async, of course this would mean it can't run on AppExit.

    let map_ser = MapSer {
        history: &map_history,
        editor_context: &editor_context,
        brushes: q_brushes
            .iter()
            .map(|(meta, node)| MapNodeSer { meta, node })
            .collect_vec(),
    };

    // let map_file = MapFile {
    //     map: map.clone(),
    //     editor: editor_context.clone(),
    // };

    let mut file = File::create(&map_session.save_path).unwrap();
    let bytes = map_ser.to_bytes().unwrap();
    file.write_all(&bytes).unwrap();
    // postcard::to_io(&map_file, file).unwrap();

    info!("map saved to {:?}", map_session.save_path);
}

fn unload_map(q_nodes: Query<Entity, With<MapNodeMeta>>, mut commands: Commands) {
    info!("unload map with {} nodes", q_nodes.iter().count());
    for entity in q_nodes.iter() {
        commands.entity(entity).despawn_recursive();
    }
    //commands.remove_resource::<Map>();
}

fn init_empty_map(
    data_path: Res<AppDataPath>,
    map_session: Option<Res<MapSession>>,
    mut commands: Commands,
) {
    commands.insert_resource(MapHistory::default());
    if map_session.is_none() {
        commands.insert_resource(MapSession {
            save_path: [data_path.get(), &default_map_filename()].iter().collect(),
        });
    }
}

fn brush_texture_settings(settings: &mut ImageLoaderSettings) {
    *settings = ImageLoaderSettings {
        sampler: ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::Repeat,
            address_mode_v: ImageAddressMode::Repeat,
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            ..default()
        }),
        ..default()
    }
}

fn load_map_assets(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let texture = asset_server
        .load_with_settings("base_content/surfaces/concrete.png", brush_texture_settings);

    let material = materials.add(StandardMaterial {
        base_color_texture: Some(texture),
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..default()
    });

    commands.insert_resource(MapAssets {
        default_material: material,
    });
    info!("loaded map assets.....");
}

pub fn undo_map_change(
    mut history: ResMut<MapHistory>,
    mut delta_apply: EventWriter<MapDeltaApply>,
) {
    if history.can_undo() {
        let delta = history.undo();
        delta_apply.send(delta.into());
    } else {
        info!("Nothing to undo");
    }
}

pub fn redo_map_change(
    mut history: ResMut<MapHistory>,
    mut delta_apply: EventWriter<MapDeltaApply>,
) {
    if history.can_redo() {
        let delta = history.redo();
        delta_apply.send(delta.into());
    } else {
        info!("Nothing to redo");
    }
}

/// Push map changes into the map history. (Will also apply to world)
#[derive(Event, Clone)]
pub struct MapDeltaPush(Vec<MapDelta>);

impl From<MapDelta> for MapDeltaPush {
    fn from(value: MapDelta) -> Self {
        Self(vec![value])
    }
}

impl From<Vec<MapDelta>> for MapDeltaPush {
    fn from(value: Vec<MapDelta>) -> Self {
        Self(value)
    }
}

/// Apply map changes to the world.
#[derive(Event, Clone)]
pub struct MapDeltaApply(Vec<MapDelta>);

impl From<MapDelta> for MapDeltaApply {
    fn from(value: MapDelta) -> Self {
        Self(vec![value])
    }
}

impl From<Vec<MapDelta>> for MapDeltaApply {
    fn from(value: Vec<MapDelta>) -> Self {
        Self(value)
    }
}

fn push_deltas(
    mut delta_events: EventReader<MapDeltaPush>,
    mut delta_apply: EventWriter<MapDeltaApply>,
    mut history: ResMut<MapHistory>,
) {
    for pushed_deltas in delta_events.read() {
        for delta in &pushed_deltas.0 {
            info!("push: {:?}", delta);
            history.push(delta.clone());
        }
        delta_apply.send(MapDeltaApply(pushed_deltas.0.clone()));
    }
}

fn apply_deltas(
    lookup: Res<MapLookup>,
    mut delta_events: EventReader<MapDeltaApply>,
    mut deploy_evs: EventWriter<MapNodeDeploy>,
    mut q_nodes: Query<&mut MapNodeMeta>,
    mut commands: Commands,
) {
    for delta in delta_events.read().flat_map(|ev| ev.0.iter()) {
        match delta {
            MapDelta::Nop => (),
            MapDelta::AddNode { id, name, node } => {
                let mut cmds = commands.spawn(MapNodeMeta {
                    id: *id,
                    name: name.clone(),
                });
                node.insert_as_component(cmds.reborrow());
                deploy_evs.send(MapNodeDeploy { target: cmds.id() });
            }
            MapDelta::ModifyNode { id, before, after } => {
                let e = lookup.node_to_entity(id).unwrap();
                let mut cmds = commands.entity(*e);
                before.remove_as_component(cmds.reborrow());
                after.insert_as_component(cmds.reborrow());
                deploy_evs.send(MapNodeDeploy { target: cmds.id() });
            }
            MapDelta::RenameNode { id, after, .. } => {
                let e = lookup.node_to_entity(id).unwrap();
                let mut meta = q_nodes.get_mut(*e).unwrap();
                meta.name = after.clone();
            }
            MapDelta::RemoveNode { id, .. } => {
                let e = lookup.node_to_entity(id).unwrap();
                commands.entity(*e).despawn_recursive();
            }
        }
        info!("apply: {:?}", delta);
    }
}

fn pre_deploy_clean(mut deploy_events: EventReader<MapNodeDeploy>, mut commands: Commands) {
    for event in deploy_events.read() {
        let mut entity_commands = commands.entity(event.target);
        entity_commands.despawn_descendants();
        // NOTE: When this is applied, the Children component will be gone, so it's important to
        // despawn descendants BEFORE retaining.
        // NOTE: "retain" method here has to make sure ALL map node variants are retained,
        // feels a little silly..
        entity_commands.retain::<(MapNodeMeta, Brush, Light)>();
    }
}

// fn deploy_nodes(
//     map_assets: Res<MapAssets>,
//     mut deploy_events: EventReader<DeployMapNode>,
//     mut commands: Commands,
//     mut meshes: ResMut<Assets<Mesh>>,
// ) {
//     for event in deploy_events.read() {
//         let mut entity_commands = commands.entity(event.target_entity);
//         entity_commands.despawn_descendants();
//         // NOTE: When this is applied, the Children component will be gone, so it's important to
//         // despawn descendants BEFORE retaining.
//         entity_commands.retain::<Id>();
//
//         // Once this match is stupid large it should be split up. Perhaps using observer pattern,
//         // fire an event using MapNodeKind generic. Register listeners for each kind.
//         match &event.node {
//             MapNode::Light(ref light) => {
//                 entity_commands.insert((
//                     Transform::from_translation(light.position),
//                     match light.light_type {
//                         light::LightType::Point => PointLight {
//                             color: light.color,
//                             intensity: light.intensity,
//                             range: light.range,
//                             ..default()
//                         },
//                         light::LightType::Spot => {
//                             unimplemented!("u would have to rotate it n shit")
//                         }
//                     },
//                 ));
//             }
//             MapNode::Brush(ref brush) => {
//                 // Brush will use base entity as a container for sides.
//                 let center = brush.bounds.center();
//                 let size = brush.bounds.size();
//
//                 entity_commands.insert((
//                     brush.clone(),
//                     Transform::IDENTITY.with_translation(center),
//                     RigidBody::Static,
//                     Collider::cuboid(size.x, size.y, size.z),
//                 ));
//
//                 for side in brush.bounds.sides_local() {
//                     commands
//                         .spawn((
//                             Transform::IDENTITY.with_translation(side.pos),
//                             Mesh3d(meshes.add(side.mesh())),
//                             //MeshMaterial3d(materials.add(color)),
//                             MeshMaterial3d(map_assets.default_material.clone()),
//                         ))
//                         .set_parent(event.target_entity);
//                 }
//             }
//         }
//     }
// }

fn track_map_nodes(
    q_inserted: Query<(Entity, &MapNodeMeta), Added<MapNodeMeta>>,
    mut q_removed: RemovedComponents<MapNodeMeta>,
    mut lookup: ResMut<MapLookup>,
) {
    for (entity, meta) in &q_inserted {
        lookup.insert(meta.id, entity);
        info!("begin track {}: {}", meta.id, entity);
    }
    for removed_entity in q_removed.read() {
        lookup.remove_by_right(&removed_entity);
        info!("end track {}", removed_entity);
    }
}

pub fn plugin(app: &mut App) {
    //app.add_event::<DeployMapNode>();
    app.add_plugins(nodes::plugin);
    app.add_event::<MapDeltaPush>()
        .add_event::<MapDeltaApply>()
        .add_event::<MapNodeDeploy>();
    app.init_resource::<MapLookup>();
    app.add_systems(Startup, (init_empty_map, start_loading_map));
    app.add_systems(
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
    );
    app.add_systems(
        Update,
        (
            // TODO: prob needs a differnet run condition
            load_map_assets.run_if(resource_added::<MapSession>),
            (update_editor_context, save_map)
                .chain()
                .run_if(resource_exists::<MapSession>.and(on_event::<AppExit>)),
            insert_map_when_loaded.run_if(resource_exists::<LoadingMap>),
            push_deltas,
            apply_deltas.after(push_deltas),
            pre_deploy_clean.after(apply_deltas),
            // apply_changes_to_map.run_if(resource_exists::<Map>),
            // reflect_map_changes_in_world.run_if(resource_exists_and_changed::<Map>),
            // deploy_nodes
            //     .after(reflect_map_changes_in_world)
            //     .after(load_map_assets),
        ),
    );
    app.add_systems(Last, track_map_nodes.run_if(on_event::<MapDeltaApply>));
}

pub mod changes;
pub mod elements;
pub mod history;
pub mod states;

use bevy::{log::tracing::Instrument, platform::collections::HashMap, prelude::*};
use color_eyre::eyre::eyre;
use redb::ReadTransaction;
use thiserror::Error;

use crate::{
    app_data::{self, AppDataPath},
    core::{
        db::{Db, EnsureExists, Meta, TBL_META, TBL_OBJECTS},
        map::{
            changes::{ChangeSet, PendingChanges},
            elements::{
                brush::Brush, light::Light, AppRoleRegistry, ElementId, ElementRoleRegistry,
            },
            history::{HistNode, UpdateCurrentHistNode, TBL_HIST_NODES},
            states::{MapState, RestoreState, StateSnapshot, TBL_STATES},
        },
    },
    id::{Id, IdGen},
    util::brush_texture_settings,
};

pub fn db_is_initialized(reader: &ReadTransaction) -> bool {
    // This clusterfuck ensures false is returned when the meta table doesn't exist yet
    // (redb has no "table exists" function afaik)
    reader
        .open_table(TBL_META)
        .map(|table| table.get(()).unwrap_or(None).is_some())
        .ok()
        .is_some()
}

pub fn get_current_meta(reader: &ReadTransaction) -> Result<Meta> {
    Ok(reader
        .open_table(TBL_META)?
        .get(())?
        .ensure_exists()?
        .value())
}

pub fn get_current_hist_node(reader: &ReadTransaction) -> Result<HistNode> {
    let meta = get_current_meta(reader)?;
    Ok(reader
        .open_table(TBL_HIST_NODES)?
        .get(&meta.hist_node_id)?
        .ensure_exists()?
        .value())
}

pub fn get_current_state(reader: &ReadTransaction) -> Result<MapState> {
    let hist = get_current_hist_node(reader)?;
    Ok(reader
        .open_table(TBL_STATES)?
        .get(&hist.state_id)?
        .ensure_exists()?
        .value())
}

fn init_map(
    asset_server: Res<AssetServer>,
    app_data_path: Res<AppDataPath>,
    mut commands: Commands,
    mut id_gen: ResMut<IdGen>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) -> Result {
    let db_path = format!("{}/map.mmap", app_data_path.get());
    let db = Db::new(&db_path);

    let mut restore: Option<Id> = None;
    let reader = db.begin_read()?;
    if db_is_initialized(&reader) {
        // Load the map
        let hist_node = get_current_hist_node(&reader)?;
        restore = Some(hist_node.state_id);

        // dumbly insert the editor context too late
        let meta = get_current_meta(&reader)?;
        commands.insert_resource(meta.editor_context);
    } else {
        // Write initial stuff
        let writer = db.begin_write()?;
        {
            let mut tbl_states = writer.open_table(states::TBL_STATES)?;
            let initial_state_id = id_gen.generate();
            tbl_states.insert(initial_state_id, states::MapState::default())?;

            // Initial history node
            let initial_hist_id = id_gen.generate();
            writer.open_table(history::TBL_HIST_NODES)?.insert(
                initial_hist_id,
                HistNode {
                    timestamp: history::new_timestamp(),
                    parent_id: None,
                    child_ids: Vec::default(),
                    state_id: initial_state_id,
                },
            )?;

            // Init the object table so it exists even if no objects are written.
            writer.open_table(TBL_OBJECTS)?;

            writer.open_table(TBL_META)?.insert(
                (),
                Meta {
                    name: "test map".to_string(),
                    hist_node_id: initial_hist_id,
                    editor_context: default(),
                },
            )?;
        }
        writer.commit()?;
    }

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

    // Command ordering is important here, db has to exist when state is restored.
    commands.insert_resource(db);

    if let Some(id) = restore {
        commands.trigger(RestoreState {
            id,
            fresh_map: true,
        });
    }

    Ok(())
}

#[derive(Resource, Default)]
pub struct ElementLookup(HashMap<Id, Entity>);

#[derive(Error, Debug)]
#[error("No entity found for map element: {}", self.0)]
pub struct ElementLookupError(Id);

impl ElementLookup {
    pub fn find(&self, element_id: &Id) -> Result<Entity, ElementLookupError> {
        self.0
            .get(element_id)
            .copied()
            .ok_or(ElementLookupError(*element_id))
    }

    pub fn insert(&mut self, element_id: Id, entity: Entity) {
        self.0.insert(element_id, entity);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Id, &Entity)> {
        self.0.iter()
    }
}

fn track_element_ids(
    q_added_ids: Query<(&ElementId, Entity), Added<ElementId>>,
    mut q_removed_ids: RemovedComponents<ElementId>,
    mut lookup: ResMut<ElementLookup>,
) {
    for (id, entity) in q_added_ids.iter() {
        lookup.0.insert(**id, entity);
    }

    for entity in q_removed_ids.read() {
        lookup.0.retain(|_, e| *e != entity);
    }
}

pub fn plugin(app: &mut App) {
    app.add_plugins((states::plugin, changes::plugin, history::plugin));
    app.init_resource::<IdGen>();
    app.init_resource::<ElementLookup>();
    app.init_resource::<PendingChanges>();
    app.init_resource::<ElementRoleRegistry>();
    app.register_map_element_role::<Brush>();
    app.register_map_element_role::<Light>();
    app.add_systems(Startup, init_map);
    app.add_systems(Update, track_element_ids);
}

#[derive(Resource)]
pub struct MapAssets {
    pub default_material: Handle<StandardMaterial>,
}

pub mod db;
pub mod elements;
pub mod history;

use bevy::{input::common_conditions::input_pressed, platform::collections::HashMap, prelude::*};
use color_eyre::eyre::eyre;
use redb::ReadableTable;
use thiserror::Error;

use crate::{
    core::map::brush::{Brush, BrushBounds},
    experimental::map::{
        db::{Db, Meta, META_TABLE},
        elements::{Element, ElementId, ElementRole, ErasedContent},
        history::{Change, Delta, HistNode, Snapshot},
    },
    id::{Id, IdGen},
};

fn new_test_map(mut commands: Commands, mut id_gen: ResMut<IdGen>) -> Result {
    let map = Db::new_temp();
    let tx = map.begin_write()?;

    {
        // Initial empty snapshot

        let mut tbl_snap = tx.open_table(history::SNAPSHOT_TABLE)?;
        let initial_snap_id = id_gen.generate();
        tbl_snap.insert(initial_snap_id, Snapshot::empty())?;

        // Initial history node
        let mut tbl_hist = tx.open_table(history::HIST_TABLE)?;
        let initial_hist_id = id_gen.generate();
        tbl_hist.insert(
            initial_hist_id,
            HistNode {
                timestamp: history::new_timestamp(),
                parent_id: None,
                child_ids: Vec::default(),
                snapshot_id: initial_snap_id,
            },
        )?;

        let mut tbl_meta = tx.open_table(META_TABLE)?;
        tbl_meta.insert(
            (),
            Meta {
                name: "a map".to_string(),
                hist_node_id: initial_hist_id,
            },
        )?;
    }
    tx.commit()?;

    commands.insert_resource(map);
    Ok(())
}

fn push_test(map_db: Res<Db>, mut id_gen: ResMut<IdGen>) -> Result {
    let txn = map_db.begin_write()?;
    {
        // Insert the content
        let mut tbl_brush = txn.open_table(elements::CONTENT_TABLE_BRUSH)?;

        let new_elem_content_id = id_gen.generate();

        let new_brush = Brush {
            bounds: BrushBounds {
                start: Vec3::NEG_ONE,
                end: Vec3::ONE,
            },
        };

        tbl_brush.insert(&new_elem_content_id, &new_brush)?;

        // Create element linking to content
        let new_elem_id = id_gen.generate();
        let new_elem = Element {
            name: "a brush".to_string(),
            role: ElementRole::Brush,
            content_id: new_elem_content_id,
        };

        // Get meta to get id of current hist entry
        let mut tbl_meta = txn.open_table(db::META_TABLE)?;
        let meta = tbl_meta.get(())?.ok_or(eyre!("no map meta"))?.value();

        // Create history entry
        let hist_key = id_gen.generate();

        let new_node = HistNode {
            timestamp: history::new_timestamp(),
            parent_id: Some(meta.hist_node_id),
            child_ids: Vec::default(),
            change: Change::Element {
                key: new_elem_id,
                delta: Delta::Create {
                    content_key: new_elem_content_id,
                    element: new_elem,
                },
            },
        };

        let mut hist = txn.open_table(history::HIST_TABLE)?;
        hist.insert(&hist_key, &new_node)?;

        // Set meta to point to newest history entry
        tbl_meta.insert(
            (),
            Meta {
                hist_node_id: hist_key,
                ..meta
            },
        )?;

        info!("yey done!");
    }

    txn.commit()?;

    Ok(())
}

// Committing things to the map...
#[derive(Event)]
pub struct Commit{changes: Vec<Change>};

impl Commit {
    pub fn single(change: Change) -> Self {
        Self{changes: vec![change]}
    }

    pub fn many(changes: Vec<Change>) -> Self {
        Self{changes}
    }
}

pub enum Change {
    Create {
        name: String,
        content: ErasedContent,
    },
    Rename {
        element_key: Id,
        new_name: String,
    },
    ModifyContent {
        element_key: Id,
        new_content: ErasedContent,
    },
    Remove {
        element_key: Id,
    },
}

fn commit_to_map(
    map_db: Res<Db>,
    elem_lookup: Res<ElementLookup>,
    q_elements: Query<&mut Element>,
    mut id_gen: ResMut<IdGen>,
    mut commits: EventReader<Commit>,
) -> Result {
    // Commit to map should do changes in the game world, then capture snapshot into history... i guess..
    // right now, changes go to map, then map is diff'd with the world.. instead just push changes to world, let world be, then snap..
    // idk man. what to do with element content? save to db here in commit? save to db in snapshot process?? here its easy because event contains type erased content. not so ez in a snapshot system.
    // so if the content is inserted in db when committing, how will it be handled when snapshotting? maybe just don't care? it is there already, why bother, let it be..
    //
    for commit in commits.read() {
        // 1: reflect in world

        for change in commit.changes {
            match change {
                Change::Create { name, content } => todo!(),
                Change::Rename { element_key, new_name } => todo!(),
                Change::ModifyContent { element_key, new_content } => todo!(),
                Change::Remove { element_key } => todo!(),
            }
        }

        // 2: capture a snapshot into history - this should happen for each commit, so its difficult to split into another system, because at that point the world will be a reflection of ALL commits this frame.
        // on the other hand, world changes such as spawning will not be queryable until AFTER system has run.. yet another dilemma..
        // idea: only process at most one commit per frame. issue: event queue is cleared after a few frames, so the commits would have to be stored elsewhere.
        // ...
        // perhaps the commit_to_map system should have full world access; it won't hurt performance and should probably lock all threads anyway. that way we could capture snapshots between each commit.
    }
    Ok(())
}

fn new_thing() {
    //map_mods.write(
}
fn undo() {}
fn redo() {}

#[derive(Resource, Default)]
pub struct ElementLookup(HashMap<Id, Entity>);

#[derive(Error, Debug)]
#[error("No entity found for {}", self.0)]
pub struct ElementLookupError(Id);

impl ElementLookup {
    pub fn find(&self, element_id: &Id) -> Result<Entity, ElementLookupError> {
        self.0
            .get(element_id)
            .copied()
            .ok_or(ElementLookupError(*element_id))
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
    app.init_resource::<IdGen>();
    app.init_resource::<ElementLookup>();
    app.add_systems(Startup, new_test_map);
    app.add_systems(
        Update,
        (
            push_test.run_if(input_pressed(KeyCode::KeyF)),
            track_element_ids,
        ),
    );
}

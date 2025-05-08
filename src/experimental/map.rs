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
        history::{Change, Delta, HistNode},
    },
    id::{Id, IdGen},
};

fn new_test_map(mut commands: Commands, mut id_gen: ResMut<IdGen>) -> Result {
    let map = Db::new_temp();
    let tx = map.begin_write()?;

    {
        let mut tbl_hist = tx.open_table(history::HIST_TABLE)?;
        let initial_hist_id = id_gen.generate();
        tbl_hist.insert(
            initial_hist_id,
            HistNode {
                timestamp: history::new_timestamp(),
                parent_key: None,
                child_keys: Vec::default(),
                change: Change::InitMap,
            },
        )?;

        let mut tbl_meta = tx.open_table(META_TABLE)?;
        tbl_meta.insert(
            (),
            Meta {
                name: "a map".to_string(),
                hist_key: initial_hist_id,
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
            content_key: new_elem_content_id,
        };

        // Get meta to get id of current hist entry
        let mut tbl_meta = txn.open_table(db::META_TABLE)?;
        let meta = tbl_meta.get(())?.ok_or(eyre!("no map meta"))?.value();

        // Create history entry
        let hist_key = id_gen.generate();

        let new_node = HistNode {
            timestamp: history::new_timestamp(),
            parent_key: Some(meta.hist_key),
            child_keys: Vec::default(),
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
        tbl_meta.insert((), Meta { hist_key, ..meta })?;

        info!("yey done!");
    }

    txn.commit()?;

    Ok(())
}

// Committing things to the map...
#[derive(Event)]
pub enum Commit {
    Create {
        name: String,
        content: ErasedContent,
    },
    Rename {
        element_key: Id,
        new_name: String,
    },
    Modify {
        element_key: Id,
        new_content: ErasedContent,
    },
    Remove {
        element_key: Id,
    },
    // - Paste.. for copy-pasting
    // - Multi edit?
    //  - would be nice to move multiple elements same distance.
    //      - either specify each element with before-after or a relative distance for all elements to move..
    //  - would also be nice to multi apply surfaces (materials).
    //
    //  How to structure all this..
    //  - Commit(Vec<Change>)?
    //      - git-inspired
}

fn commit_to_map(
    map_db: Res<Db>,
    elem_lookup: Res<ElementLookup>,
    q_elements: Query<&Element>,
    mut id_gen: ResMut<IdGen>,
    mut commits: EventReader<Commit>,
) -> Result {
    for commit in commits.read() {
        match commit {
            Commit::Create { name, content } => {
                let tx = map_db.begin_write()?;
                {
                    // Insert the content
                    let new_content_key = id_gen.generate();
                    match content.role() {
                        ElementRole::Brush => {
                            let brush: &Brush = content.downcast_ref()?;
                            let mut tbl_brushes = tx.open_table(elements::CONTENT_TABLE_BRUSH)?;
                            tbl_brushes.insert(new_content_key, brush)?;
                        }
                        ElementRole::Light => todo!(),
                    }

                    // Construct the element using content key
                    let new_elem_key = id_gen.generate();
                    let new_elem = Element {
                        name: name.clone(),
                        role: content.role(),
                        content_key: new_content_key,
                    };

                    // Insert the history entry
                    let mut tbl_hist = tx.open_table(history::HIST_TABLE)?;
                    let mut tbl_meta = tx.open_table(META_TABLE)?;
                    let meta = tbl_meta.get(())?.unwrap().value();
                    let prev_hist_key = meta.hist_key;

                    let new_hist_key = id_gen.generate();
                    tbl_hist.insert(
                        new_hist_key,
                        HistNode {
                            timestamp: history::new_timestamp(),
                            parent_key: Some(prev_hist_key),
                            child_keys: Vec::new(),
                            change: Change::Element {
                                key: new_elem_key,
                                delta: Delta::Create {
                                    element: new_elem,
                                    content_key: new_content_key,
                                },
                            },
                        },
                    )?;

                    tbl_meta.insert(
                        (),
                        Meta {
                            hist_key: new_hist_key,
                            ..meta
                        },
                    )?;
                }
                tx.commit()?;
            }
            Commit::Rename {
                element_key,
                new_name,
            } => {
                let tx = map_db.begin_write()?;
                {
                    let element = q_elements.get(elem_lookup.find(element_key)?)?;

                    // Insert the history entry
                    let mut tbl_hist = tx.open_table(history::HIST_TABLE)?;
                    let mut tbl_meta = tx.open_table(META_TABLE)?;
                    let meta = tbl_meta.get(())?.unwrap().value();
                    let prev_hist_key = meta.hist_key;

                    let new_hist_key = id_gen.generate();
                    tbl_hist.insert(
                        new_hist_key,
                        HistNode {
                            timestamp: history::new_timestamp(),
                            parent_key: Some(prev_hist_key),
                            child_keys: Vec::new(),
                            change: Change::Element {
                                key: *element_key,
                                delta: Delta::Modify {
                                    then: element.clone(),
                                    now: Element {
                                        name: new_name.clone(),
                                        role: element.role,
                                        content_key: element.content_key,
                                    },
                                },
                            },
                        },
                    )?;

                    tbl_meta.insert(
                        (),
                        Meta {
                            hist_key: new_hist_key,
                            ..meta
                        },
                    )?;
                }
                tx.commit()?;
            }
            Commit::Modify {
                element_key,
                new_content,
            } => todo!(),
            Commit::Remove { element_key } => todo!(),
        }
    }
    // TODO: Collect result for each commit, so that further commits can still be run in case of failure.
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

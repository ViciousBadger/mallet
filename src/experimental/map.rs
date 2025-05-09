pub mod changes;
pub mod db;
pub mod elements;
pub mod history;
pub mod states;

use bevy::{
    input::common_conditions::input_just_pressed, platform::collections::HashMap, prelude::*,
};
use thiserror::Error;

use crate::{
    core::map::brush::{Brush, BrushBounds},
    experimental::map::{
        changes::{apply_pending_changes, Create, PendingChanges},
        db::{Db, Meta, TBL_META},
        elements::ElemId,
        history::HistNode,
    },
    id::{Id, IdGen},
};

fn new_test_map(mut commands: Commands, mut id_gen: ResMut<IdGen>) -> Result {
    let map = Db::new_temp();
    let tx = map.begin_write()?;

    {
        // Initial empty state

        let mut tbl_states = tx.open_table(states::TBL_STATES)?;
        let initial_state_id = id_gen.generate();
        tbl_states.insert(initial_state_id, states::State::empty())?;

        // Initial history node
        let mut tbl_hist = tx.open_table(history::TBL_HIST_NODES)?;
        let initial_hist_id = id_gen.generate();
        tbl_hist.insert(
            initial_hist_id,
            HistNode {
                timestamp: history::new_timestamp(),
                parent_id: None,
                child_ids: Vec::default(),
                state_id: initial_state_id,
            },
        )?;

        let mut tbl_meta = tx.open_table(TBL_META)?;
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

fn new_thing(mut changes: ResMut<PendingChanges>) {
    changes.push_many(vec![
        Create {
            name: "first brush".to_string(),
            params: Brush {
                bounds: BrushBounds {
                    start: Vec3::ZERO,
                    end: Vec3::ONE,
                },
            },
        },
        Create {
            name: "second brush".to_string(),
            params: Brush {
                bounds: BrushBounds {
                    start: Vec3::ZERO,
                    end: Vec3::ONE,
                },
            },
        },
    ]);
    changes.push_single(Create {
        name: "third brush (in its own change set)".to_string(),
        params: Brush {
            bounds: BrushBounds {
                start: Vec3::ZERO,
                end: Vec3::ONE,
            },
        },
    });

    info!("ok, pushed some changes.");
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
    q_added_ids: Query<(&ElemId, Entity), Added<ElemId>>,
    mut q_removed_ids: RemovedComponents<ElemId>,
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
    app.init_resource::<PendingChanges>();
    app.add_systems(Startup, new_test_map);
    app.add_systems(
        Update,
        (
            new_thing.run_if(input_just_pressed(KeyCode::KeyF)),
            track_element_ids,
        ),
    );
    app.add_systems(Last, apply_pending_changes);
}

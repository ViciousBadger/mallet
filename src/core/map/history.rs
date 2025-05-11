use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use redb::TableDefinition;
use serde::{Deserialize, Serialize};

use crate::{
    core::{
        binds::Binding,
        db::{Db, Meta, Typed, TBL_META},
        map::states::RestoreState,
    },
    id::Id,
};

pub const TBL_HIST_NODES: TableDefinition<Id, Typed<HistNode>> = TableDefinition::new("hist_nodes");

#[derive(Serialize, Deserialize, Debug)]
pub struct HistNode {
    pub timestamp: i64,
    pub parent_id: Option<Id>,
    pub child_ids: Vec<Id>,
    pub state_id: Id,
}

pub fn new_timestamp() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}

#[derive(Event)]
struct JumpToHistoryNode {
    pub id: Id,
}

fn jump_to_hist_node(
    trigger: Trigger<JumpToHistoryNode>,
    db: Res<Db>,
    mut commands: Commands,
) -> Result {
    let reader = db.begin_read()?;
    let hist_node = reader
        .open_table(TBL_HIST_NODES)?
        .get(trigger.id)?
        .unwrap()
        .value();
    commands.trigger(RestoreState {
        id: hist_node.state_id,
        fresh_map: false,
    });
    commands.trigger(UpdateCurrentHistNode(trigger.id));

    Ok(())
}

#[derive(Event)]
pub struct UpdateCurrentHistNode(pub Id);

fn update_cur_hist_node(trigger: Trigger<UpdateCurrentHistNode>, db: Res<Db>) -> Result {
    let reader = db.begin_read()?;
    let meta = reader.open_table(TBL_META)?.get(())?.unwrap().value();

    let writer = db.begin_write()?;
    writer.open_table(TBL_META)?.insert(
        (),
        Meta {
            hist_node_id: trigger.0,
            ..meta
        },
    )?;
    writer.commit()?;
    Ok(())
}

fn undo(db: Res<Db>, mut commands: Commands) -> Result {
    let reader = db.begin_read()?;
    let meta = reader.open_table(TBL_META)?.get(())?.unwrap().value();
    let cur_hist_node = reader
        .open_table(TBL_HIST_NODES)?
        .get(meta.hist_node_id)?
        .unwrap()
        .value();
    if let Some(parent_hist_node_id) = cur_hist_node.parent_id {
        commands.trigger(JumpToHistoryNode {
            id: parent_hist_node_id,
        });
        info!("doing an undo");
    } else {
        info!("not doing an undo - no parent on this hist node");
    }
    Ok(())
}

fn redo(db: Res<Db>, mut commands: Commands) -> Result {
    let reader = db.begin_read()?;
    let meta = reader.open_table(TBL_META)?.get(())?.unwrap().value();
    let cur_hist_node = reader
        .open_table(TBL_HIST_NODES)?
        .get(meta.hist_node_id)?
        .unwrap()
        .value();
    if let Some(last_child_if_hist_node) = cur_hist_node.child_ids.last() {
        commands.trigger(JumpToHistoryNode {
            id: *last_child_if_hist_node,
        });
        info!("doing a redo");
    } else {
        info!("not doing a redo - no children on this hist node");
    }
    Ok(())
}

pub fn plugin(app: &mut App) {
    app.add_observer(jump_to_hist_node);
    app.add_observer(update_cur_hist_node);
    app.add_systems(
        Update,
        (
            undo.run_if(input_just_pressed(Binding::Undo)),
            redo.run_if(input_just_pressed(Binding::Redo)),
        ),
    );
}

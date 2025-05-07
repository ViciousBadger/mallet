pub mod db;

use bevy::{input::common_conditions::input_pressed, prelude::*};
use color_eyre::eyre::eyre;
use redb::ReadableTable;

use crate::{
    core::map::{
        brush::{Brush, BrushBounds},
        light::{Light, LightType},
    },
    experimental::map::db::{
        new_timestamp, Action, Db, Delta, Element, ElementRole, HistNode, Meta, HIST_TABLE,
        META_TABLE,
    },
    id::IdGen,
};

fn new_test_map(mut commands: Commands, mut id_gen: ResMut<IdGen>) -> Result {
    let map = Db::new_temp();
    let tx = map.begin_write()?;

    {
        let mut tbl_hist = tx.open_table(HIST_TABLE)?;
        let initial_hist_id = id_gen.generate();
        tbl_hist.insert(
            initial_hist_id,
            HistNode::MapInit {
                timestamp: new_timestamp(),
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
        let mut tbl_brush = txn.open_table(db::CONTENT_TABLE_BRUSH)?;

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

        let new_node = HistNode::Node {
            parent_key: meta.hist_key,
            timestamp: new_timestamp(),
            action: Action::Delta {
                element_id: new_elem_id,
                delta: Delta::Create {
                    content_key: new_elem_content_id,
                    element: new_elem.clone(),
                },
            },
        };

        let mut hist = txn.open_table(db::HIST_TABLE)?;
        hist.insert(&hist_key, &new_node)?;

        // Set meta to point to newest history entry
        tbl_meta.insert((), Meta { hist_key, ..meta })?;

        info!("yey done!");
    }

    txn.commit()?;

    Ok(())
}

fn new_thing() {
    //map_mods.write(
}
fn undo() {}
fn redo() {}

pub fn plugin(app: &mut App) {
    app.init_resource::<IdGen>();
    app.add_systems(Startup, new_test_map);
    app.add_systems(Update, (push_test.run_if(input_pressed(KeyCode::KeyF)),));
}

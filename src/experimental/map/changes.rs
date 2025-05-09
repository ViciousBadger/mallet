use bevy::prelude::*;

// #[derive(Debug)]
// pub enum Change {
//     Create {
//         name: String,
//         params: DynElemParams,
//     },
//     ModMeta {
//         elem_id: Id,
//         modified: ElemMeta,
//     },
//     ModParams {
//         elem_id: Id,
//         modified: DynElemParams,
//     },
//     Remove {
//         elem_id: Id,
//     },
// }
//
pub trait Change: std::fmt::Debug + Send + Sync {
    fn apply_to_world(self, world: &mut World);
}

#[derive(Debug)]
pub struct ChangeSet {
    changes: Vec<Box<dyn Change>>,
}

#[derive(Resource, Default)]
pub struct PendingChanges(Vec<ChangeSet>);

impl PendingChanges {
    pub fn push_single<T>(&mut self, change: T)
    where
        T: Change + 'static,
    {
        self.0.push(ChangeSet {
            changes: vec![Box::new(change)],
        });
    }

    pub fn push_many<T>(&mut self, changes: Vec<T>)
    where
        T: Change + 'static,
    {
        let mut boxed_changes: Vec<Box<dyn Change>> = Vec::new();
        for c in changes {
            boxed_changes.push(Box::new(c));
        }
        self.0.push(ChangeSet {
            changes: boxed_changes,
        });
    }
}

pub fn apply_pending_changes(mut pending_changes: ResMut<PendingChanges>, mut commands: Commands) {
    let change_sets: Vec<ChangeSet> = pending_changes.0.drain(..).collect();

    if !change_sets.is_empty() {
        info!("collected {} change sets", change_sets.len());
    }

    for change_set in change_sets {
        commands.run_system_cached_with(apply_change_set_and_snapshot, change_set);
    }
}

fn apply_change_set_and_snapshot(change_set: In<ChangeSet>, world: &mut World) {
    info!("apply change set: {:?}", change_set);
    let change_set = change_set.0;
    // Step 1: apply to world.
    for change in change_set.changes {
        change.apply_to_world(world);
        // world
        //     .run_system_cached_with(apply_single_change, change)
        //     .unwrap();
    }

    // Step 2: create a state resource and run the snapshot schedule. other systems will fill out state.
    todo!();

    // Step 3: insert new state into db and create a history node.
    todo!();
}

// fn apply_single_change(change: In<Change>, world: &mut World) {
//     match change.0 {
//         Change::Create { name, params } => {
//             let new_id = world.resource_mut::<IdGen>().generate();
//             let new_entity = world.spawn((
//                 ElemId::new(new_id),
//                 ElemMeta {
//                     name,
//                     role: params.role(),
//                 },
//             ));
//             info!("downcast?");
//             params.insert_with_exclusive_world(new_entity);
//             info!("applied create");
//         }
//         Change::ModMeta { elem_id, modified } => {
//             let entity_id = world
//                 .resource_mut::<ElementLookup>()
//                 .find(&elem_id)
//                 .unwrap();
//             let mut entity = world.entity_mut(entity_id);
//             entity.insert(modified);
//             info!("applied modmeta");
//         }
//         Change::ModParams { elem_id, modified } => {
//             let entity_id = world
//                 .resource_mut::<ElementLookup>()
//                 .find(&elem_id)
//                 .unwrap();
//             let entity = world.entity_mut(entity_id);
//             modified.insert_with_exclusive_world(entity);
//             info!("applied modparams");
//         }
//         Change::Remove { elem_id } => {
//             let entity_id = world
//                 .resource_mut::<ElementLookup>()
//                 .find(&elem_id)
//                 .unwrap();
//             world.despawn(entity_id);
//             info!("applied remove");
//         }
//     }
// }

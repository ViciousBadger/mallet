use bevy::prelude::*;
use daggy::{Dag, NodeIndex, Walker};
use serde::{Deserialize, Serialize};

use crate::{core::map::TypedMapNode, util::Id};

/// A "symmetric" map change that stores enough data to be reversable.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MapDelta {
    Nop,
    AddNode {
        id: Id,
        name: String,
        node: TypedMapNode,
    },
    ModifyNode {
        id: Id,
        before: TypedMapNode,
        after: TypedMapNode,
    },
    RenameNode {
        id: Id,
        before: String,
        after: String,
    },
    RemoveNode {
        id: Id,
        name: String,
        node: TypedMapNode,
    },
}

impl MapDelta {
    pub fn reverse_of(&self) -> MapDelta {
        match self {
            MapDelta::Nop => MapDelta::Nop,
            MapDelta::AddNode { id, name, node } => MapDelta::RemoveNode {
                id: *id,
                name: name.clone(),
                node: node.clone(),
            },
            MapDelta::ModifyNode { id, before, after } => MapDelta::ModifyNode {
                id: *id,
                before: after.clone(),
                after: before.clone(),
            },
            MapDelta::RenameNode { id, before, after } => MapDelta::RenameNode {
                id: *id,
                before: after.clone(),
                after: before.clone(),
            },
            MapDelta::RemoveNode { id, name, node } => MapDelta::AddNode {
                id: *id,
                name: name.clone(),
                node: node.clone(),
            },
        }
    }
}

#[derive(Resource, Serialize, Deserialize)]
pub struct MapHistory {
    cur_delta_idx: NodeIndex<u32>,
    delta_graph: Dag<MapDelta, ()>,
}

impl Default for MapHistory {
    fn default() -> Self {
        let mut graph: Dag<MapDelta, ()> = Dag::new();
        let root_node = graph.add_node(MapDelta::Nop);
        Self {
            cur_delta_idx: root_node,
            delta_graph: graph,
        }
    }
}

impl MapHistory {
    pub fn push(&mut self, new_delta: MapDelta) {
        let (_new_edge, new_node) = self
            .delta_graph
            .add_child(self.cur_delta_idx, (), new_delta);
        self.cur_delta_idx = new_node;
    }

    pub fn can_undo(&self) -> bool {
        self.delta_graph
            .parents(self.cur_delta_idx)
            .walk_next(&self.delta_graph)
            .is_some()
    }

    #[must_use]
    pub fn undo(&mut self) -> MapDelta {
        let (_, parent_node_idx) = self
            .delta_graph
            .parents(self.cur_delta_idx)
            .walk_next(&self.delta_graph)
            .expect("Use can_undo() to check first");

        let reverse_of_current = self
            .delta_graph
            .node_weight(self.cur_delta_idx)
            .unwrap()
            .reverse_of();

        self.cur_delta_idx = parent_node_idx;
        reverse_of_current
    }

    pub fn can_redo(&self) -> bool {
        self.delta_graph
            .children(self.cur_delta_idx)
            .walk_next(&self.delta_graph)
            .is_some()
    }

    #[must_use]
    pub fn redo(&mut self) -> MapDelta {
        // Assume the last child node to be most relevant change tree.
        let (_, child_node_idx) = self
            .delta_graph
            .children(self.cur_delta_idx)
            .walk_next(&self.delta_graph)
            .expect("Use can_redo() to check first");

        let child_delta = self
            .delta_graph
            .node_weight(child_node_idx)
            .unwrap()
            .clone();

        self.cur_delta_idx = child_node_idx;
        child_delta
    }
}

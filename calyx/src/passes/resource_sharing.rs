use super::sharing_components::ShareComponents;
use crate::analysis;
use crate::errors::CalyxResult;
use crate::ir::{self, traversal::Named, CloneName, RRC};
use ir::traversal::ConstructVisitor;
use std::collections::{HashMap, HashSet};

/// Rewrites groups to share cells marked with the "share" attribute
/// when the groups are guaranteed to never run in parallel.
pub struct ResourceSharing {
    /// Mapping from the name of a group to the cells that it uses.
    used_cells_map: HashMap<ir::Id, Vec<ir::Id>>,

    /// This is used to rewrite all uses of `old_cell` with `new_cell` in the group.
    rewrites: Vec<(RRC<ir::Cell>, RRC<ir::Cell>)>,

    /// Set of shareable components.
    shareable_components: HashSet<ir::Id>,
}

impl Named for ResourceSharing {
    fn name() -> &'static str {
        "resource-sharing"
    }

    fn description() -> &'static str {
        "shares resources between groups that don't execute in parallel"
    }
}

impl ConstructVisitor for ResourceSharing {
    fn from(ctx: &ir::Context) -> CalyxResult<Self> {
        let mut shareable_components = HashSet::new();
        // add share=1 primitives to the shareable_components set
        for prim in ctx.lib.signatures() {
            if let Some(&1) = prim.attributes.get("share") {
                shareable_components.insert(prim.name.clone());
            }
        }
        // add share=1 user defined components to the shareable_components set
        for comp in &ctx.components {
            if let Some(&1) = comp.attributes.get("share") {
                shareable_components.insert(comp.name.clone());
            }
        }
        Ok(ResourceSharing {
            used_cells_map: HashMap::new(),
            rewrites: Vec::new(),
            shareable_components,
        })
    }

    fn clear_data(&mut self) {
        self.used_cells_map = HashMap::new();
        self.rewrites = Vec::new();
    }
}

impl ShareComponents for ResourceSharing {
    fn initialize(
        &mut self,
        component: &ir::Component,
        _sigs: &ir::LibrarySignatures,
    ) {
        self.used_cells_map = component
            .groups
            .iter()
            .map(|group| {
                (
                    group.clone_name(),
                    analysis::ReadWriteSet::uses(&group.borrow().assignments)
                        .filter(|cell| self.cell_filter(&cell.borrow()))
                        .map(|cell| cell.clone_name())
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
    }

    fn lookup_group_conflicts(&self, group_name: &ir::Id) -> Vec<ir::Id> {
        self.used_cells_map[group_name].clone()
    }

    fn cell_filter(&self, cell: &ir::Cell) -> bool {
        if let Some(type_name) = cell.type_name() {
            self.shareable_components.contains(type_name)
        } else {
            false
        }
    }

    fn custom_conflicts<F>(&self, _comp: &ir::Component, mut add_conflicts: F)
    where
        F: FnMut(Vec<ir::Id>),
    {
        for used in self.used_cells_map.values() {
            add_conflicts(used.clone())
        }
    }

    fn set_rewrites(&mut self, rewrites: Vec<(RRC<ir::Cell>, RRC<ir::Cell>)>) {
        self.rewrites = rewrites;
    }

    fn get_rewrites(&self) -> &[(RRC<ir::Cell>, RRC<ir::Cell>)] {
        &self.rewrites
    }
}

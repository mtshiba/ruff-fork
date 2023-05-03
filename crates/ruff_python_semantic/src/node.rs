use std::num::TryFromIntError;
use std::ops::{Index, IndexMut};

use rustc_hash::FxHashMap;
use rustpython_parser::ast::Stmt;

use ruff_python_ast::types::RefEquality;

/// Id uniquely identifying a statement in a program.
///
/// Using a `u32` is sufficient because Ruff only supports parsing documents with a size of max `u32::max`
/// and it is impossible to have more statements than characters in the file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct NodeId(u32);

impl TryFrom<usize> for NodeId {
    type Error = TryFromIntError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(Self(u32::try_from(value)?))
    }
}

impl From<NodeId> for usize {
    fn from(value: NodeId) -> Self {
        value.0 as usize
    }
}

#[derive(Debug)]
struct Node<'a> {
    /// The statement this node represents.
    stmt: &'a Stmt,
    /// The ID of the parent of this node, if any.
    parent: Option<NodeId>,
    /// The depth of this node in the tree.
    depth: u32,
}

/// The nodes of a program indexed by [`NodeId`]
#[derive(Debug, Default)]
pub struct Nodes<'a> {
    nodes: Vec<Node<'a>>,
    node_to_id: FxHashMap<RefEquality<'a, Stmt>, NodeId>,
}

impl<'a> Nodes<'a> {
    /// Pushes a new node onto the node tree and returns its unique id.
    pub fn push_node(&mut self, stmt: &'a Stmt, parent: Option<NodeId>) -> NodeId {
        let next_id = NodeId::try_from(self.nodes.len()).unwrap();
        self.node_to_id.insert(RefEquality(stmt), next_id);
        self.nodes.push(Node {
            stmt,
            parent,
            depth: parent.map_or(0, |parent| self.nodes[usize::from(parent)].depth + 1),
        });
        next_id
    }

    /// Returns the [`NodeId`] of the given node.
    #[inline]
    pub fn node_id(&self, node: &'a Stmt) -> Option<NodeId> {
        self.node_to_id.get(&RefEquality(node)).copied()
    }

    /// Return the [`NodeId`] of the parent node.
    #[inline]
    pub fn parent_id(&self, node_id: NodeId) -> Option<NodeId> {
        self.nodes[usize::from(node_id)].parent
    }

    /// Return the depth of the node.
    #[inline]
    pub fn depth(&self, node_id: NodeId) -> u32 {
        self.nodes[usize::from(node_id)].depth
    }

    /// Return the parent of the given node.
    pub fn parent(&self, node: &'a Stmt) -> Option<&'a Stmt> {
        let node_id = self.node_to_id.get(&RefEquality(node))?;
        let parent_id = self.nodes[usize::from(*node_id)].parent?;
        Some(self.nodes[usize::from(parent_id)].stmt)
    }

    /// Returns an iterator over all [`NodeId`] ancestors, starting from the given [`NodeId`].
    pub fn ancestor_ids(&self, node_id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        std::iter::successors(Some(node_id), |&node_id| {
            self.nodes[usize::from(node_id)].parent
        })
    }
}

impl<'a> Index<NodeId> for Nodes<'a> {
    type Output = &'a Stmt;

    fn index(&self, index: NodeId) -> &Self::Output {
        &self.nodes[usize::from(index)].stmt
    }
}

impl<'a> IndexMut<NodeId> for Nodes<'a> {
    fn index_mut(&mut self, index: NodeId) -> &mut Self::Output {
        &mut self.nodes[usize::from(index)].stmt
    }
}

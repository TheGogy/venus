pub mod helpers;
pub mod iterative_deepening;
pub mod negamax;
pub mod pv;
pub mod stack;

/// Node Type trait.
/// PV:   Whether this node is on the principal variation.
/// RT: Whether this node is the root of the search tree.
/// Next: The type of node that a PV search will be from this node.
pub trait NodeType {
    const PV: bool;
    const RT: bool;
    type Next: NodeType;
}

struct Root; // Root node.
struct OnPV; // PV node (non-null search window)
struct OffPV; // Non-PV node (null search window)

impl NodeType for Root {
    const PV: bool = true;
    const RT: bool = true;
    type Next = OnPV;
}

impl NodeType for OnPV {
    const PV: bool = true;
    const RT: bool = false;
    type Next = Self;
}

impl NodeType for OffPV {
    const PV: bool = false;
    const RT: bool = false;
    type Next = Self;
}

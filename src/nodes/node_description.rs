#[derive(Debug, Default, Clone)]
/// Describes a node. 
/// Please note that this separates meta data like a node's name from its implementation. 
/// (see also [crate::nodes::node::Node]).
pub struct NodeDescription {
    pub name: String,
    pub description: String,
    pub kind: String
}

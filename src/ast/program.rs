use super::node::Node;

/// Parsed Ember program.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Program {
    /// Top-level definitions.
    pub definitions: Vec<Node>,
    /// Main executable nodes.
    pub main: Vec<Node>,
}

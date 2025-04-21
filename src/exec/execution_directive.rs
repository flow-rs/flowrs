use crate::flow::flow_types::NodeIOIndex;

#[derive(Debug, Clone)]
pub enum NodeExecutionDirective {
    /// The node is ready for immediate next execution
    ContinueImmediately,

    /// Wait for these inputs before executing again
    WaitForInputs(Vec<NodeIOIndex>),

    /// Pause indefinitely (e.g., until externally resumed)
    Suspend,
}

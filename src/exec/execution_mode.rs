pub enum ExecutionMode {
    // "normal" FBP execution mode where nodes operate continuously and react to incoming messages
    Continuous,
    // synchronized execution mode where nodes wait for a synchronization message before executing a single execution step
    Synchronized,
}

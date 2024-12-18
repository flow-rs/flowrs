#[cfg(test)]
mod test {
    use flowrs::comm::communication::Communicator;
    use flowrs::comm::communication::NodeCommunicator;
    use flowrs::comm::thread_communicator::ThreadCommunicator;
    use flowrs::connection::Input;
    use flowrs::connection::Output;
    use flowrs::exec::execution::ExecutionContext;
    use flowrs::exec::execution::StandardExecutor;
    use flowrs::flow_impl::Flow;
    use flowrs_std::add::AddNode;
    use flowrs_std::value::ValueNode;

    #[test]
    fn test_simple_flow_execution() {
        //Define communicators
        let mut node1_comm = NodeCommunicator::ThreadComm(
            ThreadCommunicator::<u32>::new().expect("should construct"),
        );
        let mut node2_comm = NodeCommunicator::ThreadComm(
            ThreadCommunicator::<u32>::new().expect("should construct"),
        );
        let output_comm = NodeCommunicator::ThreadComm(
            ThreadCommunicator::<u32>::new().expect("should construct"),
        );

        //Define nodes
        let number_node_1: ValueNode<u32> = ValueNode::<u32>::new(3, node1_comm.clone_send());
        let number_node_2: ValueNode<u32> = ValueNode::<u32>::new(2, node2_comm.clone_send());
        let add_node = AddNode::<u32, u32, u32>::new(
            node1_comm.move_recv().expect("should move"),
            node2_comm.move_recv().expect("should move"),
            output_comm.clone_send(),
        );
        //Define flow
        let mut flow = Flow::new_empty();
        flow.add_node_with_id(number_node_1, 1);
        flow.add_node_with_id(number_node_2, 2);
        flow.add_node_with_id(add_node, 3);
        flow.connect_nodes(1, 3, 0, 0);
        flow.connect_nodes(2, 3, 0, 1);
        assert_eq!(flow.num_nodes(), 3);
        // //Define executor
        // let executor = StandardExecutor::new();
        // ExecutionContext::new(executor, flow)
    }
}

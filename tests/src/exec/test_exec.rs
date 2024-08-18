#[cfg(test)]
mod test {
    use flowrs::flow_impl::Flow;
    use flowrs::node::ChangeObserver;
    use flowrs_std::add::AddNode;
    use flowrs_std::value::ValueNode;

    #[test]
    fn test_simple_flow_execution() {
        //Define ChangeObserver
        let change_observer = ChangeObserver::new();
        //Define nodes
        let number_node_1: ValueNode<u32> = ValueNode::<u32>::new(3, Some(&change_observer));
        let number_node_2: ValueNode<u32> = ValueNode::<u32>::new(2, Some(&change_observer));
        let add_node: AddNode<u32, u32, u32> =
            AddNode::<u32, u32, u32>::new(Some(&change_observer));
        //Define flow
        let mut flow = Flow::new_empty();
        flow.add_node_with_id(number_node_1, 1);
        flow.add_node_with_id(number_node_2, 2);
        flow.add_node_with_id(add_node, 3);
        assert_eq!(flow.num_nodes(), 3);
        //Define executor
    }
}

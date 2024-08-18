#[cfg(test)]
mod test {
    use flowrs::{self, node::ChangeObserver};
    //use flowrs::nodes::node::{ChangeObserver, Context};
    use flowrs_std::value::ValueNode;

    #[test]
    fn test_simple_flow_execution() {
        //Define ChangeObserver
        let change_observer = ChangeObserver::new();
        //Define nodes
        let number_node_1 = ValueNode::<u32>::new(3, Some(&change_observer));
        assert!(true);
    }
}

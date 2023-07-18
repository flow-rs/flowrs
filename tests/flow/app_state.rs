#[cfg(test)]
mod app_state {
    use std::sync::mpsc::{Sender, Receiver, channel};

    use flow::app_state::{AppState, FlowType};

    #[test]
    fn should_deserialize_empty_state() {
        let json_str = r#"
    {
        "nodes": [],
        "edges": []
    }
    "#;

        let json_data: AppState = serde_json::from_str(json_str).unwrap();
        assert!(json_data.nodes.len() == 0);
    }

    #[test]
    #[should_panic(expected = r#"Addition of JSON values of type String("NaN") and Number(12) is not supported."#)]
    fn should_panic_on_invalid_addition() {
        let json_str = r#"
        {
            "nodes": [
                {
                    "name": "lhs",
                    "kind": "nodes.basic",
                    "props": "NaN"
                },
                {
                    "name": "rhs",
                    "kind": "nodes.basic",
                    "props": 12
                },
                {
                    "name": "add",
                    "kind": "nodes.arithmetics.add",
                    "props": null
                }
            ],
            "edges": [
                {
                    "input": "lhs",
                    "output": "add",
                    "index": 0
                },
                {
                    "input": "rhs",
                    "output": "add",
                    "index": 1
                }
            ]
        }
        "#;
        let mut app_state: AppState = serde_json::from_str(json_str).unwrap();
        app_state.run();
    }

    #[test]
    fn should_deserialize_non_empty_state() {
        let json_str = r#"
        {
            "nodes": [
                {
                    "name": "lhs",
                    "kind": "nodes.basic",
                    "props": 30
                },
                {
                    "name": "rhs",
                    "kind": "nodes.basic",
                    "props": 12
                },
                {
                    "name": "add",
                    "kind": "nodes.arithmetics.add",
                    "props": null
                }
            ],
            "edges": [
                {
                    "input": "lhs",
                    "output": "add",
                    "index": 0
                },
                {
                    "input": "rhs",
                    "output": "add",
                    "index": 1
                }
            ]
        }
        "#;
        let (mock_s, mock_r): (Sender<FlowType>, Receiver<FlowType>) = channel();
        let mut app_state: AppState = serde_json::from_str(json_str).unwrap();
        assert!(app_state.nodes.len() == 3);
        app_state.get_node("add").chain(vec![mock_s]);
        app_state.run();
        let dyn_output = mock_r.try_recv().unwrap().0;
        let output = dyn_output.downcast::<f64>().unwrap();
        assert!(*output == 42.0)
    }
}

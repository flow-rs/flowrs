#[cfg(test)]
mod app_state {
    use flow::app_state::AppState;


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
    fn should_deserialize_non_empty_state() {
        let json_str = r#"
        {
            "nodes": [
                {
                    "name": "lhs",
                    "kind": "nodes.basic",
                    "props": 12
                },
                {
                    "name": "rhs",
                    "kind": "nodes.basic",
                    "props": 30
                },
                {
                    "name": "add",
                    "kind": "nodes.arithmetics.add",
                    "props": {"none": "Undefined"}
                },
                {
                    "name": "debug",
                    "kind": "nodes.debug",
                    "props": {"none": "Undefined"}
                }
            ],
            "edges": [
                {
                    "source": {"node": "lhs", "index": 0},
                    "dest": {"node": "add", "index": 0}
                },
                {
                    "source": {"node": "rhs", "index": 0},
                    "dest": {"node": "add", "index": 1}
                },
                {
                    "source": {"node": "add", "index": 0},
                    "dest": {"node": "debug", "index": 0}
                }
            ]
        }
        "#;

        let mut app_state: AppState = serde_json::from_str(json_str).unwrap();
        assert!(app_state.nodes.len() == 4);
        app_state.run();
    }
}

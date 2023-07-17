use std::{collections::HashMap, ops::Add, sync::Arc};

use serde::{Deserialize, Deserializer};

use crate::{add::AddNode, basic::BasicNode, connection::ConnectError, job::Context, Node};

#[derive(Debug, Clone, Deserialize)]
pub enum FlowType {
    I32(i32),
    String(String),
    Vec(Vec<FlowType>),
    Undefined,
}

impl Add for FlowType {
    type Output = FlowType;

    fn add(self, rhs: Self) -> Self::Output {
        match self {
            FlowType::I32(x) => match rhs {
                FlowType::I32(y) => FlowType::I32(x + y),
                _ => panic!("Operation not supported"),
            },
            _ => panic!("Operation not supported"),
        }
    }
}

pub struct AppState {
    pub nodes: HashMap<String, Box<dyn Node<FlowType, FlowType>>>,
    pub context: Arc<Context>,
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            nodes: HashMap::new(),
            context: Arc::new(Context {}),
        }
    }

    pub fn add_node(&mut self, name: &str) -> String {
        let node = Box::new(AddNode::new(name, self.context.clone()));
        let _ = &self.nodes.insert(name.to_string(), node);
        name.to_owned()
    }

    pub fn add_basic(&mut self, name: &str, props: FlowType) -> String {
        let node = Box::new(BasicNode::new(name, self.context.clone(), props));
        let _ = &self.nodes.insert(name.to_string(), node);
        name.to_owned()
    }

    pub fn chain_at(
        &mut self,
        lhs: String,
        rhs: String,
        index: usize,
    ) -> Result<(), ConnectError<FlowType>> {
        let lhs = self.nodes.get(&lhs).unwrap().clone();
        let rhs = self.nodes.get(&rhs).unwrap().clone();
        lhs.chain(vec![rhs.input_at(index)?]);
        Ok(())
    }
}

#[derive(Deserialize)]
struct JsonNode {
    name: String,
    kind: String,
    props: HashMap<String, FlowType>,
    //inputs: Vec<Queue>,
    //outputs: Vec<Queue>,
}

#[derive(Deserialize)]
struct Queue {
    //ty: String,
}

#[derive(Deserialize)]
struct JsonEdge {
    input: String,
    index: usize,
    output: String,
}

#[derive(Deserialize)]
struct JsonData {
    nodes: Vec<JsonNode>,
    edges: Vec<JsonEdge>,
}

impl<'de> Deserialize<'de> for AppState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut app_state = AppState::new();
        let json_data: JsonData = JsonData::deserialize(deserializer)?;
        json_data.nodes.iter().for_each(|node| {
            match node.kind.as_str() {
                "nodes.arithmetics.add" => app_state.add_node(&node.name),
                "nodes.basic" => {
                    app_state.add_basic(&node.name, node.props.get("value").unwrap().to_owned())
                }
                value => panic!("Node kind `{}` is not yet supported.", value),
            };
        });
        json_data.edges.iter().for_each(|edge| {
            let _ = app_state.chain_at(edge.input.clone(), edge.output.clone(), edge.index);
        });

        Ok(app_state)
    }
}

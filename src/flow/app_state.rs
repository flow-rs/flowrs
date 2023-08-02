use std::{any::Any, collections::HashMap, ops::Add, sync::Arc};

use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::{
    add::AddNode,
    basic::BasicNode,
    connection::{connect, ConnectError, Input, Output, RuntimeConnectable},
    node::Node,
    node::{Context, State},
    nodes::debug::DebugNode,
};

#[derive(Clone, Debug)]
pub struct FlowType(pub Arc<dyn Any + Send + Sync>);

pub trait RuntimeNode: Node + RuntimeConnectable {}
impl<T> RuntimeNode for T where T: Node + RuntimeConnectable {}
// This implementation gives some control over which types should be
// addable throughout the entire flow. As of now only homogenious types
// allow addition.
// As the Properties of a Node can be any JSON value, the addition of
// such properties is limited to numbers (casted as float), lists and
// strings (both concatinated upon addition).
impl Add for FlowType {
    type Output = FlowType;

    fn add(self, rhs: Self) -> Self::Output {
        if let Some(lhs) = self.0.downcast_ref::<i64>() {
            if let Some(rhs) = rhs.0.downcast_ref::<i64>() {
                return FlowType(Arc::new(lhs + rhs));
            }
        }
        if let Some(lhs) = self.0.downcast_ref::<i32>() {
            if let Some(rhs) = rhs.0.downcast_ref::<i32>() {
                return FlowType(Arc::new(lhs + rhs));
            }
        }
        if let Some(lhs) = self.0.downcast_ref::<String>() {
            if let Some(rhs) = rhs.0.downcast_ref::<String>() {
                let mut res = lhs.clone();
                res.push_str(rhs);
                return FlowType(Arc::new(res));
            }
        }
        if let Some(lhs) = self.0.downcast_ref::<Value>() {
            if let Some(rhs) = rhs.0.downcast_ref::<Value>() {
                return match (lhs, rhs) {
                    (Value::Number(a), Value::Number(b)) => {
                        FlowType(Arc::new(a.as_f64().unwrap() + b.as_f64().unwrap()))
                    }
                    (Value::String(a), Value::String(b)) => {
                        let mut res = a.clone();
                        res.push_str(b);
                        FlowType(Arc::new(a.clone()))
                    }
                    (Value::Array(a), Value::Array(b)) => {
                        let mut res = a.clone();
                        res.append(b.to_owned().as_mut());
                        FlowType(Arc::new(a.clone()))
                    }
                    (a, b) => panic!(
                        "Addition of JSON values of type {:?} and {:?} is not supported.",
                        a, b
                    ),
                };
            }
        }
        panic!(
            "Addition not supported for type {:?} and {:?}.",
            self.type_id(),
            rhs.type_id()
        );
    }
}

pub struct AppState {
    // For a yet TBD reason a HashMap of dyn types looses track of channel pointers.
    // As a workaround Nodes are resolved in a two step process and stored in a Vec.
    pub nodes: Vec<Box<dyn RuntimeNode + Send>>,
    pub node_idc: HashMap<String, usize>,
    pub context: State<Context>,
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            nodes: Vec::new(),
            node_idc: HashMap::new(),
            context: State::new(Context::new()),
        }
    }

    pub fn add_node(&mut self, name: &str, kind: String, props: Value) -> String {
        let node: Box<dyn RuntimeNode + Send> = match kind.as_str() {
            "nodes.arithmetics.add" => Box::new(AddNode::<FlowType, FlowType, FlowType>::new(
                name,
                self.context.clone(),
                Value::Null,
            )),
            "nodes.basic" => Box::new(BasicNode::new(
                name,
                self.context.clone(),
                FlowType(Arc::new(props)),
            )),
            "nodes.debug" => Box::new(DebugNode::<FlowType>::new(
                name,
                self.context.clone(),
                Value::Null,
            )),
            _ => panic!("Nodes of type {} are not yet supported.", kind),
        };
        self.nodes.push(node);
        self.node_idc.insert(name.to_owned(), self.nodes.len() - 1);
        name.to_owned()
    }

    pub fn connect_at(
        &mut self,
        lhs: String,
        rhs: String,
        index_in: usize,
        index_out: usize,
    ) -> Result<(), ConnectError<FlowType>> {
        let lhs_idx = self.node_idc.get(&lhs).unwrap().clone();
        let rhs_idx = self.node_idc.get(&rhs).unwrap().clone();
        // TODO: RefCell is not an ideal solution here.
        let out_edge = self.nodes[lhs_idx]
            .output_at(index_out)
            .downcast_ref::<Output<FlowType>>()
            .expect(&format!(
                "{} Nodes output at {} couldn't be downcasted",
                lhs, index_in
            ))
            .clone();
        let in_edge = self.nodes[rhs_idx]
            .input_at(index_in)
            .downcast_ref::<Input<FlowType>>()
            .unwrap()
            .to_owned();
        connect(out_edge, in_edge);
        Ok(())
    }

    pub fn run(&mut self) {
        self.nodes.iter_mut().for_each(|job| job.on_init());
        self.nodes.iter_mut().for_each(|job| job.on_ready());
        // Capped at 100 here for testing purposes. TODO: change to infinite loop with stop condition.
        for _ in 0..100 {
            self.nodes.iter_mut().for_each(|job| {
                let _ = job.update();
            });
        }
        self.nodes.iter_mut().for_each(|job| job.on_shutdown());
    }
}

#[derive(Deserialize)]
struct JsonNode {
    name: String,
    kind: String,
    props: Value,
}

#[derive(Deserialize)]
struct JsonConnection {
    node: String,
    index: usize,
}

#[derive(Deserialize)]
struct JsonEdge {
    source: JsonConnection,
    dest: JsonConnection,
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
            app_state.add_node(&node.name, node.kind.clone(), node.props.to_owned());
        });
        json_data.edges.iter().for_each(|edge| {
            let _ = app_state.connect_at(
                edge.source.node.clone(),
                edge.dest.node.clone(),
                edge.dest.index,
                edge.source.index,
            );
        });

        Ok(app_state)
    }
}

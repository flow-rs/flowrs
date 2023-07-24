use serde::Deserialize;

use crate::flow::app_state::FlowType;

use super::connection::Edge;

#[derive(Deserialize)]
pub struct Context {}

pub trait RuntimeConnectable {
    fn input_at(&self, index: usize) -> FlowType;
    fn output_at(&self, index: usize) -> FlowType;
}

pub trait Node {
    type Output;
    fn name(&self) -> &str;
    fn on_init(&mut self);
    fn on_ready(&mut self);
    fn on_shutdown(&mut self);
    fn update(&mut self);
    fn connect(&mut self, edge: Edge<Self::Output>);
}

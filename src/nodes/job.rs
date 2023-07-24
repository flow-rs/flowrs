use std::{any::Any, sync::{Arc, Mutex}, rc::Rc};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Context {}

pub trait RuntimeConnectable {
    fn input_at(&self, index: usize) -> Rc<dyn Any>;
    fn output_at(&self, index: usize) -> Rc<dyn Any>;
}

pub trait Node {
    fn name(&self) -> &str;
    fn on_init(&mut self);
    fn on_ready(&mut self);
    fn on_shutdown(&mut self);
    fn update(&mut self);
}

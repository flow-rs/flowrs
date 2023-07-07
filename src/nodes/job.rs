use std::sync::mpsc::Sender;

use serde::Deserialize;

use super::connection::ConnectError;

#[derive(Deserialize)]
pub struct Context {}

pub trait Node<I, O>: Job + Connectable<I, O> {}
impl<I, O, T: Job + Connectable<I, O>> Node<I, O> for T {}

pub trait Job {
    fn name(&self) -> &String;
    fn handle(&mut self);
    fn init(&mut self);
    fn destory(&mut self);
}

pub trait Connectable<I, O> {
    fn chain(&mut self, successors: Vec<Sender<O>>);
    fn inputs(&self) -> &Vec<Sender<I>>;
    fn input(&self) -> Result<Sender<I>, ConnectError<I>>;
    fn input_at(&self, index: usize) -> Result<Sender<I>, ConnectError<I>>;
    fn output(&self) -> &Vec<Sender<O>>;
    fn send_at(&self, index: usize, value: I) -> Result<(), ConnectError<I>>;
    fn send(&self, value: I) -> Result<(), ConnectError<I>>;
}

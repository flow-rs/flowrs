use std::sync::mpsc::Sender;

use super::connection::ConnectError;

pub struct Context {}

pub trait Job {
    fn handle(&mut self);
    fn name(&self) -> &String;
}

pub trait Connectable<I, O> {
    fn chain(&mut self, successors: Vec<Sender<O>>) -> &Self;
    fn inputs(&self) -> &Vec<Sender<I>>;
    fn input(&self) -> Result<Sender<I>, ConnectError<I>>;
    fn input_at(&self, index: usize) -> Result<Sender<I>, ConnectError<I>>;
    fn output(&self) -> &Vec<Sender<O>>;
    fn send_at(&self, index: usize, value: I) -> Result<(), ConnectError<I>>;
    fn send(&self, value: I) -> Result<(), ConnectError<I>>;
}

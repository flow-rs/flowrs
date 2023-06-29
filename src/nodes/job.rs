use std::sync::mpsc::Sender;

pub struct Context {}

pub trait Job {
    fn handle(&mut self);
    fn name(&self) -> &String;
}

pub trait Connectable<I, O> {
    fn chain(&mut self, successors: Vec<Sender<O>>) -> &Self;
    fn input(&self) -> &Vec<Sender<I>>;
    fn output(&self) -> &Vec<Sender<O>>;
    fn send_at(&self, index: usize, value: I);
    fn send(&self, value: I);
}

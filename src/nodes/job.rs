use std::{sync::mpsc::Sender};

pub trait Job<I, O> where {
    fn handle(&mut self);
    fn connect(&mut self, successors: Vec<Sender<O>>) -> &Self;
    fn input(&self) -> &Vec<Sender<I>>;
    fn output(&self) -> &Vec<Sender<O>>;
}

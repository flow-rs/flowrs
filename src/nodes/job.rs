use std::sync::mpsc::Sender;

pub trait Job {
    fn handle(&mut self);
    fn name(&self) -> &String;
}

pub trait Connectable<I, O> {
    fn connect(&mut self, successors: Vec<Sender<O>>) -> &Self;
    fn input(&self) -> &Vec<Sender<I>>;
    fn output(&self) -> &Vec<Sender<O>>;
}

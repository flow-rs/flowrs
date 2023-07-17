use std::{sync::mpsc::Sender, cell::Cell};

use serde::Deserialize;

use crate::connection::Connection;

use super::connection::ConnectError;

#[derive(Deserialize)]
pub struct Context {}

pub trait Node<I, O>: Job + Connectable<I, O> {
    fn on_init(&mut self);
    fn on_ready(&mut self);
    fn on_shutdown(&mut self);
}

pub trait Job {
    fn name(&self) -> &String;
    fn on_handle(&mut self);
}

pub trait Connectable<I, O> {
    fn conn(&mut self) -> &mut Connection<I, O>;
    fn chain(&self, successors: Vec<Sender<O>>);
    fn inputs(&self) -> &Vec<Sender<I>>;
    fn input(&self) -> Result<Sender<I>, ConnectError<I>>;
    fn input_at(&self, index: usize) -> Result<Sender<I>, ConnectError<I>>;
    fn output(&self) -> &Cell<Vec<Sender<O>>>;
    fn send_out(&self, value: O);
    fn send_at(&self, index: usize, value: I) -> Result<(), ConnectError<I>>;
    fn send(&self, value: I) -> Result<(), ConnectError<I>>;
}

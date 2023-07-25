use serde::Deserialize;

#[derive(Deserialize)]
pub struct Context {}

pub trait Node {
    fn name(&self) -> &str;
    fn on_init(&mut self);
    fn on_ready(&mut self);
    fn on_shutdown(&mut self);
    fn update(&mut self);
}

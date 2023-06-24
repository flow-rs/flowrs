use std::rc::Rc;

pub trait Job where {
    fn handle(&mut self);
}

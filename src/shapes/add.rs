use std::ops::Add;

#[derive(Clone)]
struct Queue<T> {
    name: String,
    jobs: Vec<T>,
}

#[derive(Clone)]
pub struct Node<I, O> {
    state: Option<I>,
    name: String,
    out: Queue<O>,
}

pub trait Addition<I, O, F>
where
    I: Add<Output = O> + Copy,
    F: Fn(O) + Sized,
{
    fn in1(&mut self, v: I);
    fn in2(&mut self, v: I);
}

pub trait Debug<T> {
    fn in1(v: T);
}

pub trait Schedule<T, F>
where
    T: Sized,
    F: Fn(T) + Sized,
{
    fn write(&mut self, v: T);
    fn handle_next(self);
    fn connect(&mut self, f: F);
}

impl<T, F> Schedule<T, F> for Queue<T>
where
    T: Sized + Clone,
    F: Fn(T) + Sized,
{
    fn write(&mut self, v: T) {
        self.jobs.push(v);
    }

    fn handle_next(mut self) {
        !todo!()
    }

    fn connect(&mut self, f: F) {
        match self.jobs.pop() {
            Some(j) => f(j),
            None => return,
        }
    }
}

impl<I, O, F> Addition<I, O, F> for Node<I, O>
where
    O: Clone,
    I: Add<Output = O> + Copy,
    F: Fn(O) + Sized,
{
    fn in1(&mut self, v: I) {
        match self.state {
            Some(w) => <Queue<O> as Schedule<O, F>>::write(&mut self.out, w + v),
            None => self.state = Some(v),
        };
    }

    fn in2(&mut self, v: I) {
        match self.state {
            Some(w) => <Queue<O> as Schedule<O, F>>::write(&mut self.out, w + v),
            None => self.state = Some(v),
        };
    }
}

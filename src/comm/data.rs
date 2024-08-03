use std::fmt;

#[derive(Debug, PartialEq)]
pub struct DataWrapper {}

impl fmt::Display for DataWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl DataWrapper {
    pub fn parse(_str: String) -> Self {
        todo!()
    }
}

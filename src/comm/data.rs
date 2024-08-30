use std::{fmt, str::FromStr};

#[derive(Debug, PartialEq)]
//Wraps a single data point of type D
pub struct DataWrapper<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    data: D,
}

impl<D> DataWrapper<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    pub fn parse(str: String) -> Result<Self, D::Err> {
        let res = str.parse::<D>()?;
        Ok(DataWrapper::<D> { data: res })
    }

    pub fn get_data(&self) -> D {
        self.data.clone()
    }
}

impl<D> fmt::Display for DataWrapper<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_primitives() {
        let data_wrapper_1 = DataWrapper::<u32> { data: 3 };
        let data_wrapper_2_res = DataWrapper::<u32>::parse("3".to_string());
        assert!(data_wrapper_2_res.is_ok());
        let data_wrapper_2 = data_wrapper_2_res.unwrap();
        assert_eq!(data_wrapper_1.get_data(), data_wrapper_2.get_data());
    }
}

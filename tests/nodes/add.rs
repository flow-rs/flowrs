#[cfg(test)]
mod nodes {
    use fbp::job::Job;
    use std::{ops::Add, sync::mpsc::channel};

    use fbp::add::AddNode;

    #[test]
    fn should_add_132() {
        let (s1, r1) = channel();
        let (s2, r2) = channel();
        let (s3, r3) = channel();

        let mut add = AddNode {
            state: None,
            name: "AddNodeI32".to_owned(),
            input: vec![r1, r2],
            output: vec![s3],
            neutral: 0,
            to_state: |i| i,
            from_state: |i| i,
        };

        s1.send(1);
        s2.send(2);

        assert!(add.state == None, "State was not empty at start");
        add.handle();
        assert!(add.state == Some(1));
        add.handle();
        assert!(add.state == None);
        assert!(add.output.len() == 1);
        assert!(r3.recv().unwrap() == 3);
    }

    #[test]
    fn should_add_132_str() {
        #[derive(Debug, PartialEq, Eq, Clone)]
        enum IorS {
            Int(i32),
            Str(String),
        }
        impl Add for IorS {
            type Output = IorS;

            fn add(self, rhs: Self) -> Self::Output {
                let int1 = match self {
                    IorS::Int(i) => i,
                    IorS::Str(s) => s.parse::<i32>().unwrap(),
                };
                let int2 = match rhs {
                    IorS::Int(i) => i,
                    IorS::Str(s) => s.parse::<i32>().unwrap(),
                };
                IorS::Int(int1 + int2)
            }
        }
        let (s1, r1) = channel();
        let (s2, r2) = channel();
        let (s3, r3) = channel();
        let mut add = AddNode {
            state: None,
            name: "AddNodeIntegerString".to_owned(),
            input: vec![r1, r2],
            output: vec![s3],
            neutral: IorS::Int(0),
            to_state: |i| i,
            from_state: |i| i,
        };

        s1.send(IorS::Int(1));
        s2.send(IorS::Str("2".to_owned()));

        assert!(add.state == None, "State was not empty at start");
        add.handle();
        assert!(add.state == Some(IorS::Int(1)));
        add.handle();
        assert!(add.state == None);
        assert!(r3.recv().unwrap() == IorS::Int(3));
    }
}

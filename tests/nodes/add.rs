#[cfg(test)]
mod nodes {
    use flow::job::{Connectable, Context, Job};
    use std::{
        ops::Add,
        sync::{
            mpsc::{channel, Receiver, Sender},
            Arc,
        },
        vec,
    };

    use flow::add::AddNode;

    #[test]
    fn should_add_132() {
        let context = Arc::new(Context {});
        let (mock_s, mock_r): (Sender<i32>, Receiver<i32>) = channel();

        let mut add = AddNode::new("AddNodeI32", context, 0);

        let _ = add.send(1);
        let _ = add.send_at(1, 2);
        add.chain(vec![mock_s]);

        assert!(add.state == None, "State was not empty at start");
        add.handle();
        assert!(add.state == Some(1));
        add.handle();
        assert!(add.state == None);
        assert!(add.output().len() == 1);
        assert!(mock_r.recv().unwrap() == 3);
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
        let context = Arc::new(Context {});
        let (mock_s, mock_r): (Sender<IorS>, Receiver<IorS>) = channel();

        let mut add = AddNode::new("AddNodeI32", context, IorS::Int(0));

        let _ = add.input()[0].send(IorS::Int(1));
        let _ = add.input()[1].send(IorS::Str("2".into()));
        add.chain(vec![mock_s]);

        assert!(add.state == None, "State was not empty at start");
        add.handle();
        assert!(add.state == Some(IorS::Int(1)));
        add.handle();
        assert!(add.state == None);
        assert!(mock_r.recv().unwrap() == IorS::Int(3));
    }
}

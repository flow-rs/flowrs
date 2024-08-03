use flowrs::comm;
use std::io::{stderr, stdin, stdout, Write};

fn main() {
    'outer: loop {
        let mut input = String::new();
        match stdin().read_line(&mut input) {
            Ok(_n) => match comm::messages::Message::from_str(&input) {
                Some(msg) => {
                    let output = format!("{:?}\n", msg);
                    match msg {
                        comm::messages::Message::StopExecution => break 'outer,
                        _ => (),
                    }
                    'inner: loop {
                        if let Ok(_) = stdout().write_all(output.as_bytes()) {
                            stdout().flush().unwrap();
                            break 'inner;
                        }
                    }
                }
                None => {
                    let error_msg = b"No message";
                    'inner: loop {
                        if let Ok(_) = stderr().write_all(error_msg) {
                            stderr().flush().unwrap();
                            break 'inner;
                        }
                    }
                }
            },
            Err(err) => {
                let error_msg = format!("{:?}", err);
                'inner: loop {
                    if let Ok(_) = stderr().write_all(error_msg.as_bytes()) {
                        stderr().flush().unwrap();
                        break 'inner;
                    }
                }
            }
        }
    }
}

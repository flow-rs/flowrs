#[cfg(test)]
mod scheduler {
    
    use flow::{executor::{Executor, MultiThreadedExecutor}, scheduler::{RoundRobinScheduler},node::{Context, State, Node}, flow_type::Flow, version::Version};
    use flow::connection::{connect, Edge, Input};

    use std::{thread, sync::mpsc, time::Duration, sync::{Arc, Mutex}};
    use serde_json::Value;

    use flow::add::AddNode;


    #[test]
    fn test() {
 
        let (sender, receiver) = mpsc::channel();
        
        let context = State::new(Context::new());

        let add = AddNode::<i32, i32, i32>::new("AddNodeI32", context.clone(), Value::Null);
        let mock_input = Input::<i32>::new();        
        connect(add.output_1.clone(), mock_input.clone());


        let mut flow = Flow::new("flow1", Version::new(1,0,0));

        add.input_1.send(1);
        add.input_2.send(2);
        
        flow.add_node(add);

        let thread_handle = thread::spawn( move || {
        
            let num_threads = 4;
            let mut executor = MultiThreadedExecutor::new(num_threads, context);
            let mut scheduler = RoundRobinScheduler::new();

            let _ = sender.send(executor.controller());

            executor.run(flow, scheduler);
        });

        let controller = receiver.recv().unwrap();

        thread::sleep(Duration::from_secs(3));

        println!("CANCEL");

        controller.lock().unwrap().cancel();
       
        thread_handle.join().unwrap();

        //println!("Has next: {}",  mock_output.has_next());

    }

    #[test]
    fn test2() {
         
        let context = State::new(Context::new());

        let add = AddNode::<i32, i32, i32>::new("AddNodeI32", context, Value::Null);
        let mock_output = Edge::new();        
        connect(add.output_1.clone(), mock_output.clone());

       
        let thread_handle = thread::spawn( move || {
        
            add.update();

        });

       
        thread::sleep(Duration::from_secs(3));

       
        thread_handle.join().unwrap();

        println!("Has next: {}",  mock_output.has_next());

    }


} 
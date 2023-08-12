use crate::exec::execution_state::ExecutionState;

pub struct ExecutionController {
    state: ExecutionState,
    cancellation_requested: bool,
    change_notifier: std::sync::mpsc::Sender<bool>,
}


impl ExecutionController {
    pub fn new(change_notifier: std::sync::mpsc::Sender<bool>) -> Self {
        Self {
            state: ExecutionState::Ready,
            cancellation_requested: false,
            change_notifier: change_notifier,
        }
    }

    pub fn cancel(&mut self) {
        self.cancellation_requested = true;
        if self.state == ExecutionState::Sleeping {
            let _res = self.change_notifier.send(true);
        }
    }

    pub fn state(&self) -> ExecutionState {
        self.state
    }

    pub fn set_state(&mut self, s: ExecutionState) {
        self.state = s
    }

    pub fn cancellation_requested(&self) -> bool {
        self.cancellation_requested
    }
}
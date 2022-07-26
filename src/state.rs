use crate::message::Message;

pub struct PeerStateMachine {
    state: State,
    log: Vec<Message>,
}

#[derive(Debug)]
pub enum State {
    Offline,
    Syncing,
    Connected,
}

impl Default for State {
    fn default() -> Self {
        State::Offline
    }
}

impl PeerStateMachine {
    fn new() -> Self {
        Self {
            state: State::default(),
            log: Vec::new(),
        }
    }

    fn apply(&mut self, msg: Message) -> Result<(), crate::Error> {
        unimplemented!()
    }
}



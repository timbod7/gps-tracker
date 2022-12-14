pub struct Debouncer {
    state: State,
    count: usize,
    wait: usize,
}

impl Debouncer {
    pub fn new(wait: usize) -> Self {
        Debouncer {
            state: State::Init,
            count: 0,
            wait,
        }
    }

    pub fn next(&mut self, input: bool) -> Option<Transition> {
        match (&self.state, input) {
            (State::Init, true) => {
                self.state = State::High;
                Option::None
            }
            (State::Init, false) => {
                self.state = State::Low;
                Option::None
            }
            (State::Low, false) => Option::None,
            (State::Low, true) => {
                self.state = State::ToHigh;
                self.count = 1;
                Option::None
            }
            (State::ToHigh, false) => {
                self.state = State::Low;
                Option::None
            }
            (State::ToHigh, true) => {
                self.count += 1;
                if self.count == self.wait {
                    self.state = State::High;
                    Option::Some(Transition::ToHigh)
                } else {
                    Option::None
                }
            }
            (State::High, true) => Option::None,
            (State::High, false) => {
                self.state = State::ToLow;
                self.count = 1;
                Option::None
            }
            (State::ToLow, true) => {
                self.state = State::High;
                Option::None
            }
            (State::ToLow, false) => {
                self.count += 1;
                if self.count == self.wait {
                    self.state = State::Low;
                    Option::Some(Transition::ToLow)
                } else {
                    Option::None
                }
            }
        }
    }
}

enum State {
    Init,
    ToHigh,
    High,
    ToLow,
    Low,
}

pub enum Transition {
    ToHigh,
    ToLow,
}

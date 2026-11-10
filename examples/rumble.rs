//! rumble — pulse the motors without depending on the button mapping. Feel it, don't read it.
use rumblepad::Controller;
use std::{thread::sleep, time::Duration};

fn main() {
    let mut c = Controller::open().expect("open pad");
    for (l, r) in [(1.0, 0.0), (0.0, 1.0), (1.0, 1.0), (0.4, 0.4)] {
        c.rumble(l, r).expect("rumble on");
        sleep(Duration::from_millis(400));
        c.rumble_off().expect("rumble off");
        sleep(Duration::from_millis(400));
    }
    println!("4 pulses sent: left, right, both, weak-both");
}

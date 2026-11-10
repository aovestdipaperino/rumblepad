//! Live readout. A/B/X rumble pulse, plus quits.
use rumblepad::{Controller, btn};

fn main() {
    let mut c = Controller::open()
        .expect("open Pro Controller (Input Monitoring permission for this terminal?)");
    println!("Connected. A/B/X rumble, + quits.");
    let mut last: u32 = 0;
    loop {
        match c.poll(100) {
            Ok(Some(state)) => {
                let (l, r) = (
                    if state.pressed(btn::A) || state.pressed(btn::B) {
                        1.0
                    } else {
                        0.0
                    },
                    if state.pressed(btn::A) || state.pressed(btn::X) {
                        1.0
                    } else {
                        0.0
                    },
                );
                if state.buttons != last {
                    c.rumble(l, r).expect("rumble");
                    last = state.buttons;
                }
                print!(
                    "\r{:06x} L({:+.2},{:+.2}) R({:+.2},{:+.2})   ",
                    state.buttons, state.left.0, state.left.1, state.right.0, state.right.1
                );
                if state.pressed(btn::PLUS) {
                    break;
                }
            }
            Ok(None) => continue,
            Err(e) => eprintln!("poll error: {:?}", e),
        }
    }
}

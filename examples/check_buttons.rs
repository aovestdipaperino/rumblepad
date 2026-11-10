//! check_buttons — S-mode proof that every button is detected, confirmed by rumble.
//! One tap = that button was just detected. The demo only exits once all of them have reported.
use rumblepad::{Controller, Kind, btn};
use std::{thread::sleep, time::Duration};

const BUTTONS: &[(&str, u32)] = &[
    ("Y", btn::Y),
    ("X", btn::X),
    ("B", btn::B),
    ("A", btn::A),
    ("L", btn::L),
    ("R", btn::R),
    ("ZL", btn::ZL),
    ("ZR", btn::ZR),
    ("L3", btn::LSTICK),
    ("R3", btn::RSTICK),
    ("-", btn::MINUS),
    ("+", btn::PLUS),
    ("HOME", btn::HOME),
    ("CAPTURE", btn::CAPTURE),
    ("UP", btn::UP),
    ("DOWN", btn::DOWN),
    ("LEFT", btn::LEFT),
    ("RIGHT", btn::RIGHT),
];

fn tap(c: &mut Controller, ms: u64, n: u32) {
    for _ in 0..n {
        c.rumble(1.0, 1.0).expect("rumble on");
        sleep(Duration::from_millis(ms));
        c.rumble_off().expect("rumble off");
        sleep(Duration::from_millis(ms));
    }
}

fn main() {
    let mut c = Controller::open().expect("open pad");
    if c.kind() != Kind::Switch {
        panic!("pad is in D-input mode, where rumble is a no-op — flip the mode switch to S");
    }

    let done = (1u32 << BUTTONS.len()) - 1;
    let mut found = 0u32;
    println!("Rumble self-test: feel 2 taps now. Then press every button, 1 tap each.");
    tap(&mut c, 150, 2);

    while found != done {
        let s = match c.poll(100) {
            Ok(Some(s)) => s,
            Ok(None) => continue,
            Err(e) => {
                eprintln!("poll: {e:?}");
                continue;
            }
        };
        for (i, (name, mask)) in BUTTONS.iter().enumerate() {
            let bit = 1u32 << i;
            if found & bit == 0 && s.pressed(*mask) {
                found |= bit;
                print!("{name} ");
                std::io::Write::flush(&mut std::io::stdout()).ok();
                tap(&mut c, 100, 1);
            }
        }
    }

    println!("\nall {} buttons detected", BUTTONS.len());
    tap(&mut c, 300, 3);
}

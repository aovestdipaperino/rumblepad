#![doc(
    html_logo_url = "https://raw.githubusercontent.com/aovestdipaperino/rumblepad/main/assets/logo.svg"
)]
//! Switch-protocol pads (Nintendo, or an 8BitDo with its mode switch on S) and 8BitDo D-input
//! over HID (Bluetooth): input state + HD rumble. Rumble needs the Switch side; D-input has none.
use hidapi::{HidApi, HidDevice, HidError};

const DEADZONE: f32 = 0.1;
const STICK_CENTER: f32 = 2048.0;
const STICK_RANGE: f32 = 1500.0;

pub mod btn {
    pub const Y: u32 = 1 << 0;
    pub const X: u32 = 1 << 1;
    pub const B: u32 = 1 << 2;
    pub const A: u32 = 1 << 3;
    pub const R: u32 = 1 << 6;
    pub const ZR: u32 = 1 << 7;
    pub const MINUS: u32 = 1 << 8;
    pub const PLUS: u32 = 1 << 9;
    pub const RSTICK: u32 = 1 << 10;
    pub const LSTICK: u32 = 1 << 11;
    pub const HOME: u32 = 1 << 12;
    pub const CAPTURE: u32 = 1 << 13;
    pub const DOWN: u32 = 1 << 16;
    pub const UP: u32 = 1 << 17;
    pub const RIGHT: u32 = 1 << 18;
    pub const LEFT: u32 = 1 << 19;
    pub const L: u32 = 1 << 22;
    pub const ZL: u32 = 1 << 23;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Switch,
    BitDo,
}

// Tried in order, first pad found wins.
const PADS: &[(u16, u16, Kind)] = &[
    (0x057E, 0x2009, Kind::Switch),
    (0x2DC8, 0x5112, Kind::BitDo),
];

// 8BitDo D-input bit order -> btn:: numbering.
// shortcut: order taken from 8BitDo's documented button list, not yet confirmed button by button.
const BITDO_BITS: &[(u8, u32)] = &[
    (0, btn::A),
    (1, btn::B),
    (2, btn::X),
    (3, btn::Y),
    (4, btn::L),
    (5, btn::R),
    (6, btn::ZL),
    (7, btn::ZR),
    (8, btn::MINUS),
    (9, btn::PLUS),
    (10, btn::LSTICK),
    (11, btn::RSTICK),
    (12, btn::HOME),
    (13, btn::CAPTURE),
];

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct State {
    pub buttons: u32,
    pub left: (f32, f32),
    pub right: (f32, f32),
}

impl State {
    pub fn pressed(&self, mask: u32) -> bool {
        self.buttons & mask == mask
    }
}

fn stick_axis(raw: u16) -> f32 {
    let v = ((raw as f32 - STICK_CENTER) / STICK_RANGE).clamp(-1.0, 1.0);
    if v.abs() < DEADZONE { 0.0 } else { v }
}

fn stick(b: &[u8]) -> (f32, f32) {
    let x = b[0] as u16 | ((b[1] as u16 & 0xF) << 8);
    let y = (b[1] as u16 >> 4) | ((b[2] as u16) << 4);
    (stick_axis(x), stick_axis(y))
}

pub fn parse(r: &[u8]) -> Option<State> {
    match r.first() {
        Some(0x30) => parse_switch(r),
        Some(0x01) => parse_bitdo(r),
        _ => None,
    }
}

fn parse_switch(r: &[u8]) -> Option<State> {
    if r.len() < 12 {
        return None;
    }
    Some(State {
        buttons: r[3] as u32 | (r[4] as u32) << 8 | (r[5] as u32) << 16,
        left: stick(&r[6..9]),
        right: stick(&r[9..12]),
    })
}

// 8-bit sticks are centred at 0x7F/0x80; HID Y grows downwards, so it is negated to match the Switch's up = +.
fn axis8(v: u8) -> f32 {
    let a = ((v as f32 - 127.5) / 127.5).clamp(-1.0, 1.0);
    if a.abs() < DEADZONE { 0.0 } else { a }
}

fn parse_bitdo(r: &[u8]) -> Option<State> {
    if r.len() < 8 {
        return None;
    }
    let raw = r[1] as u32 | (r[2] as u32) << 8;
    let mut buttons = 0u32;
    for (bit, mask) in BITDO_BITS {
        if raw & (1 << bit) != 0 {
            buttons |= mask;
        }
    }
    // D-pad is a 4-bit hat in the high nibble: 8 = neutral, 0 = up, then clockwise.
    buttons |= match r[3] >> 4 {
        0 => btn::UP,
        1 => btn::UP | btn::RIGHT,
        2 => btn::RIGHT,
        3 => btn::DOWN | btn::RIGHT,
        4 => btn::DOWN,
        5 => btn::DOWN | btn::LEFT,
        6 => btn::LEFT,
        7 => btn::UP | btn::LEFT,
        _ => 0,
    };
    Some(State {
        buttons,
        left: (axis8(r[4]), -axis8(r[5])),
        right: (axis8(r[6]), -axis8(r[7])),
    })
}

pub fn encode_rumble(strength: f32) -> [u8; 4] {
    let i = (strength.clamp(0.0, 1.0) * 100.0).round() as u16;
    let high = 2 * i;
    let low = 0x40 + (i >> 1) + if i % 2 == 1 { 0x8000 } else { 0 };
    [0x00, 0x01 + high as u8, 0x40 + (low >> 8) as u8, low as u8]
}

pub struct Controller {
    dev: HidDevice,
    counter: u8,
    kind: Kind,
}

impl Controller {
    pub fn open() -> Result<Self, HidError> {
        let api = HidApi::new()?;
        let mut found = None;
        for (vid, pid, kind) in PADS {
            if let Ok(dev) = api.open(*vid, *pid) {
                found = Some((dev, *kind));
                break;
            }
        }
        let (dev, kind) = found.ok_or(HidError::HidApiError {
            message: "No supported pad (Switch Pro / 8BitDo) found in the system.".into(),
        })?;
        let mut c = Controller {
            dev,
            counter: 0,
            kind,
        };

        if kind == Kind::Switch {
            // Enable vibration
            c.subcommand(0x48, &[0x01])?;
            // Full input reports
            c.subcommand(0x03, &[0x30])?;
            // Player 1 LED
            c.subcommand(0x30, &[0x01])?;
        }

        Ok(c)
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    fn next(&mut self) -> u8 {
        let n = self.counter;
        self.counter = (n + 1) & 0xF;
        n
    }

    fn send(&mut self, mut p: Vec<u8>) -> Result<(), HidError> {
        p.resize(49, 0);
        self.dev.write(&p).map(|_| ())
    }

    fn subcommand(&mut self, id: u8, data: &[u8]) -> Result<(), HidError> {
        // Layout: [0x01, packet_num, 8 rumble bytes (zeros), subcmd id, data...]
        let mut p = vec![
            0x01,
            self.next(),
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            id,
        ];
        p.extend_from_slice(data);
        self.send(p)
    }

    pub fn poll(&self, timeout_ms: i32) -> Result<Option<State>, HidError> {
        let mut buf = [0u8; 64];
        let n = self.dev.read_timeout(&mut buf, timeout_ms)?;
        Ok(parse(&buf[..n]))
    }

    pub fn rumble(&mut self, left: f32, right: f32) -> Result<(), HidError> {
        // shortcut: D-input mode has no rumble command we can find. Writes are accepted by the HID
        // layer regardless (Ok even for garbage), and 36 probes across xpad/ds4/switch/vendor
        // payloads moved nothing. The motors do work in S mode, which speaks the Switch protocol.
        if self.kind != Kind::Switch {
            return Ok(());
        }
        let mut p = vec![0x10, self.next()];
        p.extend_from_slice(&encode_rumble(left));
        p.extend_from_slice(&encode_rumble(right));
        self.send(p)
    }

    pub fn rumble_off(&mut self) -> Result<(), HidError> {
        self.rumble(0.0, 0.0)
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        let _ = self.rumble_off();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_buttons_and_sticks() {
        let mut r = [0u8; 13];
        r[0] = 0x30;
        r[3] = 0x08;
        r[5] = 0x80;
        r[6] = 0x00;
        r[7] = 0xF8;
        r[8] = 0xFF;
        let s = parse(&r).unwrap();
        assert!(s.pressed(btn::A) && s.pressed(btn::ZL) && !s.pressed(btn::B));
        assert_eq!(s.left.0, 0.0);
        assert_eq!(s.left.1, 1.0);
        assert!(parse(&[0x3F; 13]).is_none());
    }

    #[test]
    fn deadzone_and_clamp() {
        assert_eq!(stick_axis(2048 + 50), 0.0);
        assert_eq!(stick_axis(0), -1.0);
    }

    #[test]
    fn rumble_matches_kernel_table() {
        assert_eq!(encode_rumble(0.0), [0x00, 0x01, 0x40, 0x40]);
        assert_eq!(encode_rumble(0.01), [0x00, 0x03, 0xC0, 0x40]);
        assert_eq!(encode_rumble(1.0), [0x00, 0xC9, 0x40, 0x72]);
    }

    #[test]
    fn parses_bitdo_report() {
        // Report id 1: 16 button bits, 4-bit hat in the high nibble, X/Y/Z/Rz 8-bit sticks.
        let mut r = [0u8; 10];
        r[0] = 0x01;
        r[3] = 0x80; // hat neutral
        r[4..8].copy_from_slice(&[0x7F, 0x7F, 0x7F, 0x80]);
        assert_eq!(parse(&r), Some(State::default()));

        r[1] = 1 << 0; // A
        r[2] = 1 << 1; // +
        r[3] = 0x00; // hat up
        r[4] = 0xFF; // left stick fully right
        r[5] = 0x00; // HID Y grows down, so this reads as fully up
        let s = parse(&r).unwrap();
        assert!(s.pressed(btn::A | btn::PLUS | btn::UP));
        assert!(!s.pressed(btn::B));
        assert_eq!(s.left, (1.0, 1.0));
        assert_eq!(s.right, (0.0, 0.0));
        assert!(parse(&[0x01, 0x00]).is_none());
    }
}

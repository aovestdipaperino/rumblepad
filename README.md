# rumblepad

Switch-protocol and 8BitDo gamepads over HID (Bluetooth): input state and HD rumble, in Rust.

Built against, and measured on, an **8BitDo Lite 2** — a single physical pad whose mode switch
changes its HID identity entirely. That duality is the whole design of this crate.

| Mode | Enumerates as | Protocol | Rumble |
|------|---------------|----------|--------|
| **S** | `0x057E:0x2009` "Pro Controller" | Switch: report `0x30`, subcommands, output report `0x10` | **yes** |
| **D** | `0x2DC8:0x5112` "8BitDo Lite 2" | plain D-input: report ID `0x01` | **no** |

`Controller::open()` tries both and reports which it found via `Controller::kind()`. Input is
decoded into one `State` for both modes, so `state.pressed(btn::A)` reads the same either way.

## Use it

```rust
use rumblepad::{btn, Controller};

let mut c = Controller::open()?;          // Switch Pro, or an 8BitDo in S or D mode
loop {
    if let Some(s) = c.poll(100)? {        // 100 ms timeout; None = nothing arrived
        if s.pressed(btn::A) {
            c.rumble(1.0, 0.5)?;           // left, right — 0.0..=1.0
        }
        let (lx, ly) = s.left;             // deadzoned, -1.0..=1.0, +y = up
    }
}
```

`rumble_off()` is called on `Drop`, so a dropped controller stops buzzing.

## The rumble caveat, measured rather than assumed

Rumble is **S-mode only**. This is worth stating plainly because the obvious way to check is wrong:

- The HID report descriptor reports **no Output reports in either mode** — including S mode, where
  rumble demonstrably works. Over Bluetooth HID, output travels as `SET_REPORT` on the control
  channel regardless of what the descriptor advertises.
- `HidDevice::write()` returns `Ok(n)` for *any* payload. 36 writes of xpad-, DS4-, Switch- and
  vendor-guess rumble commands to a D-mode pad all returned `Ok`, and moved nothing.

So neither the descriptor nor the write result predicts rumble; only the motors do. `rumble()` is a
no-op returning `Ok(())` on a D-mode pad. If you need rumble, the pad must be on S — which also
means input arrives as Switch report `0x30`, not D-input `0x01`. You cannot have rumble and the
D-input Xbox-style layout at the same time.

## Examples

```console
cargo run --example demo            # live button/stick readout, A/B/X pulses, + quits
cargo run --example rumble          # 4 motor pulses, independent of the button mapping
cargo run --example check_buttons   # S-mode: one tap per detected button, exits when all 18 report
cargo run --example probe 057e:2009 # dump the HID report descriptor + raw input reports
cargo run --example scan 2dc8       # list every HID collection, and raw-write candidates to a VID
```

`probe` and `scan` are the tools that produced the numbers above; keep them if you are debugging a
pad this crate does not know yet.

## Platform notes

- **macOS**: if `open()` reports no device, the pad is asleep or unpaired — check
  `system_profiler SPBluetoothDataType` before blaming permissions. *Input Monitoring* only matters
  at the open/read step, once the device is actually present.
- D-input pads report **only on change**, so an idle readout prints nothing. That is not a stall.
- The D-pad in D mode is a 4-bit **hat**, not button bits (neutral = `8`), and L2/R2 arrive as axis
  bytes that this crate drops, since `State` has no trigger field.

## Status

Small and honest: 4 unit tests, and the parts that could be checked against hardware were. Two
caveats a contributor should know: the D-mode **button-name table** was derived from the report
descriptor and 8BitDo's documented button order rather than a press-by-press capture, and the
**stick polarity** in D mode follows the HID convention (Y grows downwards, so it is negated) rather
than a measurement. Both are marked with `shortcut:` comments in the source.

## License

Licensed under either of Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE)) or the MIT
license ([LICENSE-MIT](LICENSE-MIT)) at your option.

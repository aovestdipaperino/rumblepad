//! scan [VID] — list every HID collection and whether its descriptor declares Output/Feature reports.
//! With a VID given, also attempt a raw write on matching devices to test undocumented SET_REPORTs.
use hidapi::HidApi;
use std::{thread::sleep, time::Duration};

fn main() {
    let api = HidApi::new().unwrap();
    let probe_vid: Option<u16> = std::env::args()
        .nth(1)
        .and_then(|a| u16::from_str_radix(a.trim_start_matches("0x"), 16).ok());

    for d in api.device_list() {
        let (vid, pid) = (d.vendor_id(), d.product_id());
        let name = d.product_string().unwrap_or("?");
        let Ok(dev) = d.open_device(&api) else {
            println!(
                "{vid:04x}:{pid:04x} {name:<26} if{} (open failed)",
                d.interface_number()
            );
            continue;
        };
        let mut b = [0u8; 4096];
        let n = dev.get_report_descriptor(&mut b).unwrap_or(0);
        // Short-item walk: main items are Input 0x81 / Output 0x91 / Feature 0xB1.
        let (mut out, mut feat) = (false, false);
        let mut i = 0;
        while i < n {
            let b0 = b[i];
            if b0 & 0xFC == 0x80 {
                out |= b0 == 0x91;
                feat |= b0 == 0xB1;
            }
            i += 1 + usize::from(b0 & 3);
        }
        println!(
            "{vid:04x}:{pid:04x} {name:<26} if{} usage={:04x}/{:04x} desc={n:<5} output={out} feature={feat}",
            d.interface_number(),
            d.usage_page(),
            d.usage()
        );

        if probe_vid == Some(vid) {
            // Candidate rumble commands, each as 3 taps. Tell me which burst buzzed.
            let cands: &[(&str, &[u8])] = &[
                (
                    "1 xpad  ",
                    &[0x00, 0x00, 0x04, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0x00],
                ),
                (
                    "2 ds4   ",
                    &[0x05, 0x00, 0x00, 0x00, 0xF0, 0xF0, 0x00, 0x00],
                ),
                (
                    "3 switch",
                    &[0x10, 0x00, 0x00, 0x01, 0x40, 0x40, 0x00, 0x01, 0x40, 0x40],
                ),
                ("4 vendor", &[0x01, 0xFF, 0xFF, 0x00, 0x00]),
            ];
            for (name, p) in cands {
                let off = vec![0u8; p.len()];
                for _ in 0..3 {
                    println!("    {name} {:02x?} -> {:?}", p, dev.write(p));
                    sleep(Duration::from_millis(250));
                    let _ = dev.write(&off);
                    sleep(Duration::from_millis(200));
                }
                sleep(Duration::from_millis(1800));
            }
        }
    }
}

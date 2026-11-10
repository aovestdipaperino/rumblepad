//! probe [VID:PID] — dump the HID report descriptor, then hexdump input reports for 20s.
use std::time::Instant;

fn main() {
    let api = hidapi::HidApi::new().unwrap();
    let arg = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "057e:2009".into());
    let mut it = arg.split(':');
    let vid = u16::from_str_radix(it.next().unwrap(), 16).unwrap();
    let pid = u16::from_str_radix(it.next().unwrap(), 16).unwrap();
    let dev = api.open(vid, pid).unwrap();

    let mut d = [0u8; 4096];
    let n = dev.get_report_descriptor(&mut d).unwrap_or(0);
    eprintln!("descriptor {} bytes: {:02x?}", n, &d[..n]);

    let mut b = [0u8; 64];
    let t0 = Instant::now();
    while t0.elapsed().as_secs() < 20 {
        if let Ok(k) = dev.read_timeout(&mut b, 500)
            && k > 0
        {
            println!("{:5.2} {:02x?}", t0.elapsed().as_secs_f32(), &b[..k]);
        }
    }
}

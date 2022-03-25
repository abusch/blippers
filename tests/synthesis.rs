use blippers::{BlipBuf, BLIP_MAX_RATIO};

const OVERSAMPLE: i32 = BLIP_MAX_RATIO;
const BLIP_SIZE: i32 = 32;

struct Fixture {
    b: BlipBuf,
    buf: Box<[i16]>,
    stereo_buf: Box<[i16]>,
    crc: u32,
}

impl Fixture {
    fn new() -> Self {
        Self {
            b: BlipBuf::new(BLIP_SIZE as usize),
            buf: vec![0; BLIP_SIZE as usize].into_boxed_slice(),
            stereo_buf: vec![0; BLIP_SIZE as usize * 2].into_boxed_slice(),
            crc: 0,
        }
    }

    fn end_frame_and_dump_buf_deltas(&mut self) {
        self.b.end_frame((BLIP_SIZE * OVERSAMPLE) as u64);
        self.b.read_samples(&mut self.buf[..], false);
        for i in 1..BLIP_SIZE as usize {
            self.print(format!("{} ", self.buf[i] - self.buf[i - 1]));
        }
        self.print("\n");
        self.b.clear();
    }

    fn calc_crc32(&self, input: &[u8], mut crc: u32) -> u32 {
        crc = !crc;
        for b in input {
            crc ^= *b as u32;
            for _ in (1..=8).rev() {
                crc = (crc >> 1) ^ (0xEDB88320 & (-(crc as i32 & 1) as u32));
            }
        }

        !crc
    }

    fn clear_crc(&mut self) {
        self.crc = 0;
    }

    fn check_crc(&mut self, crc: u32) {
        assert_eq!(self.crc, crc, "CRC mismatch");
        self.clear_crc();
    }

    fn print(&mut self, s: impl AsRef<str>) {
        print!("{}", s.as_ref());
        self.crc = self.calc_crc32(s.as_ref().as_bytes(), self.crc);
    }
}

fn fixture() -> Fixture {
    Fixture::new()
}

#[test]
fn add_delta_tails() {
    let mut f = fixture();
    f.b.add_delta(0, 16384);
    f.end_frame_and_dump_buf_deltas();
    f.check_crc(0x8DE789B2);

    f.b.add_delta(OVERSAMPLE as u64 / 2, 16384);
    f.end_frame_and_dump_buf_deltas();
    f.check_crc(0x3BD3F8BF);
}

#[test]
fn add_delta_interpolation() {
    let mut f = fixture();
    f.b.add_delta(OVERSAMPLE as u64 / 2, 32768);
    f.end_frame_and_dump_buf_deltas();

    // Values should be halfway between values for above and below
    f.b.add_delta(OVERSAMPLE as u64 / 2 + OVERSAMPLE as u64 / 64, 32768);
    f.end_frame_and_dump_buf_deltas();

    f.b.add_delta(OVERSAMPLE as u64 / 2 + OVERSAMPLE as u64 / 32, 32768);
    f.end_frame_and_dump_buf_deltas();

    f.check_crc(0x2593B066);
}

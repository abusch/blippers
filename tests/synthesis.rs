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

#[test]
fn saturation() {
    let test_saturation = |delta| {
        let mut f = fixture();
        f.b.add_delta_fast(0, delta);
        f.b.end_frame(OVERSAMPLE as u64 * BLIP_SIZE as u64);
        f.b.read_samples(&mut f.buf, false);
        f.buf[20]
    };

    assert_eq!(test_saturation(35000), 32767);
    assert_eq!(test_saturation(-35000), -32768);
}

#[test]
fn stereo_interleave() {
    let mut f = fixture();
    
    f.b.add_delta(0, 16384);
    f.b.end_frame((BLIP_SIZE * OVERSAMPLE) as u64);
    f.b.read_samples(&mut f.buf, false);

    f.b.clear();
    f.b.add_delta(0, 16384);
    f.b.end_frame((BLIP_SIZE * OVERSAMPLE) as u64);
    f.b.read_samples(&mut f.stereo_buf, true);

    for i in 0..BLIP_SIZE {
        assert_eq!(f.stereo_buf[i as usize * 2], f.buf[i as usize]);
    }
}

#[test]
fn clear() {
    let mut f = fixture();

    // Make first and last internal samples non-zero
    f.b.add_delta(0, 32768);
    f.b.add_delta(((BLIP_SIZE + 2) * OVERSAMPLE + OVERSAMPLE / 2) as u64, 32768);

    f.b.clear();

    for _ in (0..=2).rev() {
        f.b.end_frame((BLIP_SIZE * OVERSAMPLE) as u64);
        f.b.read_samples(&mut f.buf, false);
        assert_eq!(f.buf.len() as i32, BLIP_SIZE);
        for i in 0..BLIP_SIZE {
            assert_eq!(f.buf[i as usize], 0);
        }
    }
}

//! Implements a simple square wave generator as would be in a sound chip emulator
use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Seek, Write},
};

use blippers::BlipBuf;
use hound::{SampleFormat, WavSpec, WavWriter};

/// 44.1 kHz sample rate
const SAMPLE_RATE: f64 = 44100.0;
/// 3.58 MHz clock rate
const CLOCK_RATE: f64 = 3579545.45;

fn main() {
    let mut gen = WaveGenerator::new();

    let mut b = BlipBuf::new((SAMPLE_RATE / 10.0) as usize);
    b.set_rates(CLOCK_RATE, SAMPLE_RATE);

    let mut wav = wave_writer().unwrap();

    while gen.sample_count < 2 * SAMPLE_RATE as i32 {
        // Generate 1/60 second each time through loop
        let clocks = (CLOCK_RATE / 60.0) as u64;

		/* We could instead run however many clocks are needed to get a fixed number
		of samples per frame:
		let samples_needed = sample_rate / 60;
		clocks = blip.clocks_needed(samples_needed);
		*/

        gen.run_wave(&mut b, clocks);
        b.end_frame(clocks);
        // Adjust for new time frame
        gen.time -= clocks;

        gen.flush_samples(&mut b, &mut wav);

        // slowly increase volume and lower pitch
        gen.volume += 100;
        gen.period += gen.period / 28 + 3;
    }
}

/// Generates a square wave
struct WaveGenerator {
    /// clock time of next delta
    time: u64,
    volume: i32,
    /// clocks between deltas
    period: u64,
    /// current amplitude in delta buffer
    amp: i32,
    /// number of samples written to the WAV file
    sample_count: i32,
    /// +1 or -1
    phase: i32,
}

impl WaveGenerator {
    fn new() -> Self {
        Self {
            time: 0,
            volume: 0,
            period: 1,
            amp: 0,
            sample_count: 0,
            phase: 1,
        }
    }

    fn run_wave(&mut self, blip: &mut BlipBuf, clocks: u64) {
        // Add deltas that fall before end time
        while self.time < clocks {
            let delta = self.phase * self.volume - self.amp;
            self.amp += delta;
            blip.add_delta(self.time, delta);
            self.phase = -self.phase;

            self.time += self.period;
        }
    }

    fn flush_samples<W: Write + Seek>(&mut self, blip: &mut BlipBuf, wav: &mut WavWriter<W>) {
        // if we only wanted 512-sample chunks, never smaller, we would do >= 512 instead of > 0.
        // Any remaining samples would be left in buffer for next time.
        while blip.samples_avail() > 0 {
            let mut buf = [0_i16; 512];

            // count is number of samples actually read (in case there were fewer than temp_size
            // samples actually available)
            let count = blip.read_samples(&mut buf, false);
            buf[0..count].iter().for_each(|s| {
                wav.write_sample(*s).unwrap();
                self.sample_count += 1;
            });
        }
    }
}

fn wave_writer() -> Result<WavWriter<BufWriter<File>>, Box<dyn Error>> {
    let w = WavWriter::create(
        "out.wav",
        WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        },
    )?;

    Ok(w)
}

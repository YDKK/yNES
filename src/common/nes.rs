use super::apu::*;
use super::cpu::*;
use super::ppu::*;
use super::rom::*;

/// PPU clocks per CPU clock
const PPU_CLOCKS_PER_CPU: u8 = 3;

/// CPU clock rate / target sample rate — used for downsampling
const CPU_CLOCK_RATE: f64 = 1_789_773.0;
const TARGET_SAMPLE_RATE: f64 = 44_100.0;
const CYCLES_PER_SAMPLE: f64 = CPU_CLOCK_RATE / TARGET_SAMPLE_RATE;

/// Maximum audio samples per frame (slightly over 44100/60 ≈ 735 + margin)
const MAX_SAMPLES_PER_FRAME: usize = 750;

pub struct Nes {
    cpu: Cpu,
    ppu: Ppu,
    rom: Rom,
    clock_count: u8,
    apu: Apu,
    last_nmi: bool,
    // Pre-allocated audio buffer to avoid per-frame Vec allocation
    audio_buf: Box<[f32; MAX_SAMPLES_PER_FRAME]>,
    // Downsampling state for clock_frame
    resample_fraction: f64,
    sample_accumulator: f64,
    sample_count: u32,
}

pub struct PadInputs {
    pub pad1: PadInput,
    pub pad2: PadInput,
}
pub struct PadInput {
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}
impl std::default::Default for PadInput {
    fn default() -> Self {
        PadInput { a: false, b: false, select: false, start: false, up: false, down: false, left: false, right: false }
    }
}

impl Nes {
    pub fn get_version() -> String {
        env!("CARGO_PKG_VERSION").into()
    }
    pub fn new(rom: &[u8]) -> Result<Self, String> {
        let rom = Rom::load(rom)?;
        let nes = Nes {
            cpu: Cpu::new(),
            ppu: Ppu::new(rom.mirroring, rom.has_chr_ram()),
            rom,
            clock_count: 0,
            apu: Apu::new(),
            last_nmi: false,
            audio_buf: Box::new([0.0; MAX_SAMPLES_PER_FRAME]),
            resample_fraction: 0.0,
            sample_accumulator: 0.0,
            sample_count: 0,
        };

        Ok(nes)
    }

    /// Single PPU-clock step. Returns (end_frame, apu_sample).
    /// Called at PPU rate (3× CPU rate).
    pub fn clock(&mut self, pad: &PadInputs) -> (bool, Option<f32>) {
        let mut apu_out = None;
        if self.clock_count == 0 {
            self.cpu.clock(&self.rom, &mut self.apu, &mut self.ppu, pad);
            let value = self.apu.clock(self.rom.get_prog());
            apu_out = Some(value);
        }

        let (end_frame, nmi) = self.ppu.clock(&self.rom);
        if nmi && nmi != self.last_nmi {
            self.cpu.nmi();
        }
        self.last_nmi = nmi;

        self.clock_count += 1;
        self.clock_count %= PPU_CLOCKS_PER_CPU;

        (end_frame, apu_out)
    }

    /// Execute one full frame worth of clocks.
    /// Returns a slice of audio samples for this frame (downsampled to 44100 Hz).
    /// The returned slice borrows from the internal buffer and is valid until the next call.
    pub fn clock_frame(&mut self, pad: &PadInputs) -> &[f32] {
        let mut sample_idx: usize = 0;

        loop {
            if self.clock_count == 0 {
                self.cpu.clock(&self.rom, &mut self.apu, &mut self.ppu, pad);

                // Clock APU inline to maintain correct timing with CPU
                let sample = self.apu.clock(self.rom.get_prog()) as f64;
                self.sample_accumulator += sample;
                self.sample_count += 1;
                self.resample_fraction += 1.0;

                if self.resample_fraction >= CYCLES_PER_SAMPLE {
                    self.resample_fraction -= CYCLES_PER_SAMPLE;
                    if sample_idx < MAX_SAMPLES_PER_FRAME {
                        let avg = (self.sample_accumulator / self.sample_count as f64) as f32;
                        self.audio_buf[sample_idx] = avg;
                        sample_idx += 1;
                    }
                    self.sample_accumulator = 0.0;
                    self.sample_count = 0;
                }
            }

            let (end_frame, nmi) = self.ppu.clock(&self.rom);
            if nmi && nmi != self.last_nmi {
                self.cpu.nmi();
            }
            self.last_nmi = nmi;

            self.clock_count += 1;
            self.clock_count %= PPU_CLOCKS_PER_CPU;

            if end_frame {
                break;
            }
        }

        &self.audio_buf[..sample_idx]
    }

    pub fn get_screen(&self) -> &[u8; 256 * 240] {
        self.ppu.get_screen()
    }
}

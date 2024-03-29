use super::apu::*;
use super::cpu::*;
use super::ppu::*;
use super::rom::*;

pub struct Nes {
  cpu: Cpu,
  ppu: Ppu,
  rom: Rom,
  clock_count: u8,
  apu: Apu,
  last_nmi: bool,
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
      ppu: Ppu::new(rom.vertical_mirroring),
      rom,
      clock_count: 0,
      apu: Apu::new(),
      last_nmi: false,
    };

    Ok(nes)
  }
  //(end_frame, apu_out)
  pub fn clock(&mut self, pad: &PadInputs) -> (bool, Option<f32>) {
    let mut apu_out = None;
    if self.clock_count == 0 {
      self.cpu.clock(&self.rom, &mut self.apu, &mut self.ppu, pad);
      let value = self.apu.clock();
      apu_out = Some(value);
    }

    let (end_frame, nmi) = self.ppu.clock(&self.rom);
    if nmi && nmi != self.last_nmi {
      self.cpu.nmi();
    }
    self.last_nmi = nmi;

    self.clock_count += 1;
    self.clock_count %= 3;

    (end_frame, apu_out)
  }
  pub fn get_screen(&self) -> &[u8; 256 * 240] {
    self.ppu.get_screen()
  }
}

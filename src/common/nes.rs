use super::cpu::*;
use super::ppu::*;
use super::rom::*;

pub struct Nes {
  cpu: Cpu,
  ppu: Ppu,
  rom: Rom,
  clock_count: u8,
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
  pub fn new(rom_path: String) -> Result<Self, String> {
    let nes = Nes { cpu: Cpu::new(), ppu: Ppu::new(), rom: Rom::open(rom_path)?, clock_count: 0 };

    Ok(nes)
  }
  pub fn clock(&mut self, pad: &PadInputs) -> bool {
    if self.clock_count % 3 == 0 {
      self.cpu.clock(&self.rom, &mut self.ppu, pad);
    }
    self.clock_count += 1;
    self.clock_count %= 3;

    let (end_frame, nmi) = self.ppu.clock(&self.rom);
    if nmi {
      self.cpu.nmi();
    }
    end_frame
  }
  pub fn get_screen(&self) -> &[u8; 256 * 240] {
    self.ppu.get_screen()
  }
}

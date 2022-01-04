use super::cpu::*;
use super::ppu::*;
use super::rom::*;

pub struct Nes {
  cpu: Cpu,
  ppu: Ppu,
  rom: Rom,
  clock_count: u8,
}

struct Pad {
  a: bool,
  b: bool,
  select: bool,
  start: bool,
  up: bool,
  down: bool,
  left: bool,
  right: bool,
}

impl Nes {
  pub fn new(rom_path: String) -> Result<Self, String> {
    let nes = Nes { cpu: Cpu::new(), ppu: Ppu::new(), rom: Rom::open(rom_path)?, clock_count: 0 };

    Ok(nes)
  }
  pub fn clock(&mut self) -> bool {
    if self.clock_count % 3 == 0 {
      self.cpu.clock(&self.rom, &mut self.ppu);
    }
    self.clock_count += 1;
    self.clock_count %= 3;

    self.ppu.clock(&self.rom)
  }
  pub fn get_screen(&self) -> &[u8; 256 * 240] {
    self.ppu.get_screen()
  }
}

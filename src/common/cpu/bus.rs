use super::super::ppu::*;
use super::super::rom::*;

struct WRam {
  memory: Box<[u8; 0x800]>,
}

impl WRam {
  fn read(&self, addr: u16) -> u8 {
    self.memory[addr as usize]
  }
  fn write(&mut self, addr: u16, value: u8) {
    self.memory[addr as usize] = value
  }
}

pub struct Bus {
  w_ram: WRam,
}

impl Bus {
  pub fn new() -> Bus {
    Bus { w_ram: WRam { memory: Box::new([0; 0x800]) } }
  }
  pub fn read(&self, rom: Option<&Rom>, ppu: &mut Option<&mut Ppu>, addr: u16) -> u8 {
    match addr {
      0x0000..=0x1FFF => {
        let addr = addr & 0x07FF;
        self.w_ram.read(addr)
      }
      0x2000..=0x3FFF => {
        //PPU
        let addr = (addr & 0x7) as u8;
        ppu.as_mut().unwrap().read(rom.unwrap(), addr)
      }
      0x4000..=0x401F => {
        //APU, PAD
        todo!()
      }
      0x4020..=0x5FFF => {
        //拡張ROM
        todo!()
      }
      0x6000..=0x7FFF => {
        //拡張RAM
        todo!()
      }
      0x8000..=0xFFFF => {
        //PRG-ROM
        let mut addr = addr - 0x8000;
        if rom.unwrap().get_prog().len() == 0x4000 {
          addr = addr - 0x4000;
        }
        rom.unwrap().get_prog()[addr as usize]
      }
    }
  }
  pub fn write(&mut self, ppu: &mut Option<&mut Ppu>, addr: u16, value: u8) {
    match addr {
      0x0000..=0x1FFF => {
        let addr = addr & 0x07FF;
        self.w_ram.write(addr, value);
      }
      0x2000..=0x3FFF => {
        //PPU
        let addr = (addr & 0x7) as u8;
        ppu.as_mut().unwrap().write(addr, value);
      }
      0x4000..=0x401F => {
        //APU, PAD
        todo!()
      }
      0x4020..=0x5FFF => {
        //拡張ROM
        todo!()
      }
      0x6000..=0x7FFF => {
        //拡張RAM
        todo!()
      }
      0x8000..=0xFFFF => {
        //PRG-ROM
        panic!()
      }
    }
  }
}

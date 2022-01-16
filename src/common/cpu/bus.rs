use super::super::apu::*;
use super::super::nes::{PadInput, PadInputs};
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

struct Pad {
  read_cycle: u8,
  strobe: bool,
}

impl Pad {
  fn read(&mut self, input: &PadInput) -> u8 {
    if self.strobe {
      self.read_cycle = 0;
    } else {
      self.read_cycle %= 8;
    }
    let result = if match self.read_cycle {
      0 => input.a,
      1 => input.b,
      2 => input.select,
      3 => input.start,
      4 => input.up,
      5 => input.down,
      6 => input.left,
      7 => input.right,
      _ => panic!(),
    } {
      0b1
    } else {
      0b0
    };
    self.read_cycle += 1;
    result
  }
  fn set_strobe(&mut self, value: bool) {
    self.strobe = value;
    if value {
      self.read_cycle = 0;
    }
  }
}

pub struct Bus {
  w_ram: WRam,
  pad1: Pad,
  pad2: Pad,
}

impl Bus {
  pub fn new() -> Bus {
    Bus {
      w_ram: WRam { memory: Box::new([0; 0x800]) },
      pad1: Pad { read_cycle: 0, strobe: false },
      pad2: Pad { read_cycle: 0, strobe: false },
    }
  }
  pub fn read(
    &mut self,
    rom: Option<&Rom>,
    apu: &mut Option<&mut Apu>,
    ppu: &mut Option<&mut Ppu>,
    inputs: Option<&PadInputs>,
    addr: u16,
  ) -> u8 {
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
      0x4000..=0x4015 => {
        //APU
        let addr = addr as u8;
        apu.as_mut().unwrap().read(addr)
      }
      0x4016 => self.pad1.read(&inputs.unwrap().pad1),
      0x4017 => self.pad2.read(&inputs.unwrap().pad2),
      0x4018..=0x401F => todo!(), //?
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
        if addr >= 0x4000 && rom.unwrap().get_prog().len() == 0x4000 {
          addr -= 0x4000;
        }
        rom.unwrap().get_prog()[addr as usize]
      }
    }
  }
  pub fn write(
    &mut self,
    rom: Option<&Rom>,
    apu: &mut Option<&mut Apu>,
    ppu: &mut Option<&mut Ppu>,
    addr: u16,
    value: u8,
  ) -> u16 {
    match addr {
      0x0000..=0x1FFF => {
        let addr = addr & 0x07FF;
        self.w_ram.write(addr, value);
      }
      0x2000..=0x3FFF => {
        //PPU
        let addr = (addr & 0xFF) as u8;
        ppu.as_mut().unwrap().write(addr, value);
      }
      0x4014 => {
        //DMA
        let addr = (value as u16) << 8;
        let vec = (addr..=addr + 0xFF)
          .map(|x| self.read(rom, apu, ppu, None, x))
          .collect::<Vec<u8>>();
        let data = vec.as_slice().try_into().unwrap();
        ppu.as_mut().unwrap().dma_write(data);
        return 513;
      }
      0x4016 => {
        self.pad1.set_strobe((value & 0b1) == 0b1);
        self.pad2.set_strobe((value & 0b1) == 0b1);
      }
      0x4000..=0x4017 => {
        //APU
        let addr = addr as u8;
        apu.as_mut().unwrap().write(addr, value);
      }
      0x4018..=0x401F => todo!(), //?
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
    0
  }
}

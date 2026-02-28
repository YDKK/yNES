use super::super::apu::*;
use super::super::nes::{PadInput, PadInputs};
use super::super::ppu::*;
use super::super::rom::*;

struct WRam {
    memory: Box<[u8; 0x800]>,
}

impl WRam {
    #[inline(always)]
    fn read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }
    #[inline(always)]
    fn write(&mut self, addr: u16, value: u8) {
        self.memory[addr as usize] = value
    }
}

/// Extended RAM ($6000-$7FFF) for battery-backed save RAM and mapper work RAM
struct ExtRam {
    memory: Box<[u8; 0x2000]>,
}

impl ExtRam {
    fn new() -> Self {
        ExtRam { memory: Box::new([0; 0x2000]) }
    }
    #[inline(always)]
    fn read(&self, addr: u16) -> u8 {
        self.memory[(addr - 0x6000) as usize]
    }
    #[inline(always)]
    fn write(&mut self, addr: u16, value: u8) {
        self.memory[(addr - 0x6000) as usize] = value;
    }
}

struct Pad {
    read_cycle: u8,
    strobe: bool,
}

impl Pad {
    #[inline(always)]
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
    ext_ram: ExtRam,
    pad1: Pad,
    pad2: Pad,
}

impl Bus {
    pub fn new() -> Bus {
        Bus {
            w_ram: WRam { memory: Box::new([0; 0x800]) },
            ext_ram: ExtRam::new(),
            pad1: Pad { read_cycle: 0, strobe: false },
            pad2: Pad { read_cycle: 0, strobe: false },
        }
    }
    #[inline(always)]
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
            0x4018..=0x401F => 0,                       // Open bus / test mode
            0x4020..=0x5FFF => 0,                       // 拡張ROM - return open bus
            0x6000..=0x7FFF => self.ext_ram.read(addr), //拡張RAM

            0x8000..=0xFFFF => {
                //PRG-ROM
                let r = rom.unwrap();
                let prog = r.get_prog();
                let mut offset = (addr - 0x8000) as usize;
                if offset >= prog.len() && !prog.is_empty() {
                    offset %= prog.len();
                }
                if prog.is_empty() {
                    0
                } else {
                    prog[offset]
                }
            }
        }
    }
    pub fn write(&mut self, apu: &mut Option<&mut Apu>, ppu: &mut Option<&mut Ppu>, addr: u16, value: u8) -> u16 {
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
                // Need to read from our own bus - but we can't borrow rom/apu/ppu again
                // So we'll read WRAM directly for DMA from page 0/1, or ROM for high pages
                let mut data = [0u8; 0x100];
                for i in 0..=0xFFu16 {
                    let src = addr + i;
                    data[i as usize] = match src {
                        0x0000..=0x1FFF => self.w_ram.read(src & 0x07FF),
                        0x6000..=0x7FFF => self.ext_ram.read(src),
                        _ => 0, // DMA from other sources is uncommon
                    };
                }
                ppu.as_mut().unwrap().dma_write(&data);
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
            0x4018..=0x401F => {}                               // Test mode
            0x4020..=0x5FFF => {}                               //拡張ROM
            0x6000..=0x7FFF => self.ext_ram.write(addr, value), //拡張RAM
            0x8000..=0xFFFF => {} // PRG-ROM writes (used by mappers, ignored for mapper 0)
        }
        0
    }
}

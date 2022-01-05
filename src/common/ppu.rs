use super::rom::*;
use super::util::*;

struct VRam {
  // pattern_table_0: [u8; 0x1000],
  // pattern_table_1: [u8; 0x1000],
  name_table_0: Box<[u8; 0x3C0]>,
  attribute_table_0: Box<[u8; 0x40]>,
  name_table_1: Box<[u8; 0x3C0]>,
  attribute_table_1: Box<[u8; 0x40]>,
  name_table_2: Box<[u8; 0x3C0]>,
  attribute_table_2: Box<[u8; 0x40]>,
  name_table_3: Box<[u8; 0x3C0]>,
  attribute_table_3: Box<[u8; 0x40]>,
  background_palette: Box<[u8; 0x10]>,
  sprite_palette: Box<[u8; 0x10]>,
  sprite_memory: Box<[u8; 0x100]>,
}

impl VRam {
  fn read(&self, rom: &Rom, addr: u16) -> u8 {
    let addr = addr as usize;
    match addr {
      0x0000..=0x0FFF => rom.get_chr()[addr],
      0x1000..=0x1FFF => rom.get_chr()[addr - 0x1000],
      0x2000..=0x23BF => self.name_table_0[addr - 0x2000],
      0x23C0..=0x23FF => self.attribute_table_0[addr - 0x23C0],
      0x2400..=0x27BF => self.name_table_1[addr - 0x2400],
      0x27C0..=0x27FF => self.attribute_table_1[addr - 0x27C0],
      0x2800..=0x2BBF => self.name_table_2[addr - 0x2800],
      0x2BC0..=0x2BFF => self.attribute_table_2[addr - 0x2BC0],
      0x2C00..=0x2FBF => self.name_table_3[addr - 0x2C00],
      0x2FC0..=0x2FFF => self.attribute_table_3[addr - 0x2FC0],
      0x3000..=0x3EFF => self.read(rom, (addr - 0x1000) as u16),
      0x3F00..=0x3F0F => self.background_palette[addr - 0x3F00],
      0x3F10..=0x3F1F => self.sprite_palette[addr - 0x3F10],
      0x3F20..=0x3FFF => self.read(rom, (addr - 0x0020) as u16),
      _ => panic!(),
    }
  }
  fn write(&mut self, addr: u16, value: u8) {
    let addr = addr as usize;
    match addr {
      0x2000..=0x23BF => self.name_table_0[addr - 0x2000] = value,
      0x23C0..=0x23FF => self.attribute_table_0[addr - 0x23C0] = value,
      0x2400..=0x27BF => self.name_table_1[addr - 0x2400] = value,
      0x27C0..=0x27FF => self.attribute_table_1[addr - 0x27C0] = value,
      0x2800..=0x2BBF => self.name_table_2[addr - 0x2800] = value,
      0x2BC0..=0x2BFF => self.attribute_table_2[addr - 0x2BC0] = value,
      0x2C00..=0x2FBF => self.name_table_3[addr - 0x2C00] = value,
      0x2FC0..=0x2FFF => self.attribute_table_3[addr - 0x2FC0] = value,
      0x3000..=0x3EFF => self.write((addr - 0x1000) as u16, value),
      0x3F00..=0x3F0F => self.background_palette[addr - 0x3F00] = value,
      0x3F10..=0x3F1F => self.sprite_palette[addr - 0x3F10] = value,
      0x3F20..=0x3FFF => self.write((addr - 0x0020) as u16, value),
      _ => panic!(),
    }
  }
}

struct ControlRegister {
  nmi_on_v_blank: bool,
  ppu_select: bool,
  sprite_size: bool, //One
  bg_pattern_table: bool,
  sprite_chr_table: bool,
  v_ram_io_addressing: bool,
  main_screen: u8,
}
impl ControlRegister {
  fn read(&self) -> u8 {
    let mut value: u8 = 0;
    if self.nmi_on_v_blank {
      value |= 0b1000_0000;
    }
    if self.ppu_select {
      value |= 0b0100_0000;
    }
    if self.sprite_size {
      value |= 0b0010_0000;
    }
    if self.bg_pattern_table {
      value |= 0b0001_0000;
    }
    if self.sprite_chr_table {
      value |= 0b0000_1000;
    }
    if self.v_ram_io_addressing {
      value |= 0b0000_0100;
    }
    value |= self.main_screen;
    value
  }
  fn write(&mut self, value: u8) {
    self.nmi_on_v_blank = (value & 0b1000_0000) == 0b1000_0000;
    self.ppu_select = (value & 0b0100_0000) == 0b0100_0000;
    self.sprite_size = (value & 0b0010_0000) == 0b0010_0000;
    self.bg_pattern_table = (value & 0b0001_0000) == 0b0001_0000;
    self.sprite_chr_table = (value & 0b0000_1000) == 0b0000_1000;
    self.v_ram_io_addressing = (value & 0b0000_0100) == 0b0000_0100;
    self.main_screen = value & 0b0000_0011;
  }
}

struct ControlRegister2 {
  color_emphasis_red: bool,
  color_emphasis_green: bool,
  color_emphasis_blue: bool,
  show_sprite: bool,
  show_bg: bool,
  show_left_column_sprite: bool,
  show_left_column_bg: bool,
  monochrome: bool,
}
impl ControlRegister2 {
  fn read(&self) -> u8 {
    let mut value: u8 = 0;
    if self.color_emphasis_red {
      value |= 0b1000_0000;
    }
    if self.color_emphasis_green {
      value |= 0b0100_0000;
    }
    if self.color_emphasis_blue {
      value |= 0b0010_0000;
    }
    if self.show_sprite {
      value |= 0b0001_0000;
    }
    if self.show_bg {
      value |= 0b0000_1000;
    }
    if self.show_left_column_sprite {
      value |= 0b0000_0100;
    }
    if self.show_left_column_bg {
      value |= 0b0000_0010;
    }
    if self.monochrome {
      value |= 0b0000_0001;
    }
    value
  }
  fn write(&mut self, value: u8) {
    self.color_emphasis_red = (value & 0b1000_0000) == 0b1000_0000;
    self.color_emphasis_green = (value & 0b0100_0000) == 0b0100_0000;
    self.color_emphasis_blue = (value & 0b0010_0000) == 0b0010_0000;
    self.show_sprite = (value & 0b0001_0000) == 0b0001_0000;
    self.show_bg = (value & 0b0000_1000) == 0b0000_1000;
    self.show_left_column_sprite = (value & 0b0000_0100) == 0b0000_0100;
    self.show_left_column_bg = (value & 0b0000_0010) == 0b0000_0010;
    self.monochrome = (value & 0b0000_0001) == 0b0000_0001;
  }
}
struct StatusRegister {
  v_blank: bool,
  sprite_0_hit: bool,
  sprite_overflow: bool,
}
impl StatusRegister {
  fn read(&self) -> u8 {
    let mut value: u8 = 0;
    if self.v_blank {
      value |= 0b1000_0000;
    }
    if self.sprite_0_hit {
      value |= 0b0100_0000;
    }
    if self.sprite_overflow {
      value |= 0b0010_0000;
    }
    value
  }
}

struct Registers {
  control_register: ControlRegister,
  control_register2: ControlRegister2,
  status_register: StatusRegister,
}

struct Bus {
  v_ram: VRam,
}

enum State {
  Idle,
  Writing,
}

pub struct Ppu {
  bus: Bus,
  registers: Registers,
  sprite_addr: u8,
  scroll_horizontal: u8,
  scroll_vertical: u8,
  v_ram_addr_h: u8,
  v_ram_addr_l: u8,
  state: State,
  current_x: u16,
  current_y: u16,
  frame: [u8; 256 * 240],
}

struct Sprite {
  pattern: u8,
  background: bool,
  pallet: u8,
  index: u8,
}

impl Ppu {
  pub fn new() -> Self {
    let ppu = Ppu {
      bus: Bus {
        v_ram: VRam {
          name_table_0: Box::new([0; 0x3C0]),
          attribute_table_0: Box::new([0; 0x40]),
          name_table_1: Box::new([0; 0x3C0]),
          attribute_table_1: Box::new([0; 0x40]),
          name_table_2: Box::new([0; 0x3C0]),
          attribute_table_2: Box::new([0; 0x40]),
          name_table_3: Box::new([0; 0x3C0]),
          attribute_table_3: Box::new([0; 0x40]),
          background_palette: Box::new([0; 0x10]),
          sprite_palette: Box::new([0; 0x10]),
          sprite_memory: Box::new([0; 0x100]), //?
        },
      },
      registers: Registers {
        control_register: ControlRegister {
          nmi_on_v_blank: false,
          ppu_select: false,
          sprite_size: true, //One
          bg_pattern_table: false,
          sprite_chr_table: false,
          v_ram_io_addressing: false,
          main_screen: 0,
        },
        control_register2: ControlRegister2 {
          color_emphasis_red: false,
          color_emphasis_green: false,
          color_emphasis_blue: false,
          show_sprite: false,
          show_bg: false,
          show_left_column_sprite: false,
          show_left_column_bg: false,
          monochrome: false,
        },
        status_register: StatusRegister { v_blank: false, sprite_0_hit: false, sprite_overflow: false },
      },
      sprite_addr: 0,
      scroll_horizontal: 0,
      scroll_vertical: 0,
      v_ram_addr_h: 0,
      v_ram_addr_l: 0,
      state: State::Idle,
      current_x: 0,
      current_y: 0,
      frame: [0; 256 * 240],
    };
    ppu
  }

  fn get_pattern(&self, rom: &Rom, pattern: u8, pixel_in_tile_x: u16, pixel_in_tile_y: u16) -> u8 {
    let pattern_addr_l = if self.registers.control_register.bg_pattern_table {
      0x1000
    } else {
      0x0000
    } + ((pattern as u16) << 4)
      + pixel_in_tile_y;
    let pattern_addr_h = pattern_addr_l + 0x08;
    let pattern_l = self.bus.v_ram.read(rom, pattern_addr_l);
    let pattern_h = self.bus.v_ram.read(rom, pattern_addr_h);
    ((pattern_l >> (7 - pixel_in_tile_x)) & 0b01) | (((pattern_h >> (7 - pixel_in_tile_x)) << 1) & 0b10)
  }

  pub fn clock(&mut self, rom: &Rom) -> (bool, bool) {
    let mut nmi = false;
    match self.current_y {
      0..=239 => match self.current_x {
        0..=255 => {
          let tile_x = self.current_x / 8;
          let tile_y = self.current_y / 8;
          let tile_y_addr = tile_y * 0x20;
          let pixel_in_tile_x = self.current_x % 8;
          let pixel_in_tile_y = self.current_y % 8;

          //スプライト
          let mut sprite_index: u8 = 0;
          let sprite = if self.registers.control_register2.show_sprite
            && (self.current_x >= 8 || self.registers.control_register2.show_left_column_sprite)
          {
            loop {
              let addr = (sprite_index as usize) * 4;
              let sprite_y = self.bus.v_ram.sprite_memory[addr].saturating_add(1);
              if (sprite_y / 8) == (tile_y as u8) {
                let sprite_x = self.bus.v_ram.sprite_memory[addr + 3];
                if (sprite_x / 8) == (tile_x as u8) {
                  let sprite_tile = self.bus.v_ram.sprite_memory[addr + 1];
                  let sprite_attr = self.bus.v_ram.sprite_memory[addr + 2];
                  //垂直反転
                  let pixel_in_tile_y = if sprite_attr & 0b1000_0000 == 0b1000_0000 {
                    7 - pixel_in_tile_y
                  } else {
                    pixel_in_tile_y
                  };
                  //水平反転
                  let pixel_in_tile_x = if sprite_attr & 0b0100_0000 == 0b0100_0000 {
                    7 - pixel_in_tile_x
                  } else {
                    pixel_in_tile_x
                  };

                  let pattern = self.get_pattern(rom, sprite_tile, pixel_in_tile_x, pixel_in_tile_y);
                  if pattern != 0 {
                    let background = sprite_attr & 0b0010_0000 == 0b0010_0000;
                    let pallet = sprite_attr & 0b11;
                    break Some(Sprite { pattern, background, pallet, index: sprite_index });
                  }
                }
              }
              sprite_index += 1;
              if sprite_index == 64 {
                break None;
              }
            }
          } else {
            None
          };

          //BG
          let mut pattern = if self.registers.control_register2.show_bg
            && (self.current_x >= 8 || self.registers.control_register2.show_left_column_bg)
          {
            let name_base_addr = match self.registers.control_register.main_screen {
              0 => 0x2000,
              1 => 0x2400,
              2 => 0x2800,
              3 => 0x2C00,
              _ => panic!(),
            };
            let name_addr = name_base_addr + tile_y_addr + tile_x;
            let name = self.bus.v_ram.read(rom, name_addr);
            self.get_pattern(rom, name, pixel_in_tile_x, pixel_in_tile_y)
          } else {
            0
          };

          if self.current_x != 255
            && sprite.is_some()
            && sprite.as_ref().unwrap().index == 0
            && pattern != 0
            && sprite.as_ref().unwrap().pattern != 0
          {
            //sprite 0 hit
            self.registers.status_register.sprite_0_hit = true;
          }

          let pallet = if sprite.is_none() || sprite.as_ref().unwrap().background {
            //BGを描画する
            let attribute_block_x = tile_x / 4;
            let attribute_block_y = tile_y_addr / 4;
            let attribute_base_addr = match self.registers.control_register.main_screen {
              0 => 0x23C0,
              1 => 0x27C0,
              2 => 0x2BC0,
              3 => 0x2FC0,
              _ => panic!(),
            };
            let attribute_addr = attribute_base_addr + attribute_block_y + attribute_block_x;
            let attribute = self.bus.v_ram.read(rom, attribute_addr);
            let block = ((self.current_x / 32) % 2) + (((self.current_y / 32) % 2) * 2);
            (attribute >> (block * 2)) & 0b11
          } else {
            //スプライトを描画する
            pattern = sprite.as_ref().unwrap().pattern;
            sprite.as_ref().unwrap().pallet
          };

          let pallet_addr = 0x3F00 + ((pallet as u16) << 2);
          let colors = [
            self.bus.v_ram.read(rom, pallet_addr),
            self.bus.v_ram.read(rom, pallet_addr + 1),
            self.bus.v_ram.read(rom, pallet_addr + 2),
            self.bus.v_ram.read(rom, pallet_addr + 3),
          ];
          let pixel = colors[pattern as usize];

          self.frame[(self.current_y * 256 + self.current_x) as usize] = pixel;
        }
        256..=340 => {} //Hblank
        _ => panic!(),
      },
      240 => {} //post-render
      241 => {
        //Vblank
        if self.current_x == 0 {
          self.registers.status_register.v_blank = true;
          nmi = self.registers.control_register.nmi_on_v_blank;
        }
      }
      242..=260 => {} //Vblank
      261 => {
        if self.current_x == 0 {
          self.registers.status_register.sprite_0_hit = false;
        }
      } //pre-render scanline
      _ => panic!(),
    }
    self.current_x += 1;
    if self.current_x > 340 {
      self.current_x = 0;
      self.current_y += 1;
      self.current_y %= 262;
    }
    (self.current_x == 0 && self.current_y == 0, nmi)
  }
  pub fn get_screen(&self) -> &[u8; 256 * 240] {
    &self.frame
  }
  pub fn read(&mut self, rom: &Rom, addr: u8) -> u8 {
    match addr {
      0x02 => {
        self.state = State::Idle;
        let result = self.registers.status_register.read();
        self.registers.status_register.v_blank = false;
        result
      }
      0x04 => {
        let result = self.bus.v_ram.sprite_memory[self.sprite_addr as usize];
        self.sprite_addr = self.sprite_addr.wrapping_add(1); //TODO?
        result
      }
      0x07 => {
        let mut addr = get_addr(self.v_ram_addr_h, self.v_ram_addr_l);
        let result = self.bus.v_ram.read(rom, addr);
        addr = addr.wrapping_add(if self.registers.control_register.v_ram_io_addressing {
          32
        } else {
          1
        });
        self.v_ram_addr_h = ((addr & 0xFF00) >> 8) as u8;
        self.v_ram_addr_l = addr as u8;
        result
      }
      _ => panic!(),
    }
  }
  pub fn write(&mut self, addr: u8, value: u8) {
    match addr {
      0x00 => self.registers.control_register.write(value),
      0x01 => self.registers.control_register2.write(value),
      0x03 => self.sprite_addr = value,
      0x04 => {
        self.bus.v_ram.sprite_memory[self.sprite_addr as usize] = value;
        self.sprite_addr = self.sprite_addr.wrapping_add(1); //TODO?
      }
      0x05 => match self.state {
        State::Idle => {
          self.scroll_horizontal = value;
          self.state = State::Writing;
        }
        State::Writing => {
          self.scroll_vertical = value;
          self.state = State::Idle;
        }
      },
      0x06 => match self.state {
        State::Idle => {
          self.v_ram_addr_h = value;
          self.state = State::Writing;
        }
        State::Writing => {
          self.v_ram_addr_l = value;
          self.state = State::Idle;
        }
      },
      0x07 => {
        let mut addr = get_addr(self.v_ram_addr_h, self.v_ram_addr_l);
        self.bus.v_ram.write(addr, value);
        addr = addr.wrapping_add(if self.registers.control_register.v_ram_io_addressing {
          32
        } else {
          1
        });
        self.v_ram_addr_h = ((addr & 0xFF00) >> 8) as u8;
        self.v_ram_addr_l = addr as u8;
      } //VRAM
      0x14 => todo!(), //DMA
      _ => panic!(),
    }
  }
}

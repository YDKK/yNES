use super::rom::*;
use super::util::*;

struct VRam {
    name_table: Box<[u8; 0x1000]>, // 4 nametables (used for 4-screen; 2KB mirrored otherwise)
    background_palette: Box<[u8; 0x10]>,
    sprite_palette: Box<[u8; 0x10]>,
    sprite_memory: Box<[u8; 0x100]>,
    mirroring: MirroringMode,
    chr_ram: Option<Box<[u8; 0x2000]>>,
}

impl VRam {
    /// Resolve a nametable address ($2000-$2FFF) to an index into name_table[0x1000]
    #[inline(always)]
    fn resolve_nametable_addr(&self, addr: u16) -> usize {
        let offset = (addr - 0x2000) as usize;
        let table = offset / 0x400; // 0-3
        let within = offset % 0x400;
        let physical_table = match self.mirroring {
            MirroringMode::Horizontal => match table {
                0 | 1 => 0,
                2 | 3 => 1,
                _ => unreachable!(),
            },
            MirroringMode::Vertical => match table {
                0 | 2 => 0,
                1 | 3 => 1,
                _ => unreachable!(),
            },
            MirroringMode::SingleScreenLower => 0,
            MirroringMode::SingleScreenUpper => 1,
            MirroringMode::FourScreen => table,
        };
        physical_table * 0x400 + within
    }

    fn read(&self, rom: &Rom, addr: u16) -> u8 {
        let addr = addr & 0x3FFF; // Mirror above $3FFF
        match addr {
            0x0000..=0x1FFF => {
                if let Some(ref ram) = self.chr_ram {
                    ram[addr as usize]
                } else {
                    let chr = rom.get_chr();
                    if chr.is_empty() {
                        return 0;
                    }
                    if chr.len() == 0x1000 {
                        chr[(addr as usize) & 0x0FFF]
                    } else {
                        chr[addr as usize % chr.len()]
                    }
                }
            }
            0x2000..=0x2FFF => {
                let idx = self.resolve_nametable_addr(addr);
                self.name_table[idx]
            }
            0x3000..=0x3EFF => self.read(rom, addr - 0x1000),
            0x3F00..=0x3FFF => {
                let paddr = ((addr - 0x3F00) % 0x20) as usize;
                match paddr {
                    0x10 | 0x14 | 0x18 | 0x1C => self.background_palette[paddr - 0x10],
                    0x00..=0x0F => self.background_palette[paddr],
                    0x10..=0x1F => self.sprite_palette[paddr - 0x10],
                    _ => 0,
                }
            }
            _ => 0,
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        let addr = addr & 0x3FFF;
        match addr {
            0x0000..=0x1FFF => {
                if let Some(ref mut ram) = self.chr_ram {
                    ram[addr as usize] = value;
                }
            }
            0x2000..=0x2FFF => {
                let idx = self.resolve_nametable_addr(addr);
                self.name_table[idx] = value;
            }
            0x3000..=0x3EFF => self.write(addr - 0x1000, value),
            0x3F00..=0x3FFF => {
                let paddr = ((addr - 0x3F00) % 0x20) as usize;
                match paddr {
                    0x10 | 0x14 | 0x18 | 0x1C => self.background_palette[paddr - 0x10] = value,
                    0x00..=0x0F => self.background_palette[paddr] = value,
                    0x10..=0x1F => self.sprite_palette[paddr - 0x10] = value,
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

struct ControlRegister {
    nmi_on_v_blank: bool,
    ppu_select: bool,
    sprite_size: bool,
    bg_pattern_table: bool,
    sprite_chr_table: bool,
    v_ram_io_addressing: bool,
    main_screen: u8,
}
impl ControlRegister {
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    frame: Box<[u8; 256 * 240]>,
    read_buffer: u8,
}

struct Sprite {
    pattern: u8,
    background: bool,
    pallet: u8,
    index: u8,
}

impl Ppu {
    pub fn new(mirroring: MirroringMode, has_chr_ram: bool) -> Self {
        Ppu {
            bus: Bus {
                v_ram: VRam {
                    name_table: Box::new([0; 0x1000]),
                    background_palette: Box::new([0; 0x10]),
                    sprite_palette: Box::new([0; 0x10]),
                    sprite_memory: Box::new([0; 0x100]),
                    mirroring,
                    chr_ram: if has_chr_ram { Some(Box::new([0; 0x2000])) } else { None },
                },
            },
            registers: Registers {
                control_register: ControlRegister {
                    nmi_on_v_blank: false,
                    ppu_select: true,
                    sprite_size: false,
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
            frame: Box::new([0; 256 * 240]),
            read_buffer: 0,
        }
    }

    #[inline(always)]
    fn get_pattern(&self, rom: &Rom, pattern: u8, pixel_in_tile_x: u16, pixel_in_tile_y: u16, is_sprite: bool) -> u8 {
        let pattern_addr_l = if if is_sprite {
            self.registers.control_register.sprite_chr_table
        } else {
            self.registers.control_register.bg_pattern_table
        } {
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
            0..=239 => {
                if self.current_x <= 255 {
                    let mut scrolled_x = self.current_x + self.scroll_horizontal as u16;
                    let screen_overwrap_x = if scrolled_x > 255 {
                        scrolled_x -= 256;
                        true
                    } else {
                        false
                    };
                    let mut scrolled_y = self.current_y + self.scroll_vertical as u16;
                    let screen_overwrap_y = if scrolled_y > 239 {
                        scrolled_y -= 240;
                        true
                    } else {
                        false
                    };
                    let mut main_screen = self.registers.control_register.main_screen;
                    if screen_overwrap_x {
                        if main_screen % 2 == 0 {
                            main_screen += 1;
                        } else {
                            main_screen -= 1;
                        }
                    }
                    if screen_overwrap_y {
                        if main_screen / 2 == 0 {
                            main_screen += 2;
                        } else {
                            main_screen -= 2;
                        }
                    }

                    let tile_x = scrolled_x / 8;
                    let tile_y = scrolled_y / 8;
                    let pixel_in_tile_x = scrolled_x % 8;
                    let pixel_in_tile_y = scrolled_y % 8;

                    //スプライト
                    let mut sprite_index: u8 = 0;
                    let sprite = if self.registers.control_register2.show_sprite
                        && (self.current_x >= 8 || self.registers.control_register2.show_left_column_sprite)
                    {
                        loop {
                            let addr = (sprite_index as usize) * 4;
                            let sprite_y = self.bus.v_ram.sprite_memory[addr].saturating_add(1);
                            if (sprite_y as u16 <= self.current_y) && (sprite_y as u16 + 7 >= self.current_y) {
                                let sprite_x = self.bus.v_ram.sprite_memory[addr + 3];
                                if (sprite_x as u16 <= self.current_x) && (sprite_x as u16 + 7 >= self.current_x) {
                                    let sprite_tile = self.bus.v_ram.sprite_memory[addr + 1];
                                    let sprite_attr = self.bus.v_ram.sprite_memory[addr + 2];
                                    //垂直反転
                                    let mut pixel_in_tile_y = (self.current_y - (sprite_y as u16)) % 8;
                                    if (sprite_attr & 0b1000_0000) == 0b1000_0000 {
                                        pixel_in_tile_y = 7 - pixel_in_tile_y;
                                    }
                                    //水平反転
                                    let mut pixel_in_tile_x = (self.current_x - (sprite_x as u16)) % 8;
                                    if (sprite_attr & 0b0100_0000) == 0b0100_0000 {
                                        pixel_in_tile_x = 7 - pixel_in_tile_x;
                                    }

                                    let pattern =
                                        self.get_pattern(rom, sprite_tile, pixel_in_tile_x, pixel_in_tile_y, true);
                                    if pattern != 0 {
                                        let background = (sprite_attr & 0b0010_0000) == 0b0010_0000;
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
                        let name_base_addr = 0x2000 + (main_screen as u16) * 0x400;
                        let tile_y_addr = tile_y * 0x20;
                        let name_addr = name_base_addr + tile_y_addr + tile_x;
                        let name = self.bus.v_ram.read(rom, name_addr);
                        self.get_pattern(rom, name, pixel_in_tile_x, pixel_in_tile_y, false)
                    } else {
                        0
                    };

                    if (self.current_x != 255)
                        && sprite.is_some()
                        && (sprite.as_ref().unwrap().index == 0)
                        && (pattern != 0)
                        && (sprite.as_ref().unwrap().pattern != 0)
                    {
                        //sprite 0 hit
                        self.registers.status_register.sprite_0_hit = true;
                    }

                    let pallet_addr = if sprite.is_none() || (pattern != 0 && sprite.as_ref().unwrap().background) {
                        //BGを描画する
                        let attribute_block_x = tile_x / 4;
                        let attribute_block_y_addr = tile_y / 4 * 8;
                        let attribute_base_addr = 0x23C0 + (main_screen as u16) * 0x400;
                        let attribute_addr = attribute_base_addr + attribute_block_y_addr + attribute_block_x;
                        let attribute = self.bus.v_ram.read(rom, attribute_addr);
                        let block = ((tile_x / 2) % 2) + (((tile_y / 2) % 2) * 2);
                        0x3F00 + ((((attribute >> (block * 2)) & 0b11) as u16) << 2)
                    } else {
                        //スプライトを描画する
                        pattern = sprite.as_ref().unwrap().pattern;
                        0x3F10 + ((sprite.as_ref().unwrap().pallet as u16) << 2)
                    };

                    let colors = [
                        self.bus.v_ram.read(rom, 0x3F00), // Universal BG color (always $3F00 when pattern==0)
                        self.bus.v_ram.read(rom, pallet_addr + 1),
                        self.bus.v_ram.read(rom, pallet_addr + 2),
                        self.bus.v_ram.read(rom, pallet_addr + 3),
                    ];
                    let pixel = colors[pattern as usize];

                    self.frame[(self.current_y * 256 + self.current_x) as usize] = pixel;
                }
                // 256..=340 => {} //Hblank
            }
            240 => {} //post-render
            241 => {
                //Vblank
                if self.current_x == 1 {
                    self.registers.status_register.v_blank = true;
                }
                nmi = self.registers.control_register.nmi_on_v_blank && self.registers.status_register.v_blank;
            }
            242..=260 => {
                //Vblank
                nmi = self.registers.control_register.nmi_on_v_blank && self.registers.status_register.v_blank;
            }
            261 => {
                //pre-render scanline
                if self.current_x == 1 {
                    self.registers.status_register.sprite_0_hit = false;
                    self.registers.status_register.v_blank = false;
                }
            }
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
                // self.sprite_addr = self.sprite_addr.wrapping_add(1); //TODO?
                result
            }
            0x07 => {
                let mut result = self.read_buffer;
                let mut addr = get_addr(self.v_ram_addr_h, self.v_ram_addr_l);
                self.read_buffer = self.bus.v_ram.read(rom, addr);
                if (0x3F00..=0x3FFF).contains(&addr) {
                    result = self.read_buffer;
                    self.read_buffer = self.bus.v_ram.read(rom, addr - 0x1000);
                }
                addr = addr.wrapping_add(if self.registers.control_register.v_ram_io_addressing {
                    32
                } else {
                    1
                });
                self.v_ram_addr_h = ((addr & 0xFF00) >> 8) as u8;
                self.v_ram_addr_l = addr as u8;
                result
            }
            _ => 0,
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
                    self.scroll_vertical &= 0b0011_1011;
                    self.scroll_vertical |= (value & 0b11) << 6;
                    self.registers.control_register.main_screen = (value & 0b00001100) >> 2;
                    self.state = State::Writing;
                }
                State::Writing => {
                    self.v_ram_addr_l = value;
                    self.scroll_horizontal &= 0b00000111;
                    self.scroll_horizontal |= (value & 0b11111) << 3;
                    self.state = State::Idle;
                }
            },
            0x07 => {
                //VRAM
                let mut addr = get_addr(self.v_ram_addr_h, self.v_ram_addr_l);
                self.bus.v_ram.write(addr, value);
                addr = addr.wrapping_add(if self.registers.control_register.v_ram_io_addressing {
                    32
                } else {
                    1
                });
                self.v_ram_addr_h = ((addr & 0xFF00) >> 8) as u8;
                self.v_ram_addr_l = addr as u8;
            }
            _ => {}
        }
    }

    pub fn dma_write(&mut self, data: &[u8; 0x100]) {
        for byte in data {
            self.bus.v_ram.sprite_memory[self.sprite_addr as usize] = *byte;
            self.sprite_addr = self.sprite_addr.wrapping_add(1);
        }
    }
}

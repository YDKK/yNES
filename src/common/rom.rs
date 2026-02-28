struct SliceInfo {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MirroringMode {
    Horizontal,
    Vertical,
    #[allow(dead_code)]
    SingleScreenLower,
    #[allow(dead_code)]
    SingleScreenUpper,
    FourScreen,
}

pub struct Rom {
    memory: Vec<u8>,
    prog: SliceInfo,
    chr: SliceInfo,
    pub mirroring: MirroringMode,
    #[allow(dead_code)]
    pub has_battery_ram: bool,
    #[allow(dead_code)]
    pub mapper: u8,
    has_chr_ram: bool,
}

impl Rom {
    pub fn load(rom: &[u8]) -> Result<Self, String> {
        if rom.len() < 16 || rom[0..=3] != [0x4E, 0x45, 0x53, 0x1A] {
            return Err(String::from("invalid file format"));
        }

        let prog_start = 0x0010_usize;
        let prog_end = prog_start + (rom[4] as usize) * 0x4000;
        let chr_start = prog_end;
        let chr_end = chr_start + (rom[5] as usize) * 0x2000;

        let flags6 = rom[6];
        let flags7 = rom[7];

        let mirroring = if flags6 & 0b1000 != 0 {
            MirroringMode::FourScreen
        } else if flags6 & 0b1 != 0 {
            MirroringMode::Vertical
        } else {
            MirroringMode::Horizontal
        };

        let has_battery_ram = flags6 & 0b10 != 0;
        let mapper = (flags6 >> 4) | (flags7 & 0xF0);

        let has_chr_ram = rom[5] == 0;

        let mut memory = vec![];
        memory.extend_from_slice(rom);

        let rom = Rom {
            memory,
            prog: SliceInfo { start: prog_start, end: prog_end },
            chr: SliceInfo { start: chr_start, end: chr_end },
            mirroring,
            has_battery_ram,
            mapper,
            has_chr_ram,
        };
        Ok(rom)
    }
    pub fn get_prog(&self) -> &[u8] {
        &self.memory[self.prog.start..self.prog.end]
    }
    pub fn get_chr(&self) -> &[u8] {
        &self.memory[self.chr.start..self.chr.end]
    }
    pub fn has_chr_ram(&self) -> bool {
        self.has_chr_ram
    }
}

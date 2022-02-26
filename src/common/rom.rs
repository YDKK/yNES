struct SliceInfo {
  start: usize,
  end: usize,
}

pub struct Rom {
  memory: Vec<u8>,
  prog: SliceInfo,
  chr: SliceInfo,
  pub vertical_mirroring: bool,
}
impl Rom {
  pub fn load<'a>(rom: &[u8]) -> Result<Self, String> {
    if rom[0..=3] != [0x4E, 0x45, 0x53, 0x1A] {
      return Err(String::from("invalid file format"));
    }

    let prog_start = 0x0010 as usize;
    let prog_end = prog_start + (rom[4] as usize) * 0x4000;
    let chr_start = prog_end;
    let chr_end = chr_start + (rom[5] as usize) * 0x2000;
    let vertical_mirroring = rom[6] & 0b1 == 0b1;
    let mut memory = vec![];
    memory.extend_from_slice(rom);
    let rom = Rom {
      memory,
      prog: SliceInfo { start: prog_start as _, end: prog_end as _ },
      chr: SliceInfo { start: chr_start as _, end: chr_end as _ },
      vertical_mirroring,
    };
    Ok(rom)
  }
  pub fn get_prog(&self) -> &[u8] {
    &self.memory[self.prog.start..self.prog.end]
  }
  pub fn get_chr(&self) -> &[u8] {
    &self.memory[self.chr.start..self.chr.end]
  }
}

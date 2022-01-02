use std::fs::File;
use std::io;
use std::io::prelude::*;

struct SliceInfo {
  start: usize,
  end: usize,
}

pub struct Rom {
  memory: Vec<u8>,
  prog: SliceInfo,
  chr: SliceInfo,
}
impl Rom {
  pub fn open<'a>(path: String) -> Result<Self, String> {
    let file = File::open(path);
    if file.is_err() {
      return Err(String::from("open file error"));
    }
    let mut contents = vec![];
    if file.unwrap().read_to_end(&mut contents).is_err() {
      return Err(String::from("read file error"));
    }

    if contents[0..=3] != [0x4E, 0x45, 0x53, 0x1A] {
      return Err(String::from("invalid file format"));
    }

    let prog_start = 0x0010 as usize;
    let prog_end = prog_start + (contents[4] as usize) * 0x4000;
    let chr_start = prog_end;
    let chr_end = chr_start + (contents[5] as usize) * 0x2000;
    let rom = Rom {
      memory: contents,
      prog: SliceInfo { start: prog_start as _, end: prog_end as _ },
      chr: SliceInfo { start: chr_start as _, end: chr_end as _ },
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

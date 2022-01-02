pub fn get_addr(addr_h: u8, addr_l: u8) -> u16 {
  (addr_h as u16) << 8 | addr_l as u16
}

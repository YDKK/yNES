use super::ppu::*;
use super::rom::*;
use super::util::*;
mod bus;

struct ProcessorStatusRegister {
  n: bool,
  v: bool,
  o: bool, //One
  b: bool,
  d: bool,
  i: bool,
  z: bool,
  c: bool,
}
impl ProcessorStatusRegister {
  fn read(&self) -> u8 {
    let mut value: u8 = 0;
    if self.n {
      value |= 0b1000_0000;
    }
    if self.v {
      value |= 0b0100_0000;
    }
    if self.o {
      value |= 0b0010_0000;
    }
    if self.b {
      value |= 0b0001_0000;
    }
    if self.d {
      value |= 0b0000_1000;
    }
    if self.i {
      value |= 0b0000_0100;
    }
    if self.z {
      value |= 0b0000_0010;
    }
    if self.c {
      value |= 0b0000_0001;
    }
    value
  }
  fn write(&mut self, value: u8) {
    self.n = (value & 0b1000_0000) == 0b1000_0000;
    self.v = (value & 0b0100_0000) == 0b0100_0000;
    self.o = (value & 0b0010_0000) == 0b0010_0000;
    self.b = (value & 0b0001_0000) == 0b0001_0000;
    self.d = (value & 0b0000_1000) == 0b0000_1000;
    self.i = (value & 0b0000_0100) == 0b0000_0100;
    self.z = (value & 0b0000_0010) == 0b0000_0010;
    self.c = (value & 0b0000_0001) == 0b0000_0001;
  }
}
pub struct Cpu {
  a: u8,
  x: u8,
  y: u8,
  pc: u16,
  sp: u8,
  p: ProcessorStatusRegister,
  bus: bus::Bus,
  op: u8,
  state: CpuState,
  step: u8,
  addr_l: u8,
  addr_h: u8,
  immediate_operand: u8,
  is_immediate: bool,
}

enum CpuState {
  Reset,
  ReadOpcode,
  ReadOperand,
  ExecuteInstruction,
}

impl std::fmt::Display for CpuState {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
    match self {
      CpuState::Reset => write!(f, "Reset"),
      CpuState::ReadOpcode => write!(f, "ReadOpcode"),
      CpuState::ReadOperand => write!(f, "ReadOperand"),
      CpuState::ExecuteInstruction => write!(f, "ExecuteInstruction"),
    }
  }
}

impl Cpu {
  pub fn new() -> Cpu {
    Cpu {
      a: 0,
      x: 0,
      y: 0,
      pc: 0,
      sp: 0xFD,
      p: ProcessorStatusRegister {
        n: false,
        v: false,
        o: true, //One
        b: true,
        d: false,
        i: true,
        z: false,
        c: false,
      },
      bus: bus::Bus::new(),
      op: 0,
      state: CpuState::Reset,
      step: 0,
      addr_l: 0,
      addr_h: 0,
      immediate_operand: 0,
      is_immediate: false,
    }
  }
  pub fn clock(&mut self, rom: &Rom, ppu: &mut Ppu) {
    println!("PC:{:x} OP:{:x} STATE:{}", self.pc, self.op, self.state);

    let rom = Some(rom);
    let ppu = &mut Some(ppu);

    match self.state {
      CpuState::Reset => {
        self.sp = 0xFD;
        self.p.b = true;
        self.p.i = true;
        let addr_l = self.bus.read(rom, ppu, 0xFFFC);
        let addr_h = self.bus.read(rom, ppu, 0xFFFD);
        self.pc = get_addr(addr_h, addr_l);
        self.state = CpuState::ReadOpcode;
        self.step = 0;
        return;
      }
      CpuState::ReadOpcode => {
        self.is_immediate = false;
        self.op = self.bus.read(rom, ppu, self.pc);
        self.state = match INSTRUCTION_SET[self.op as usize].mode {
          AddressingMode::Implied | AddressingMode::Accumulator | AddressingMode::Relative => {
            CpuState::ExecuteInstruction
          }
          AddressingMode::Immediate => {
            self.pc += 1;
            self.immediate_operand = self.bus.read(rom, ppu, self.pc);
            self.is_immediate = true;
            CpuState::ExecuteInstruction
          }

          _ => CpuState::ReadOperand,
        };
        self.pc += 1;
        self.step = 0;
        return;
      }
      CpuState::ReadOperand => {
        match INSTRUCTION_SET[self.op as usize].mode {
          AddressingMode::Absolute => match self.step {
            0 => {
              self.addr_l = self.bus.read(rom, ppu, self.pc);
              self.pc += 1;
            }
            1 => {
              self.addr_h = self.bus.read(rom, ppu, self.pc);
              self.state = CpuState::ExecuteInstruction;
              self.step = 0;
              self.pc += 1;
              return;
            }
            _ => panic!(),
          },
          AddressingMode::AbsoluteX | AddressingMode::AbsoluteY => match self.step {
            0 => {
              let (result, overflow) =
                self
                  .bus
                  .read(rom, ppu, self.pc)
                  .overflowing_add(match INSTRUCTION_SET[self.op as usize].mode {
                    AddressingMode::AbsoluteX => self.x,
                    AddressingMode::AbsoluteY => self.y,
                    _ => panic!(),
                  });
              self.addr_l = result;
              self.p.c = overflow;
              self.pc += 1;
            }
            1 => {
              self.addr_h = self.bus.read(rom, ppu, self.pc);
              if self.p.c == false {
                self.state = CpuState::ExecuteInstruction;
                self.step = 0;
                self.pc += 1;
                return;
              }
            }
            2 => {
              self.addr_h = self.addr_h.wrapping_add(1);
              self.state = CpuState::ExecuteInstruction;
              self.step = 0;
              self.pc += 1;
              return;
            }
            _ => panic!(),
          },
          AddressingMode::ZeroPage => {
            self.addr_h = 0x00;
            self.addr_l = self.bus.read(rom, ppu, self.pc);
            self.state = CpuState::ExecuteInstruction;
            self.step = 0;
            self.pc += 1;
            return;
          }
          AddressingMode::ZeroPageX | AddressingMode::ZeroPageY => match self.step {
            0 => {
              self.addr_h = 0x00;
              self.addr_l =
                self
                  .bus
                  .read(rom, ppu, self.pc)
                  .wrapping_add(match INSTRUCTION_SET[self.op as usize].mode {
                    AddressingMode::ZeroPageX => self.x,
                    AddressingMode::ZeroPageY => self.y,
                    _ => panic!(),
                  });
            }
            1 => {
              self.state = CpuState::ExecuteInstruction;
              self.step = 0;
              self.pc += 1;
              return;
            }
            _ => panic!(),
          },
          AddressingMode::Indirect => match self.step {
            0 => {
              self.addr_l = self.bus.read(rom, ppu, self.pc);
              self.pc += 1;
            }
            1 => {
              self.addr_h = self.bus.read(rom, ppu, self.pc);
            }
            2 => {
              self.state = CpuState::ExecuteInstruction;
              self.step = 0;
              self.pc += 1;
              return;
            }
            _ => panic!(),
          },
          AddressingMode::IndirectX => match self.step {
            0 => {
              self.addr_h = 0x00;
            }
            1 => {
              self.addr_l = self.bus.read(rom, ppu, self.pc).wrapping_add(self.x);
            }
            2 => {
              let addr = get_addr(self.addr_h, self.addr_l);
              self.addr_l = self.bus.read(rom, ppu, addr);
              self.addr_h = self.bus.read(rom, ppu, addr + 1); //3サイクル目でやるのが正解っぽいけどレジスタが足りん
            }
            3 => {
              self.state = CpuState::ExecuteInstruction;
              self.step = 0;
              self.pc += 1;
              return;
            }
            _ => panic!(),
          },
          AddressingMode::IndirectY => match self.step {
            0 => {
              self.addr_l = self.bus.read(rom, ppu, self.pc);
            }
            1 => {
              let addr = get_addr(0x00, self.addr_l);
              self.addr_h = self.bus.read(rom, ppu, addr);
            }
            2 => {
              let addr = get_addr(0x00, self.addr_l.wrapping_add(1));
              self.addr_l = self.bus.read(rom, ppu, addr);
              let (result, overflow) = self.addr_l.overflowing_add(self.y);
              self.addr_l = result;
              if overflow == false {
                self.state = CpuState::ExecuteInstruction;
                self.step = 0;
                self.pc += 1;
                return;
              }
            }
            3 => {
              self.addr_h += 1;
              self.state = CpuState::ExecuteInstruction;
              self.step = 0;
              self.pc += 1;
              return;
            }
            _ => panic!(),
          },
          _ => panic!(),
        }
      }
      CpuState::ExecuteInstruction => {
        match INSTRUCTION_SET[self.op as usize].instruction {
          //演算
          Instruction::Adc => {
            let operand = if self.is_immediate {
              self.immediate_operand
            } else {
              let addr = get_addr(self.addr_h, self.addr_l);
              self.bus.read(rom, ppu, addr)
            };

            let (result, overflow) = self.a.overflowing_add(operand);
            let (result2, overflow2) = result.overflowing_add(if self.p.c { 1 } else { 0 });

            self.p.n = (result2 & 0x80) == 0x80;
            self.p.v = ((self.a ^ result2) & (operand ^ result2) & 0x80) == 0x80;
            self.p.z = result2 == 0;
            self.p.c = overflow || overflow2;

            self.a = result2;

            self.pc += 1;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Sbc => {
            let operand = if self.is_immediate {
              self.immediate_operand
            } else {
              let addr = get_addr(self.addr_h, self.addr_l);
              self.bus.read(rom, ppu, addr)
            };

            let (result, overflow) = self.a.overflowing_sub(operand);
            let (result2, overflow2) = result.overflowing_sub(if self.p.c { 0 } else { 1 });

            self.p.n = (result2 & 0x80) == 0x80;
            self.p.v = ((self.a ^ result2) & (operand ^ result2) & 0x80) == 0x80;
            self.p.z = result2 == 0;
            self.p.c = overflow || overflow2;

            self.a = result2;

            self.pc += 1;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          //論理演算
          Instruction::And => {
            let operand = if self.is_immediate {
              self.immediate_operand
            } else {
              let addr = get_addr(self.addr_h, self.addr_l);
              self.bus.read(rom, ppu, addr)
            };

            self.a &= operand;

            self.p.n = (self.a & 0x80) == 0x80;
            self.p.z = self.a == 0;

            self.pc += 1;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Ora => {
            let operand = if self.is_immediate {
              self.immediate_operand
            } else {
              let addr = get_addr(self.addr_h, self.addr_l);
              self.bus.read(rom, ppu, addr)
            };

            self.a |= operand;

            self.p.n = (self.a & 0x80) == 0x80;
            self.p.z = self.a == 0;

            self.pc += 1;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Eor => {
            let operand = if self.is_immediate {
              self.immediate_operand
            } else {
              let addr = get_addr(self.addr_h, self.addr_l);
              self.bus.read(rom, ppu, addr)
            };

            self.a ^= operand;

            self.p.n = (self.a & 0x80) == 0x80;
            self.p.z = self.a == 0;

            self.pc += 1;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          //シフト、ローテーション
          Instruction::Asl => {
            self.p.c = (self.a & 0x80) == 0x80;
            self.a &= 0x7F;
            self.a <<= 1;

            self.p.n = (self.a & 0x80) == 0x80;
            self.p.z = self.a == 0;

            self.pc += 1;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Lsr => {
            self.p.c = (self.a & 0x01) == 0x01;
            self.a >>= 1;

            self.p.n = (self.a & 0x80) == 0x80;
            self.p.z = self.a == 0;

            self.pc += 1;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Rol => {
            let carry = self.p.c;
            self.p.c = (self.a & 0x80) == 0x80;
            self.a &= 0x7F;
            self.a <<= 1;
            self.a += if carry { 1 } else { 0 };

            self.p.n = (self.a & 0x80) == 0x80;
            self.p.z = self.a == 0;

            self.pc += 1;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Ror => {
            let carry = self.p.c;
            self.p.c = (self.a & 0x01) == 0x01;
            self.a >>= 1;
            self.a += if carry { 0x80 } else { 0 };

            self.p.n = (self.a & 0x80) == 0x80;
            self.p.z = self.a == 0;

            self.pc += 1;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          //条件分岐
          Instruction::Bcc
          | Instruction::Bcs
          | Instruction::Beq
          | Instruction::Bne
          | Instruction::Bvc
          | Instruction::Bvs
          | Instruction::Bpl
          | Instruction::Bmi => {
            let jump = self.step != 0
              || match INSTRUCTION_SET[self.op as usize].instruction {
                Instruction::Bcc => !self.p.c,
                Instruction::Bcs => self.p.c,
                Instruction::Beq => self.p.z,
                Instruction::Bne => !self.p.z,
                Instruction::Bvc => !self.p.v,
                Instruction::Bvs => self.p.v,
                Instruction::Bpl => !self.p.n,
                Instruction::Bmi => self.p.n,
                _ => panic!(),
              };
            if jump {
              match self.step {
                0 => {
                  let offset = self.bus.read(rom, ppu, self.pc) as i8 as i16;
                  let last_pc = self.pc;
                  self.pc += 1;
                  let page_cross = (last_pc & 0xFF00) != (self.pc & 0xFF00);
                  let last_pc2 = self.pc;
                  self.pc = ((self.pc as i16) + offset) as u16;
                  let page_cross2 = (last_pc2 & 0xFF00) != (self.pc & 0xFF00);
                  if !page_cross && !page_cross2 {
                    self.state = CpuState::ReadOpcode;
                    self.step = 0;
                    return;
                  }
                }
                1 => {
                  self.state = CpuState::ReadOpcode;
                  self.step = 0;
                  return;
                }
                _ => panic!(),
              }
            } else {
              self.pc += 1;
              self.state = CpuState::ReadOpcode;
              self.step = 0;
              return;
            }
          }
          //ビット検査
          Instruction::Bit => {
            let operand = if self.is_immediate {
              self.immediate_operand
            } else {
              let addr = get_addr(self.addr_h, self.addr_l);
              self.bus.read(rom, ppu, addr)
            };

            let result = self.a & operand;

            self.p.n = (operand & 0x80) == 0x80;
            self.p.v = (operand & 0x40) == 0x40;
            self.p.z = result == 0;

            self.pc += 1;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          //ジャンプ
          Instruction::Jmp => match self.step {
            0 => {
              self.pc &= 0xFF00;
              self.pc |= self.addr_l as u16;
            }
            1 => {
              self.pc &= 0x00FF;
              self.pc |= (self.addr_h as u16) << 8;

              self.state = CpuState::ReadOpcode;
              self.step = 0;
              return;
            }
            _ => panic!(),
          },
          Instruction::Jsr => match self.step {
            0 => self.push((self.pc >> 8) as u8),
            1 => self.push(self.pc as u8),
            2 => {
              self.pc = get_addr(self.addr_h, self.addr_l);
              self.state = CpuState::ReadOpcode;
              self.step = 0;
              return;
            }
            _ => panic!(),
          },
          Instruction::Rts => match self.step {
            0 => {}
            1 => {}
            2 => {
              self.pc &= 0xFF00;
              self.pc |= self.pop() as u16;
            }
            3 => {
              self.pc &= 0x00FF;
              self.pc |= (self.pop() as u16) << 8;
            }
            4 => {
              self.pc += 1;
              self.state = CpuState::ReadOpcode;
              self.step = 0;
              return;
            }
            _ => panic!(),
          },
          //割り込み
          Instruction::Brk => match self.step {
            0 => {
              if self.p.i {
                self.state = CpuState::ReadOpcode;
                self.step = 0;
                return;
              }
              self.p.b = true;
              self.pc += 1;
            }
            1 => self.push((self.pc >> 8) as u8),
            2 => self.push(self.pc as u8),
            3 => self.push(self.p.read()),
            4 => {
              self.p.i = true;
              self.pc &= 0x00FF;
              self.pc |= (self.bus.read(rom, ppu, 0xFFFE) as u16) << 8;
            }
            5 => {
              self.pc &= 0xFF00;
              self.pc |= self.bus.read(rom, ppu, 0xFFFF) as u16;

              self.state = CpuState::ReadOpcode;
              self.step = 0;
              return;
            }
            _ => panic!(),
          },
          Instruction::Rti => match self.step {
            0 => {}
            1 => {}
            2 => {
              let result = self.pop();
              self.p.write(result);
            }
            3 => {
              self.pc &= 0xFF00;
              self.pc |= self.pop() as u16;
            }
            4 => {
              self.pc &= 0x00FF;
              self.pc |= (self.pop() as u16) << 8;

              self.state = CpuState::ReadOpcode;
              self.step = 0;
              return;
            }
            _ => panic!(),
          },
          //比較
          Instruction::Cmp | Instruction::Cpx | Instruction::Cpy => {
            let operand = if self.is_immediate {
              self.immediate_operand
            } else {
              let addr = get_addr(self.addr_h, self.addr_l);
              self.bus.read(rom, ppu, addr)
            };

            let (result, overflow) = match INSTRUCTION_SET[self.op as usize].instruction {
              Instruction::Cmp => self.a,
              Instruction::Cpx => self.x,
              Instruction::Cpy => self.y,
              _ => panic!(),
            }
            .overflowing_sub(operand);

            self.p.n = (result & 0x80) == 0x80;
            self.p.z = result == 0;
            self.p.c = overflow;
          }
          //インクリメント、デクリメント
          Instruction::Inc | Instruction::Dec => match self.step {
            0 => {}
            1 => {
              let operand = if self.is_immediate {
                self.immediate_operand
              } else {
                let addr = get_addr(self.addr_h, self.addr_l);
                self.bus.read(rom, ppu, addr)
              };

              let operand = match INSTRUCTION_SET[self.op as usize].instruction {
                Instruction::Inc => operand.wrapping_add(1),
                Instruction::Dec => operand.wrapping_sub(1),
                _ => panic!(),
              };
              self.p.n = (operand & 0x80) == 0x80;
              self.p.z = operand == 0;

              let addr = get_addr(self.addr_h, self.addr_l);
              self.bus.write(ppu, addr, operand);
            }
            2 => {
              self.state = CpuState::ReadOpcode;
              self.step = 0;
              return;
            }
            _ => panic!(),
          },
          Instruction::Inx | Instruction::Dex => {
            self.x = match INSTRUCTION_SET[self.op as usize].instruction {
              Instruction::Inx => self.x.wrapping_add(1),
              Instruction::Dex => self.x.wrapping_sub(1),
              _ => panic!(),
            };

            self.p.n = (self.x & 0x80) == 0x80;
            self.p.z = self.x == 0;

            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Iny | Instruction::Dey => {
            self.y = match INSTRUCTION_SET[self.op as usize].instruction {
              Instruction::Iny => self.y.wrapping_add(1),
              Instruction::Dey => self.y.wrapping_sub(1),
              _ => panic!(),
            };

            self.p.n = (self.y & 0x80) == 0x80;
            self.p.z = self.y == 0;

            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          //フラグ操作
          Instruction::Clc => {
            self.p.c = false;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Sec => {
            self.p.c = true;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Cli => {
            self.p.i = false;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Sei => {
            self.p.i = true;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Cld => {
            self.p.d = false;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Sed => {
            self.p.d = true;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Clv => {
            self.p.v = false;
            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          //ロード
          Instruction::Lda => {
            let operand = if self.is_immediate {
              self.immediate_operand
            } else {
              let addr = get_addr(self.addr_h, self.addr_l);
              self.bus.read(rom, ppu, addr)
            };

            self.a = operand;
            // println!(
            //   "LDA addr: {:x}, operand: {:x}",
            //   get_addr(self.addr_h, self.addr_l),
            //   operand
            // );

            self.p.n = (self.a & 0x80) == 0x80;
            self.p.z = self.a == 0;

            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Ldx => {
            let operand = if self.is_immediate {
              self.immediate_operand
            } else {
              let addr = get_addr(self.addr_h, self.addr_l);
              self.bus.read(rom, ppu, addr)
            };

            self.x = operand;

            self.p.n = (self.x & 0x80) == 0x80;
            self.p.z = self.x == 0;

            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Ldy => {
            let operand = if self.is_immediate {
              self.immediate_operand
            } else {
              let addr = get_addr(self.addr_h, self.addr_l);
              self.bus.read(rom, ppu, addr)
            };

            self.y = operand;

            self.p.n = (self.y & 0x80) == 0x80;
            self.p.z = self.y == 0;

            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          //ストア
          Instruction::Sta | Instruction::Stx | Instruction::Sty => {
            let addr = get_addr(self.addr_h, self.addr_l);
            self.bus.write(
              ppu,
              addr,
              match INSTRUCTION_SET[self.op as usize].instruction {
                Instruction::Sta => self.a,
                Instruction::Stx => self.x,
                Instruction::Sty => self.y,
                _ => panic!(),
              },
            );

            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          //レジスタ間転送
          Instruction::Tax | Instruction::Tsx => {
            self.x = match INSTRUCTION_SET[self.op as usize].instruction {
              Instruction::Tax => self.a,
              Instruction::Tsx => self.sp,
              _ => panic!(),
            };
            self.p.n = (self.x & 0x80) == 0x80;
            self.p.z = self.x == 0;

            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Txa | Instruction::Tya => {
            self.a = match INSTRUCTION_SET[self.op as usize].instruction {
              Instruction::Txa => self.x,
              Instruction::Tya => self.y,
              _ => panic!(),
            };
            self.p.n = (self.a & 0x80) == 0x80;
            self.p.z = self.a == 0;

            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Tay => {
            self.y = self.a;
            self.p.n = (self.y & 0x80) == 0x80;
            self.p.z = self.y == 0;

            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          Instruction::Txs => {
            self.sp = self.x;

            self.state = CpuState::ReadOpcode;
            self.step = 0;
            return;
          }
          //スタック
          Instruction::Pha | Instruction::Php => match self.step {
            0 => {}
            1 => {
              self.push(match INSTRUCTION_SET[self.op as usize].instruction {
                Instruction::Pha => self.a,
                Instruction::Php => self.p.read(),
                _ => panic!(),
              });

              self.state = CpuState::ReadOpcode;
              self.step = 0;
            }
            _ => panic!(),
          },
          Instruction::Pla => match self.step {
            0 => {}
            1 => {}
            2 => {
              self.a = self.pop();

              self.p.n = (self.a & 0x80) == 0x80;
              self.p.z = self.a == 0;
              self.state = CpuState::ReadOpcode;
              self.step = 0;
            }
            _ => panic!(),
          },
          Instruction::Plp => match self.step {
            0 => {}
            1 => {}
            2 => {
              let result = self.pop();
              self.p.write(result);

              self.state = CpuState::ReadOpcode;
              self.step = 0;
            }
            _ => panic!(),
          },
          Instruction::Nop => {
            self.state = CpuState::ReadOpcode;
            self.step = 0;
          }
          Instruction::Undefined => panic!(),
        }
      }
    }
    self.step += 1;
  }
  fn push(&mut self, value: u8) {
    self.bus.write(&mut None, 0x0100 | self.sp as u16, value);
    self.sp -= 1;
  }
  fn pop(&mut self) -> u8 {
    self.sp += 1;
    self.bus.read(None, &mut None, 0x0100 | self.sp as u16)
  }
}

enum AddressingMode {
  Implied,
  Accumulator,
  Immediate,
  ZeroPage,
  ZeroPageX,
  ZeroPageY,
  Relative,
  Absolute,
  AbsoluteX,
  AbsoluteY,
  Indirect,
  IndirectX,
  IndirectY,
  AddressingModeLength,
}
enum Instruction {
  Adc,
  Sbc,
  And,
  Ora,
  Eor,
  Asl,
  Lsr,
  Rol,
  Ror,
  Bcc,
  Bcs,
  Beq,
  Bne,
  Bvc,
  Bvs,
  Bpl,
  Bmi,
  Bit,
  Jmp,
  Jsr,
  Rts,
  Brk,
  Rti,
  Cmp,
  Cpx,
  Cpy,
  Inc,
  Dec,
  Inx,
  Dex,
  Iny,
  Dey,
  Clc,
  Sec,
  Cli,
  Sei,
  Cld,
  Sed,
  Clv,
  Lda,
  Ldx,
  Ldy,
  Sta,
  Stx,
  Sty,
  Tax,
  Txa,
  Tay,
  Tya,
  Tsx,
  Txs,
  Pha,
  Pla,
  Php,
  Plp,
  Nop,
  Undefined,
}
struct InstructionDefinition {
  mode: AddressingMode,
  instruction: Instruction,
  clock: u8,
}

const UNDEFINED_INSTRUCTION: InstructionDefinition =
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Undefined, clock: 1 };

const INSTRUCTION_SET: [InstructionDefinition; 0x100] = [
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Brk, clock: 7 },
  InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::Ora, clock: 6 },
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Ora, clock: 3 },
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Asl, clock: 5 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Php, clock: 3 },
  InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::Ora, clock: 2 },
  InstructionDefinition { mode: AddressingMode::Accumulator, instruction: Instruction::Asl, clock: 2 },
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Ora, clock: 4 },
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Asl, clock: 6 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Relative, instruction: Instruction::Bpl, clock: 2 }, //+1or2
  InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::Ora, clock: 5 }, //+1
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::Ora, clock: 4 },
  InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::Asl, clock: 6 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Clc, clock: 2 },
  InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::Ora, clock: 4 }, //+1
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::Ora, clock: 4 }, //+1
  InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::Asl, clock: 6 }, //+1
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Jsr, clock: 6 },
  InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::And, clock: 6 },
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Bit, clock: 3 },
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::And, clock: 3 },
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Rol, clock: 5 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Plp, clock: 4 },
  InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::And, clock: 2 },
  InstructionDefinition { mode: AddressingMode::Accumulator, instruction: Instruction::Rol, clock: 2 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Bit, clock: 4 },
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::And, clock: 4 },
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Rol, clock: 6 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Relative, instruction: Instruction::Bmi, clock: 2 }, //+1or2
  InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::And, clock: 5 }, //+1
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::And, clock: 4 },
  InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::Rol, clock: 6 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Sec, clock: 2 },
  InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::And, clock: 4 }, //+1
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::And, clock: 4 }, //+1
  InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::Rol, clock: 6 }, //+1
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Rti, clock: 6 },
  InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::Eor, clock: 6 },
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Eor, clock: 3 },
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Lsr, clock: 5 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Pha, clock: 3 },
  InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::Eor, clock: 2 },
  InstructionDefinition { mode: AddressingMode::Accumulator, instruction: Instruction::Lsr, clock: 2 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Jmp, clock: 3 },
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Eor, clock: 4 },
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Lsr, clock: 6 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Relative, instruction: Instruction::Bvc, clock: 2 }, //+1or2
  InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::Eor, clock: 5 }, //+1
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::Eor, clock: 4 },
  InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::Lsr, clock: 6 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Cli, clock: 2 },
  InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::Eor, clock: 4 }, //+1
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::Eor, clock: 4 }, //+1
  InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::Lsr, clock: 6 }, //+1
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Rts, clock: 6 },
  InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::Adc, clock: 6 },
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Adc, clock: 3 },
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Ror, clock: 5 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Pla, clock: 4 },
  InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::Adc, clock: 2 },
  InstructionDefinition { mode: AddressingMode::Accumulator, instruction: Instruction::Ror, clock: 2 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Indirect, instruction: Instruction::Jmp, clock: 5 },
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Adc, clock: 4 },
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Ror, clock: 6 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Relative, instruction: Instruction::Bvs, clock: 2 }, //+1or2
  InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::Adc, clock: 5 }, //+1
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::Adc, clock: 4 },
  InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::Ror, clock: 6 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Sei, clock: 2 },
  InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::Adc, clock: 4 }, //+1
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::Adc, clock: 4 }, //+1
  InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::Ror, clock: 6 }, //+1
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::Sta, clock: 6 },
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Sty, clock: 3 },
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Sta, clock: 3 },
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Stx, clock: 3 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Dey, clock: 2 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Txa, clock: 2 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Sty, clock: 4 },
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Sta, clock: 4 },
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Stx, clock: 4 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Relative, instruction: Instruction::Bcc, clock: 2 }, //+1or2
  InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::Sta, clock: 5 }, //+1
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::Sty, clock: 4 },
  InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::Sta, clock: 4 },
  InstructionDefinition { mode: AddressingMode::ZeroPageY, instruction: Instruction::Stx, clock: 4 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Tya, clock: 2 },
  InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::Sta, clock: 4 }, //+1
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Txs, clock: 2 },
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::Sta, clock: 4 }, //+1
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::Ldy, clock: 2 },
  InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::Lda, clock: 6 },
  InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::Ldx, clock: 2 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Ldy, clock: 3 },
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Lda, clock: 3 },
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Ldx, clock: 3 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Tay, clock: 2 },
  InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::Lda, clock: 2 },
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Tax, clock: 2 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Ldy, clock: 4 },
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Lda, clock: 4 },
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Ldx, clock: 4 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Relative, instruction: Instruction::Bcs, clock: 2 }, //+1or2
  InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::Lda, clock: 5 }, //+1
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::Ldy, clock: 4 },
  InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::Lda, clock: 4 },
  InstructionDefinition { mode: AddressingMode::ZeroPageY, instruction: Instruction::Ldx, clock: 4 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Clv, clock: 2 },
  InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::Lda, clock: 4 }, //+1
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Tsx, clock: 2 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::Ldy, clock: 4 }, //+1
  InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::Lda, clock: 4 }, //+1
  InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::Ldx, clock: 4 }, //+1
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::Cpy, clock: 2 },
  InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::Cmp, clock: 6 },
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Cpy, clock: 3 },
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Cmp, clock: 3 },
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Dec, clock: 5 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Iny, clock: 2 },
  InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::Cmp, clock: 2 },
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Dex, clock: 2 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Cpy, clock: 4 },
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Cmp, clock: 4 },
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Dec, clock: 6 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Relative, instruction: Instruction::Bne, clock: 2 }, //+1or2
  InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::Cmp, clock: 5 }, //+1
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::Cmp, clock: 4 },
  InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::Dec, clock: 6 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Cld, clock: 2 },
  InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::Cmp, clock: 4 }, //+1
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::Cmp, clock: 4 }, //+1
  InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::Dec, clock: 6 }, //+1
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::Cpx, clock: 2 },
  InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::Sbc, clock: 6 },
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Cpx, clock: 3 },
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Sbc, clock: 3 },
  InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::Inc, clock: 5 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Inx, clock: 2 },
  InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::Sbc, clock: 2 },
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Nop, clock: 2 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Cpx, clock: 4 },
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Sbc, clock: 4 },
  InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::Inc, clock: 6 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Relative, instruction: Instruction::Beq, clock: 2 }, //+1or2
  InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::Sbc, clock: 5 }, //+1
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::Sbc, clock: 4 },
  InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::Inc, clock: 6 },
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Sed, clock: 2 },
  InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::Sbc, clock: 4 }, //+1
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  UNDEFINED_INSTRUCTION,
  InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::Sbc, clock: 4 }, //+1
  InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::Inc, clock: 6 }, //+1
  UNDEFINED_INSTRUCTION,
];

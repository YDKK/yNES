use super::apu::*;
use super::nes::PadInputs;
use super::ppu::*;
use super::rom::*;
use super::util::*;
mod bus;

struct ProcessorStatusRegister {
    n: bool,
    v: bool,
    b: bool,
    d: bool,
    i: bool,
    z: bool,
    c: bool,
}
impl ProcessorStatusRegister {
    #[inline(always)]
    fn read(&self) -> u8 {
        let mut value: u8 = 0b0010_0000; // bit 5 always set
        if self.n {
            value |= 0b1000_0000;
        }
        if self.v {
            value |= 0b0100_0000;
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
    #[inline(always)]
    fn write(&mut self, value: u8) {
        self.n = (value & 0b1000_0000) != 0;
        self.v = (value & 0b0100_0000) != 0;
        self.b = (value & 0b0001_0000) != 0;
        self.d = (value & 0b0000_1000) != 0;
        self.i = (value & 0b0000_0100) != 0;
        self.z = (value & 0b0000_0010) != 0;
        self.c = (value & 0b0000_0001) != 0;
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
    is_accumulator: bool,
    reset: bool,
    nmi: bool,
    irq: bool,
    addressing_overflow: bool,
    suspend_cycle: u16,
}

#[derive(Debug)]
enum CpuState {
    Reset,
    Nmi,
    Irq,
    ReadOpcode,
    ReadOperand,
    ExecuteInstruction,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            sp: 0xFD,
            p: ProcessorStatusRegister { n: false, v: false, b: true, d: false, i: true, z: false, c: false },
            bus: bus::Bus::new(),
            op: 0,
            state: CpuState::Reset,
            step: 0,
            addr_l: 0,
            addr_h: 0,
            immediate_operand: 0,
            is_immediate: false,
            is_accumulator: false,
            reset: false,
            nmi: false,
            irq: false,
            addressing_overflow: false,
            suspend_cycle: 0,
        }
    }
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.reset = true;
    }
    pub fn nmi(&mut self) {
        self.nmi = true;
    }
    #[allow(dead_code)]
    pub fn irq(&mut self) {
        self.irq = true;
    }

    #[inline(always)]
    fn set_nz(&mut self, value: u8) {
        self.p.n = (value & 0x80) != 0;
        self.p.z = value == 0;
    }

    pub fn clock(&mut self, rom: &Rom, apu: &mut Apu, ppu: &mut Ppu, pad: &PadInputs) {
        let rom = Some(rom);
        let apu = &mut Some(apu);
        let ppu = &mut Some(ppu);
        let pad = Some(pad);

        if self.suspend_cycle > 0 {
            self.suspend_cycle -= 1;
            return;
        }

        match self.state {
            CpuState::Reset => {
                self.p.b = false;
                self.p.i = true;
                let addr_l = self.bus.read(rom, apu, ppu, pad, 0xFFFC);
                let addr_h = self.bus.read(rom, apu, ppu, pad, 0xFFFD);
                self.pc = get_addr(addr_h, addr_l);
                self.state = CpuState::ReadOpcode;
                self.step = 0;
                return;
            }
            CpuState::Nmi => {
                self.p.b = false;
                self.push((self.pc >> 8) as u8);
                self.push(self.pc as u8);
                self.push(self.p.read());
                self.p.i = true;
                let addr_l = self.bus.read(rom, apu, ppu, pad, 0xFFFA);
                let addr_h = self.bus.read(rom, apu, ppu, pad, 0xFFFB);
                self.pc = get_addr(addr_h, addr_l);
                self.state = CpuState::ReadOpcode;
                self.step = 0;
                return;
            }
            CpuState::Irq => {
                //Iフラグはチェック済み
                self.p.b = false;
                self.push((self.pc >> 8) as u8);
                self.push(self.pc as u8);
                self.push(self.p.read());
                self.p.i = true;
                let addr_l = self.bus.read(rom, apu, ppu, pad, 0xFFFE);
                let addr_h = self.bus.read(rom, apu, ppu, pad, 0xFFFF);
                self.pc = get_addr(addr_h, addr_l);
                self.state = CpuState::ReadOpcode;
                self.step = 0;
                return;
            }
            CpuState::ReadOpcode => {
                //割り込みチェック
                if self.reset {
                    self.reset = false;
                    self.state = CpuState::Reset;
                    self.step = 0;
                    return;
                }
                if self.nmi {
                    self.nmi = false;
                    self.state = CpuState::Nmi;
                    self.step = 0;
                    return;
                }
                self.irq |= apu.as_ref().unwrap().check_irq();
                if self.irq {
                    self.irq = false;
                    if !self.p.i {
                        self.state = CpuState::Irq;
                        self.step = 0;
                        return;
                    }
                }
                self.is_immediate = false;
                self.is_accumulator = false;
                self.addressing_overflow = false;
                self.op = self.bus.read(rom, apu, ppu, pad, self.pc);
                let addressing_mode = &INSTRUCTION_SET[self.op as usize].mode;

                //ブレークポイント
                let break_point: Option<u16> = None; //Some(0x8155);
                if break_point.is_some() && break_point.unwrap() == self.pc {
                    println!("Hit break point"); //break here
                }

                //ログ出力
                let output_log = false;
                if output_log {
                    let instruction = &INSTRUCTION_SET[self.op as usize].instruction;
                    let operand_1 = self.bus.read(rom, apu, ppu, pad, self.pc + 1);
                    let operand_2 = self.bus.read(rom, apu, ppu, pad, self.pc + 2);
                    match addressing_mode {
                        AddressingMode::Accumulator | AddressingMode::Implied => {
                            println!(
                                "{:04X}  {:02X}        {:?} {:<27} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}",
                                self.pc,
                                self.op,
                                instruction,
                                "",
                                self.a,
                                self.x,
                                self.y,
                                self.p.read(),
                                self.sp
                            );
                        }
                        AddressingMode::Immediate
                        | AddressingMode::ZeroPage
                        | AddressingMode::ZeroPageX
                        | AddressingMode::ZeroPageY
                        | AddressingMode::Relative
                        | AddressingMode::IndirectX
                        | AddressingMode::IndirectY => {
                            let operand = match addressing_mode {
                                AddressingMode::Immediate => format!("#${:02X}", operand_1),
                                AddressingMode::ZeroPage | AddressingMode::Relative => format!("${:02X}", operand_1),
                                AddressingMode::ZeroPageX => format!("${:02X},X", operand_1),
                                AddressingMode::ZeroPageY => format!("${:02X},Y", operand_1),
                                AddressingMode::IndirectX => format!("(${:02X},X)", operand_1),
                                AddressingMode::IndirectY => format!("(${:02X}),Y", operand_1),
                                _ => panic!(),
                            };
                            println!(
                                "{:04X}  {:02X} {:02X}     {:?} {:<27} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}",
                                self.pc,
                                self.op,
                                operand_1,
                                instruction,
                                operand,
                                self.a,
                                self.x,
                                self.y,
                                self.p.read(),
                                self.sp
                            );
                        }
                        AddressingMode::Absolute
                        | AddressingMode::AbsoluteX
                        | AddressingMode::AbsoluteY
                        | AddressingMode::Indirect => {
                            let operand = match addressing_mode {
                                AddressingMode::Absolute => format!("${:02X}{:02X}", operand_2, operand_1),
                                AddressingMode::AbsoluteX => format!("${:02X}{:02X},X", operand_2, operand_1),
                                AddressingMode::AbsoluteY => format!("${:02X}{:02X},Y", operand_2, operand_1),
                                AddressingMode::Indirect => format!("(${:02X}{:02X})", operand_2, operand_1),
                                _ => panic!(),
                            };
                            println!(
                "{:04X}  {:02X} {:02X} {:02X}  {:?} {:<27} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}",
                self.pc,
                self.op,
                operand_1,
                operand_2,
                instruction,
                operand,
                self.a,
                self.x,
                self.y,
                self.p.read(),
                self.sp
              );
                        }
                    }
                }

                self.state = match addressing_mode {
                    AddressingMode::Accumulator => {
                        self.is_accumulator = true;
                        CpuState::ExecuteInstruction
                    }
                    AddressingMode::Implied | AddressingMode::Relative => CpuState::ExecuteInstruction,
                    AddressingMode::Immediate => {
                        self.pc += 1;
                        self.immediate_operand = self.bus.read(rom, apu, ppu, pad, self.pc);
                        self.is_immediate = true;
                        CpuState::ExecuteInstruction
                    }
                    _ => CpuState::ReadOperand,
                };

                self.pc += 1;
                self.step = 0;
                return;
            }
            CpuState::ReadOperand => match INSTRUCTION_SET[self.op as usize].mode {
                AddressingMode::Absolute => match self.step {
                    0 => {
                        self.addr_l = self.bus.read(rom, apu, ppu, pad, self.pc);
                        self.pc += 1;
                    }
                    1 => {
                        self.addr_h = self.bus.read(rom, apu, ppu, pad, self.pc);
                        self.state = CpuState::ExecuteInstruction;
                        self.step = 0;
                        self.pc += 1;
                        return;
                    }
                    _ => panic!(),
                },
                AddressingMode::AbsoluteX | AddressingMode::AbsoluteY => match self.step {
                    0 => {
                        let (result, overflow) = self.bus.read(rom, apu, ppu, pad, self.pc).overflowing_add(
                            match INSTRUCTION_SET[self.op as usize].mode {
                                AddressingMode::AbsoluteX => self.x,
                                AddressingMode::AbsoluteY => self.y,
                                _ => panic!(),
                            },
                        );
                        self.addr_l = result;
                        self.addressing_overflow = overflow;
                        self.pc += 1;
                    }
                    1 => {
                        self.addr_h = self.bus.read(rom, apu, ppu, pad, self.pc);
                        if !self.addressing_overflow {
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
                    self.addr_l = self.bus.read(rom, apu, ppu, pad, self.pc);
                    self.state = CpuState::ExecuteInstruction;
                    self.step = 0;
                    self.pc += 1;
                    return;
                }
                AddressingMode::ZeroPageX | AddressingMode::ZeroPageY => match self.step {
                    0 => {
                        self.addr_h = 0x00;
                        self.addr_l = self.bus.read(rom, apu, ppu, pad, self.pc).wrapping_add(
                            match INSTRUCTION_SET[self.op as usize].mode {
                                AddressingMode::ZeroPageX => self.x,
                                AddressingMode::ZeroPageY => self.y,
                                _ => panic!(),
                            },
                        );
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
                        self.addr_l = self.bus.read(rom, apu, ppu, pad, self.pc);
                        self.pc += 1;
                    }
                    1 => {
                        self.addr_h = self.bus.read(rom, apu, ppu, pad, self.pc);
                    }
                    2 => {
                        let addr = get_addr(self.addr_h, self.addr_l);
                        let addr2 = get_addr(self.addr_h, self.addr_l.wrapping_add(1));
                        self.addr_l = self.bus.read(rom, apu, ppu, pad, addr);
                        self.addr_h = self.bus.read(rom, apu, ppu, pad, addr2);
                        self.state = CpuState::ExecuteInstruction;
                        self.step = 0;
                        self.pc += 1;
                        return;
                    }
                    _ => panic!(),
                },
                AddressingMode::IndirectX => match self.step {
                    0 => {
                        self.addr_l = self.bus.read(rom, apu, ppu, pad, self.pc).wrapping_add(self.x);
                    }
                    1 => {
                        self.addr_h = 0x00;
                    }
                    2 => {
                        let addr = get_addr(self.addr_h, self.addr_l);
                        let addr2 = get_addr(self.addr_h, self.addr_l.wrapping_add(1));
                        self.addr_l = self.bus.read(rom, apu, ppu, pad, addr);
                        self.addr_h = self.bus.read(rom, apu, ppu, pad, addr2); //3サイクル目でやるのが正解っぽい気がする
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
                        self.addr_l = self.bus.read(rom, apu, ppu, pad, self.pc);
                    }
                    1 => {
                        let addr = get_addr(0x00, self.addr_l);
                        let addr2 = get_addr(0x00, self.addr_l.wrapping_add(1));
                        self.addr_l = self.bus.read(rom, apu, ppu, pad, addr);
                        self.addr_h = self.bus.read(rom, apu, ppu, pad, addr2);
                    }
                    2 => {
                        let (result, overflow) = self.addr_l.overflowing_add(self.y);
                        self.addr_l = result;
                        if !overflow {
                            self.state = CpuState::ExecuteInstruction;
                            self.step = 0;
                            self.pc += 1;
                            return;
                        }
                    }
                    3 => {
                        self.addr_h = self.addr_h.wrapping_add(1);
                        self.state = CpuState::ExecuteInstruction;
                        self.step = 0;
                        self.pc += 1;
                        return;
                    }
                    _ => panic!(),
                },
                _ => panic!(),
            },
            CpuState::ExecuteInstruction => {
                match INSTRUCTION_SET[self.op as usize].instruction {
                    //演算
                    Instruction::ADC => {
                        let operand = if self.is_immediate {
                            self.immediate_operand
                        } else {
                            let addr = get_addr(self.addr_h, self.addr_l);
                            self.bus.read(rom, apu, ppu, pad, addr)
                        };
                        let (result, overflow) = self.a.overflowing_add(operand);
                        let (result2, overflow2) = result.overflowing_add(if self.p.c { 1 } else { 0 });
                        self.p.v = ((self.a ^ result2) & (operand ^ result2) & 0x80) != 0;
                        self.p.c = overflow || overflow2;
                        self.a = result2;
                        self.set_nz(self.a);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::SBC => {
                        let operand = if self.is_immediate {
                            self.immediate_operand
                        } else {
                            let addr = get_addr(self.addr_h, self.addr_l);
                            self.bus.read(rom, apu, ppu, pad, addr)
                        };
                        let (result, overflow) = self.a.overflowing_sub(operand);
                        let (result2, overflow2) = result.overflowing_sub(if self.p.c { 0 } else { 1 });
                        self.p.v = ((self.a ^ operand) & (self.a ^ result2) & 0x80) != 0;
                        self.p.c = !(overflow || overflow2);
                        self.a = result2;
                        self.set_nz(self.a);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    //論理演算
                    Instruction::AND => {
                        let operand = if self.is_immediate {
                            self.immediate_operand
                        } else {
                            self.bus.read(rom, apu, ppu, pad, get_addr(self.addr_h, self.addr_l))
                        };
                        self.a &= operand;
                        self.set_nz(self.a);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::ORA => {
                        let operand = if self.is_immediate {
                            self.immediate_operand
                        } else {
                            self.bus.read(rom, apu, ppu, pad, get_addr(self.addr_h, self.addr_l))
                        };
                        self.a |= operand;
                        self.set_nz(self.a);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::EOR => {
                        let operand = if self.is_immediate {
                            self.immediate_operand
                        } else {
                            self.bus.read(rom, apu, ppu, pad, get_addr(self.addr_h, self.addr_l))
                        };
                        self.a ^= operand;
                        self.set_nz(self.a);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    //シフト、ローテーション
                    Instruction::ASL => {
                        let addr = get_addr(self.addr_h, self.addr_l);
                        let mut operand = if self.is_accumulator {
                            self.a
                        } else {
                            self.bus.read(rom, apu, ppu, pad, addr)
                        };
                        self.p.c = (operand & 0x80) != 0;
                        operand <<= 1;
                        self.set_nz(operand);
                        if self.is_accumulator {
                            self.a = operand;
                        } else {
                            self.suspend_cycle += self.bus.write(apu, ppu, addr, operand);
                        }
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::LSR => {
                        let addr = get_addr(self.addr_h, self.addr_l);
                        let mut operand = if self.is_accumulator {
                            self.a
                        } else {
                            self.bus.read(rom, apu, ppu, pad, addr)
                        };
                        self.p.c = (operand & 0x01) != 0;
                        operand >>= 1;
                        self.set_nz(operand);
                        if self.is_accumulator {
                            self.a = operand;
                        } else {
                            self.suspend_cycle += self.bus.write(apu, ppu, addr, operand);
                        }
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::ROL => {
                        let addr = get_addr(self.addr_h, self.addr_l);
                        let mut operand = if self.is_accumulator {
                            self.a
                        } else {
                            self.bus.read(rom, apu, ppu, pad, addr)
                        };
                        let carry = self.p.c;
                        self.p.c = (operand & 0x80) != 0;
                        operand <<= 1;
                        if carry {
                            operand |= 1;
                        }
                        self.set_nz(operand);
                        if self.is_accumulator {
                            self.a = operand;
                        } else {
                            self.suspend_cycle += self.bus.write(apu, ppu, addr, operand);
                        }
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::ROR => {
                        let addr = get_addr(self.addr_h, self.addr_l);
                        let mut operand = if self.is_accumulator {
                            self.a
                        } else {
                            self.bus.read(rom, apu, ppu, pad, addr)
                        };
                        let carry = self.p.c;
                        self.p.c = (operand & 0x01) != 0;
                        operand >>= 1;
                        if carry {
                            operand |= 0x80;
                        }
                        self.set_nz(operand);
                        if self.is_accumulator {
                            self.a = operand;
                        } else {
                            self.suspend_cycle += self.bus.write(apu, ppu, addr, operand);
                        }
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    //条件分岐
                    Instruction::BCC
                    | Instruction::BCS
                    | Instruction::BEQ
                    | Instruction::BNE
                    | Instruction::BVC
                    | Instruction::BVS
                    | Instruction::BPL
                    | Instruction::BMI => {
                        let jump = self.step != 0
                            || match INSTRUCTION_SET[self.op as usize].instruction {
                                Instruction::BCC => !self.p.c,
                                Instruction::BCS => self.p.c,
                                Instruction::BEQ => self.p.z,
                                Instruction::BNE => !self.p.z,
                                Instruction::BVC => !self.p.v,
                                Instruction::BVS => self.p.v,
                                Instruction::BPL => !self.p.n,
                                Instruction::BMI => self.p.n,
                                _ => panic!(),
                            };
                        if jump {
                            match self.step {
                                0 => {
                                    let offset = self.bus.read(rom, apu, ppu, pad, self.pc) as i8 as i16;
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
                    Instruction::BIT => {
                        let operand = if self.is_immediate {
                            self.immediate_operand
                        } else {
                            self.bus.read(rom, apu, ppu, pad, get_addr(self.addr_h, self.addr_l))
                        };
                        let result = self.a & operand;
                        self.p.n = (operand & 0x80) != 0;
                        self.p.v = (operand & 0x40) != 0;
                        self.p.z = result == 0;
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    //ジャンプ
                    Instruction::JMP => match self.step {
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
                    Instruction::JSR => {
                        let pc = self.pc.wrapping_sub(1);
                        match self.step {
                            0 => self.push((pc >> 8) as u8),
                            1 => self.push(pc as u8),
                            2 => {
                                self.pc = get_addr(self.addr_h, self.addr_l);
                                self.state = CpuState::ReadOpcode;
                                self.step = 0;
                                return;
                            }
                            _ => panic!(),
                        }
                    }
                    Instruction::RTS => match self.step {
                        0 | 1 => {}
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
                    Instruction::BRK => match self.step {
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
                            self.pc |= (self.bus.read(rom, apu, ppu, pad, 0xFFFE) as u16) << 8;
                        }
                        5 => {
                            self.pc &= 0xFF00;
                            self.pc |= self.bus.read(rom, apu, ppu, pad, 0xFFFF) as u16;
                            self.state = CpuState::ReadOpcode;
                            self.step = 0;
                            return;
                        }
                        _ => panic!(),
                    },
                    Instruction::RTI => match self.step {
                        0 | 1 => {}
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
                    Instruction::CMP | Instruction::CPX | Instruction::CPY => {
                        let operand = if self.is_immediate {
                            self.immediate_operand
                        } else {
                            self.bus.read(rom, apu, ppu, pad, get_addr(self.addr_h, self.addr_l))
                        };
                        let reg = match INSTRUCTION_SET[self.op as usize].instruction {
                            Instruction::CMP => self.a,
                            Instruction::CPX => self.x,
                            Instruction::CPY => self.y,
                            _ => panic!(),
                        };
                        let (result, overflow) = reg.overflowing_sub(operand);
                        self.set_nz(result);
                        self.p.c = !overflow;
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    //インクリメント、デクリメント
                    Instruction::INC | Instruction::DEC => match self.step {
                        0 => {}
                        1 => {
                            let operand = if self.is_immediate {
                                self.immediate_operand
                            } else {
                                self.bus.read(rom, apu, ppu, pad, get_addr(self.addr_h, self.addr_l))
                            };
                            let operand = match INSTRUCTION_SET[self.op as usize].instruction {
                                Instruction::INC => operand.wrapping_add(1),
                                Instruction::DEC => operand.wrapping_sub(1),
                                _ => panic!(),
                            };
                            self.set_nz(operand);
                            let addr = get_addr(self.addr_h, self.addr_l);
                            self.suspend_cycle += self.bus.write(apu, ppu, addr, operand);
                        }
                        2 => {
                            self.state = CpuState::ReadOpcode;
                            self.step = 0;
                            return;
                        }
                        _ => panic!(),
                    },
                    Instruction::INX | Instruction::DEX => {
                        self.x = match INSTRUCTION_SET[self.op as usize].instruction {
                            Instruction::INX => self.x.wrapping_add(1),
                            Instruction::DEX => self.x.wrapping_sub(1),
                            _ => panic!(),
                        };
                        self.set_nz(self.x);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::INY | Instruction::DEY => {
                        self.y = match INSTRUCTION_SET[self.op as usize].instruction {
                            Instruction::INY => self.y.wrapping_add(1),
                            Instruction::DEY => self.y.wrapping_sub(1),
                            _ => panic!(),
                        };
                        self.set_nz(self.y);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    //フラグ操作
                    Instruction::CLC => {
                        self.p.c = false;
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::SEC => {
                        self.p.c = true;
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::CLI => {
                        self.p.i = false;
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::SEI => {
                        self.p.i = true;
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::CLD => {
                        self.p.d = false;
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::SED => {
                        self.p.d = true;
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::CLV => {
                        self.p.v = false;
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    //ロード
                    Instruction::LDA => {
                        self.a = if self.is_immediate {
                            self.immediate_operand
                        } else {
                            self.bus.read(rom, apu, ppu, pad, get_addr(self.addr_h, self.addr_l))
                        };
                        self.set_nz(self.a);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::LDX => {
                        self.x = if self.is_immediate {
                            self.immediate_operand
                        } else {
                            self.bus.read(rom, apu, ppu, pad, get_addr(self.addr_h, self.addr_l))
                        };
                        self.set_nz(self.x);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::LDY => {
                        self.y = if self.is_immediate {
                            self.immediate_operand
                        } else {
                            self.bus.read(rom, apu, ppu, pad, get_addr(self.addr_h, self.addr_l))
                        };
                        self.set_nz(self.y);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    //ストア
                    Instruction::STA | Instruction::STX | Instruction::STY => {
                        let addr = get_addr(self.addr_h, self.addr_l);
                        let val = match INSTRUCTION_SET[self.op as usize].instruction {
                            Instruction::STA => self.a,
                            Instruction::STX => self.x,
                            Instruction::STY => self.y,
                            _ => panic!(),
                        };
                        self.suspend_cycle += self.bus.write(apu, ppu, addr, val);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    //レジスタ間転送
                    Instruction::TAX => {
                        self.x = self.a;
                        self.set_nz(self.x);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::TSX => {
                        self.x = self.sp;
                        self.set_nz(self.x);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::TXA => {
                        self.a = self.x;
                        self.set_nz(self.a);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::TYA => {
                        self.a = self.y;
                        self.set_nz(self.a);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::TAY => {
                        self.y = self.a;
                        self.set_nz(self.y);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    Instruction::TXS => {
                        self.sp = self.x;
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    //スタック
                    Instruction::PHA | Instruction::PHP => match self.step {
                        0 => {}
                        1 => {
                            self.push(match INSTRUCTION_SET[self.op as usize].instruction {
                                Instruction::PHA => self.a,
                                Instruction::PHP => self.p.read() | 0b00110000,
                                _ => panic!(),
                            });
                            self.state = CpuState::ReadOpcode;
                            self.step = 0;
                        }
                        _ => panic!(),
                    },
                    Instruction::PLA => match self.step {
                        0 | 1 => {}
                        2 => {
                            self.a = self.pop();
                            self.set_nz(self.a);
                            self.state = CpuState::ReadOpcode;
                            self.step = 0;
                        }
                        _ => panic!(),
                    },
                    Instruction::PLP => match self.step {
                        0 | 1 => {}
                        2 => {
                            let result = self.pop() & 0b1110_1111; //割り込みじゃないのでBフラグを落とす
                            self.p.write(result);
                            self.state = CpuState::ReadOpcode;
                            self.step = 0;
                        }
                        _ => panic!(),
                    },
                    Instruction::NOP => {
                        // Multi-byte NOPs: consume the addressing mode bytes but do nothing
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                    }
                    // ===== Unofficial Opcodes =====
                    // LAX: LDA + LDX
                    Instruction::LAX => {
                        let operand = if self.is_immediate {
                            self.immediate_operand
                        } else {
                            self.bus.read(rom, apu, ppu, pad, get_addr(self.addr_h, self.addr_l))
                        };
                        self.a = operand;
                        self.x = operand;
                        self.set_nz(operand);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    // SAX: Store A & X
                    Instruction::SAX => {
                        let addr = get_addr(self.addr_h, self.addr_l);
                        self.suspend_cycle += self.bus.write(apu, ppu, addr, self.a & self.x);
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                        return;
                    }
                    // DCP: DEC + CMP
                    Instruction::DCP => match self.step {
                        0 => {}
                        1 => {
                            let addr = get_addr(self.addr_h, self.addr_l);
                            let mut operand = self.bus.read(rom, apu, ppu, pad, addr);
                            operand = operand.wrapping_sub(1);
                            self.suspend_cycle += self.bus.write(apu, ppu, addr, operand);
                            let (result, overflow) = self.a.overflowing_sub(operand);
                            self.set_nz(result);
                            self.p.c = !overflow;
                        }
                        2 => {
                            self.state = CpuState::ReadOpcode;
                            self.step = 0;
                            return;
                        }
                        _ => panic!(),
                    },
                    // ISB (ISC): INC + SBC
                    Instruction::ISB => match self.step {
                        0 => {}
                        1 => {
                            let addr = get_addr(self.addr_h, self.addr_l);
                            let mut operand = self.bus.read(rom, apu, ppu, pad, addr);
                            operand = operand.wrapping_add(1);
                            self.suspend_cycle += self.bus.write(apu, ppu, addr, operand);
                            let (result, overflow) = self.a.overflowing_sub(operand);
                            let (result2, overflow2) = result.overflowing_sub(if self.p.c { 0 } else { 1 });
                            self.p.v = ((self.a ^ operand) & (self.a ^ result2) & 0x80) != 0;
                            self.p.c = !(overflow || overflow2);
                            self.a = result2;
                            self.set_nz(self.a);
                        }
                        2 => {
                            self.state = CpuState::ReadOpcode;
                            self.step = 0;
                            return;
                        }
                        _ => panic!(),
                    },
                    // SLO: ASL + ORA
                    Instruction::SLO => match self.step {
                        0 => {}
                        1 => {
                            let addr = get_addr(self.addr_h, self.addr_l);
                            let mut operand = self.bus.read(rom, apu, ppu, pad, addr);
                            self.p.c = (operand & 0x80) != 0;
                            operand <<= 1;
                            self.suspend_cycle += self.bus.write(apu, ppu, addr, operand);
                            self.a |= operand;
                            self.set_nz(self.a);
                        }
                        2 => {
                            self.state = CpuState::ReadOpcode;
                            self.step = 0;
                            return;
                        }
                        _ => panic!(),
                    },
                    // RLA: ROL + AND
                    Instruction::RLA => match self.step {
                        0 => {}
                        1 => {
                            let addr = get_addr(self.addr_h, self.addr_l);
                            let mut operand = self.bus.read(rom, apu, ppu, pad, addr);
                            let carry = self.p.c;
                            self.p.c = (operand & 0x80) != 0;
                            operand <<= 1;
                            if carry {
                                operand |= 1;
                            }
                            self.suspend_cycle += self.bus.write(apu, ppu, addr, operand);
                            self.a &= operand;
                            self.set_nz(self.a);
                        }
                        2 => {
                            self.state = CpuState::ReadOpcode;
                            self.step = 0;
                            return;
                        }
                        _ => panic!(),
                    },
                    // SRE: LSR + EOR
                    Instruction::SRE => match self.step {
                        0 => {}
                        1 => {
                            let addr = get_addr(self.addr_h, self.addr_l);
                            let mut operand = self.bus.read(rom, apu, ppu, pad, addr);
                            self.p.c = (operand & 0x01) != 0;
                            operand >>= 1;
                            self.suspend_cycle += self.bus.write(apu, ppu, addr, operand);
                            self.a ^= operand;
                            self.set_nz(self.a);
                        }
                        2 => {
                            self.state = CpuState::ReadOpcode;
                            self.step = 0;
                            return;
                        }
                        _ => panic!(),
                    },
                    // RRA: ROR + ADC
                    Instruction::RRA => match self.step {
                        0 => {}
                        1 => {
                            let addr = get_addr(self.addr_h, self.addr_l);
                            let mut operand = self.bus.read(rom, apu, ppu, pad, addr);
                            let carry = self.p.c;
                            self.p.c = (operand & 0x01) != 0;
                            operand >>= 1;
                            if carry {
                                operand |= 0x80;
                            }
                            self.suspend_cycle += self.bus.write(apu, ppu, addr, operand);
                            let (result, ov) = self.a.overflowing_add(operand);
                            let (result2, ov2) = result.overflowing_add(if self.p.c { 1 } else { 0 });
                            self.p.v = ((self.a ^ result2) & (operand ^ result2) & 0x80) != 0;
                            self.p.c = ov || ov2;
                            self.a = result2;
                            self.set_nz(self.a);
                        }
                        2 => {
                            self.state = CpuState::ReadOpcode;
                            self.step = 0;
                            return;
                        }
                        _ => panic!(),
                    },
                    Instruction::Undefined => {
                        // Treat as NOP
                        self.state = CpuState::ReadOpcode;
                        self.step = 0;
                    }
                }
            }
        }
        self.step += 1;
    }

    fn push(&mut self, value: u8) {
        self.suspend_cycle += self.bus.write(&mut None, &mut None, 0x0100 | self.sp as u16, value);
        self.sp = self.sp.wrapping_sub(1);
    }
    fn pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.bus.read(None, &mut None, &mut None, None, 0x0100 | self.sp as u16)
    }
}

#[derive(Clone, Copy)]
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
}

#[derive(Debug, Clone, Copy)]
enum Instruction {
    ADC,
    SBC,
    AND,
    ORA,
    EOR,
    ASL,
    LSR,
    ROL,
    ROR,
    BCC,
    BCS,
    BEQ,
    BNE,
    BVC,
    BVS,
    BPL,
    BMI,
    BIT,
    JMP,
    JSR,
    RTS,
    BRK,
    RTI,
    CMP,
    CPX,
    CPY,
    INC,
    DEC,
    INX,
    DEX,
    INY,
    DEY,
    CLC,
    SEC,
    CLI,
    SEI,
    CLD,
    SED,
    CLV,
    LDA,
    LDX,
    LDY,
    STA,
    STX,
    STY,
    TAX,
    TXA,
    TAY,
    TYA,
    TSX,
    TXS,
    PHA,
    PLA,
    PHP,
    PLP,
    NOP,
    // Unofficial opcodes
    LAX,
    SAX,
    DCP,
    ISB,
    SLO,
    RLA,
    SRE,
    RRA,
    Undefined,
}

struct InstructionDefinition {
    mode: AddressingMode,
    instruction: Instruction,
    #[allow(dead_code)]
    clock: u8,
}

const U: InstructionDefinition =
    InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::Undefined, clock: 1 };

// NOP variants for unofficial multi-byte NOPs
const NOP_IMM: InstructionDefinition =
    InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::NOP, clock: 2 };
const NOP_ZP: InstructionDefinition =
    InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::NOP, clock: 3 };
const NOP_ZPX: InstructionDefinition =
    InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::NOP, clock: 4 };
const NOP_ABS: InstructionDefinition =
    InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::NOP, clock: 4 };
const NOP_ABX: InstructionDefinition =
    InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::NOP, clock: 4 };
const NOP_IMP: InstructionDefinition =
    InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::NOP, clock: 2 };

const INSTRUCTION_SET: [InstructionDefinition; 0x100] = [
    /*00*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::BRK, clock: 7 },
    /*01*/ InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::ORA, clock: 6 },
    /*02*/ U,
    /*03*/ InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::SLO, clock: 8 },
    /*04*/ NOP_ZP,
    /*05*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::ORA, clock: 3 },
    /*06*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::ASL, clock: 5 },
    /*07*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::SLO, clock: 5 },
    /*08*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::PHP, clock: 3 },
    /*09*/ InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::ORA, clock: 2 },
    /*0A*/ InstructionDefinition { mode: AddressingMode::Accumulator, instruction: Instruction::ASL, clock: 2 },
    /*0B*/ U,
    /*0C*/ NOP_ABS,
    /*0D*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::ORA, clock: 4 },
    /*0E*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::ASL, clock: 6 },
    /*0F*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::SLO, clock: 6 },
    /*10*/
    InstructionDefinition { mode: AddressingMode::Relative, instruction: Instruction::BPL, clock: 2 }, //+1or2
    /*11*/
    InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::ORA, clock: 5 }, //+1
    /*12*/ U,
    /*13*/ InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::SLO, clock: 8 },
    /*14*/ NOP_ZPX,
    /*15*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::ORA, clock: 4 },
    /*16*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::ASL, clock: 6 },
    /*17*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::SLO, clock: 6 },
    /*18*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::CLC, clock: 2 },
    /*19*/
    InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::ORA, clock: 4 }, //+1
    /*1A*/ NOP_IMP,
    /*1B*/ InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::SLO, clock: 7 },
    /*1C*/ NOP_ABX,
    /*1D*/
    InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::ORA, clock: 4 }, //+1
    /*1E*/
    InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::ASL, clock: 6 }, //+1
    /*1F*/ InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::SLO, clock: 7 },
    /*20*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::JSR, clock: 6 },
    /*21*/ InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::AND, clock: 6 },
    /*22*/ U,
    /*23*/ InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::RLA, clock: 8 },
    /*24*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::BIT, clock: 3 },
    /*25*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::AND, clock: 3 },
    /*26*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::ROL, clock: 5 },
    /*27*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::RLA, clock: 5 },
    /*28*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::PLP, clock: 4 },
    /*29*/ InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::AND, clock: 2 },
    /*2A*/ InstructionDefinition { mode: AddressingMode::Accumulator, instruction: Instruction::ROL, clock: 2 },
    /*2B*/ U,
    /*2C*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::BIT, clock: 4 },
    /*2D*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::AND, clock: 4 },
    /*2E*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::ROL, clock: 6 },
    /*2F*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::RLA, clock: 6 },
    /*30*/
    InstructionDefinition { mode: AddressingMode::Relative, instruction: Instruction::BMI, clock: 2 }, //+1or2
    /*31*/
    InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::AND, clock: 5 }, //+1
    /*32*/ U,
    /*33*/ InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::RLA, clock: 8 },
    /*34*/ NOP_ZPX,
    /*35*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::AND, clock: 4 },
    /*36*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::ROL, clock: 6 },
    /*37*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::RLA, clock: 6 },
    /*38*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::SEC, clock: 2 },
    /*39*/
    InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::AND, clock: 4 }, //+1
    /*3A*/ NOP_IMP,
    /*3B*/ InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::RLA, clock: 7 },
    /*3C*/ NOP_ABX,
    /*3D*/
    InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::AND, clock: 4 }, //+1
    /*3E*/
    InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::ROL, clock: 6 }, //+1
    /*3F*/ InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::RLA, clock: 7 },
    /*40*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::RTI, clock: 6 },
    /*41*/ InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::EOR, clock: 6 },
    /*42*/ U,
    /*43*/ InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::SRE, clock: 8 },
    /*44*/ NOP_ZP,
    /*45*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::EOR, clock: 3 },
    /*46*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::LSR, clock: 5 },
    /*47*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::SRE, clock: 5 },
    /*48*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::PHA, clock: 3 },
    /*49*/ InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::EOR, clock: 2 },
    /*4A*/ InstructionDefinition { mode: AddressingMode::Accumulator, instruction: Instruction::LSR, clock: 2 },
    /*4B*/ U,
    /*4C*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::JMP, clock: 3 },
    /*4D*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::EOR, clock: 4 },
    /*4E*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::LSR, clock: 6 },
    /*4F*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::SRE, clock: 6 },
    /*50*/
    InstructionDefinition { mode: AddressingMode::Relative, instruction: Instruction::BVC, clock: 2 }, //+1or2
    /*51*/
    InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::EOR, clock: 5 }, //+1
    /*52*/ U,
    /*53*/ InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::SRE, clock: 8 },
    /*54*/ NOP_ZPX,
    /*55*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::EOR, clock: 4 },
    /*56*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::LSR, clock: 6 },
    /*57*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::SRE, clock: 6 },
    /*58*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::CLI, clock: 2 },
    /*59*/
    InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::EOR, clock: 4 }, //+1
    /*5A*/ NOP_IMP,
    /*5B*/ InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::SRE, clock: 7 },
    /*5C*/ NOP_ABX,
    /*5D*/
    InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::EOR, clock: 4 }, //+1
    /*5E*/
    InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::LSR, clock: 6 }, //+1
    /*5F*/ InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::SRE, clock: 7 },
    /*60*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::RTS, clock: 6 },
    /*61*/ InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::ADC, clock: 6 },
    /*62*/ U,
    /*63*/ InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::RRA, clock: 8 },
    /*64*/ NOP_ZP,
    /*65*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::ADC, clock: 3 },
    /*66*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::ROR, clock: 5 },
    /*67*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::RRA, clock: 5 },
    /*68*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::PLA, clock: 4 },
    /*69*/ InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::ADC, clock: 2 },
    /*6A*/ InstructionDefinition { mode: AddressingMode::Accumulator, instruction: Instruction::ROR, clock: 2 },
    /*6B*/ U,
    /*6C*/ InstructionDefinition { mode: AddressingMode::Indirect, instruction: Instruction::JMP, clock: 5 },
    /*6D*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::ADC, clock: 4 },
    /*6E*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::ROR, clock: 6 },
    /*6F*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::RRA, clock: 6 },
    /*70*/
    InstructionDefinition { mode: AddressingMode::Relative, instruction: Instruction::BVS, clock: 2 }, //+1or2
    /*71*/
    InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::ADC, clock: 5 }, //+1
    /*72*/ U,
    /*73*/ InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::RRA, clock: 8 },
    /*74*/ NOP_ZPX,
    /*75*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::ADC, clock: 4 },
    /*76*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::ROR, clock: 6 },
    /*77*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::RRA, clock: 6 },
    /*78*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::SEI, clock: 2 },
    /*79*/
    InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::ADC, clock: 4 }, //+1
    /*7A*/ NOP_IMP,
    /*7B*/ InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::RRA, clock: 7 },
    /*7C*/ NOP_ABX,
    /*7D*/
    InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::ADC, clock: 4 }, //+1
    /*7E*/
    InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::ROR, clock: 6 }, //+1
    /*7F*/ InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::RRA, clock: 7 },
    /*80*/ NOP_IMM,
    /*81*/ InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::STA, clock: 6 },
    /*82*/ NOP_IMM,
    /*83*/ InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::SAX, clock: 6 },
    /*84*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::STY, clock: 3 },
    /*85*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::STA, clock: 3 },
    /*86*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::STX, clock: 3 },
    /*87*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::SAX, clock: 3 },
    /*88*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::DEY, clock: 2 },
    /*89*/ NOP_IMM,
    /*8A*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::TXA, clock: 2 },
    /*8B*/ U,
    /*8C*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::STY, clock: 4 },
    /*8D*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::STA, clock: 4 },
    /*8E*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::STX, clock: 4 },
    /*8F*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::SAX, clock: 4 },
    /*90*/
    InstructionDefinition { mode: AddressingMode::Relative, instruction: Instruction::BCC, clock: 2 }, //+1or2
    /*91*/
    InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::STA, clock: 5 }, //+1
    /*92*/ U,
    /*93*/ U,
    /*94*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::STY, clock: 4 },
    /*95*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::STA, clock: 4 },
    /*96*/ InstructionDefinition { mode: AddressingMode::ZeroPageY, instruction: Instruction::STX, clock: 4 },
    /*97*/ InstructionDefinition { mode: AddressingMode::ZeroPageY, instruction: Instruction::SAX, clock: 4 },
    /*98*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::TYA, clock: 2 },
    /*99*/
    InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::STA, clock: 4 }, //+1
    /*9A*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::TXS, clock: 2 },
    /*9B*/ U,
    /*9C*/ U,
    /*9D*/
    InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::STA, clock: 4 }, //+1
    /*9E*/ U,
    /*9F*/ U,
    /*A0*/ InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::LDY, clock: 2 },
    /*A1*/ InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::LDA, clock: 6 },
    /*A2*/ InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::LDX, clock: 2 },
    /*A3*/ InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::LAX, clock: 6 },
    /*A4*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::LDY, clock: 3 },
    /*A5*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::LDA, clock: 3 },
    /*A6*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::LDX, clock: 3 },
    /*A7*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::LAX, clock: 3 },
    /*A8*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::TAY, clock: 2 },
    /*A9*/ InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::LDA, clock: 2 },
    /*AA*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::TAX, clock: 2 },
    /*AB*/ InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::LAX, clock: 2 },
    /*AC*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::LDY, clock: 4 },
    /*AD*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::LDA, clock: 4 },
    /*AE*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::LDX, clock: 4 },
    /*AF*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::LAX, clock: 4 },
    /*B0*/
    InstructionDefinition { mode: AddressingMode::Relative, instruction: Instruction::BCS, clock: 2 }, //+1or2
    /*B1*/
    InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::LDA, clock: 5 }, //+1
    /*B2*/ U,
    /*B3*/ InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::LAX, clock: 5 },
    /*B4*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::LDY, clock: 4 },
    /*B5*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::LDA, clock: 4 },
    /*B6*/ InstructionDefinition { mode: AddressingMode::ZeroPageY, instruction: Instruction::LDX, clock: 4 },
    /*B7*/ InstructionDefinition { mode: AddressingMode::ZeroPageY, instruction: Instruction::LAX, clock: 4 },
    /*B8*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::CLV, clock: 2 },
    /*B9*/
    InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::LDA, clock: 4 }, //+1
    /*BA*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::TSX, clock: 2 },
    /*BB*/ U,
    /*BC*/
    InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::LDY, clock: 4 }, //+1
    /*BD*/
    InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::LDA, clock: 4 }, //+1
    /*BE*/
    InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::LDX, clock: 4 }, //+1
    /*BF*/ InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::LAX, clock: 4 },
    /*C0*/ InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::CPY, clock: 2 },
    /*C1*/ InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::CMP, clock: 6 },
    /*C2*/ NOP_IMM,
    /*C3*/ InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::DCP, clock: 8 },
    /*C4*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::CPY, clock: 3 },
    /*C5*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::CMP, clock: 3 },
    /*C6*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::DEC, clock: 5 },
    /*C7*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::DCP, clock: 5 },
    /*C8*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::INY, clock: 2 },
    /*C9*/ InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::CMP, clock: 2 },
    /*CA*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::DEX, clock: 2 },
    /*CB*/ U,
    /*CC*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::CPY, clock: 4 },
    /*CD*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::CMP, clock: 4 },
    /*CE*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::DEC, clock: 6 },
    /*CF*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::DCP, clock: 6 },
    /*D0*/
    InstructionDefinition { mode: AddressingMode::Relative, instruction: Instruction::BNE, clock: 2 }, //+1or2
    /*D1*/
    InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::CMP, clock: 5 }, //+1
    /*D2*/ U,
    /*D3*/ InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::DCP, clock: 8 },
    /*D4*/ NOP_ZPX,
    /*D5*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::CMP, clock: 4 },
    /*D6*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::DEC, clock: 6 },
    /*D7*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::DCP, clock: 6 },
    /*D8*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::CLD, clock: 2 },
    /*D9*/
    InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::CMP, clock: 4 }, //+1
    /*DA*/ NOP_IMP,
    /*DB*/ InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::DCP, clock: 7 },
    /*DC*/ NOP_ABX,
    /*DD*/
    InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::CMP, clock: 4 }, //+1
    /*DE*/
    InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::DEC, clock: 6 }, //+1
    /*DF*/ InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::DCP, clock: 7 },
    /*E0*/ InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::CPX, clock: 2 },
    /*E1*/ InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::SBC, clock: 6 },
    /*E2*/ NOP_IMM,
    /*E3*/ InstructionDefinition { mode: AddressingMode::IndirectX, instruction: Instruction::ISB, clock: 8 },
    /*E4*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::CPX, clock: 3 },
    /*E5*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::SBC, clock: 3 },
    /*E6*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::INC, clock: 5 },
    /*E7*/ InstructionDefinition { mode: AddressingMode::ZeroPage, instruction: Instruction::ISB, clock: 5 },
    /*E8*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::INX, clock: 2 },
    /*E9*/ InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::SBC, clock: 2 },
    /*EA*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::NOP, clock: 2 },
    /*EB*/
    InstructionDefinition { mode: AddressingMode::Immediate, instruction: Instruction::SBC, clock: 2 }, // Unofficial SBC #imm
    /*EC*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::CPX, clock: 4 },
    /*ED*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::SBC, clock: 4 },
    /*EE*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::INC, clock: 6 },
    /*EF*/ InstructionDefinition { mode: AddressingMode::Absolute, instruction: Instruction::ISB, clock: 6 },
    /*F0*/
    InstructionDefinition { mode: AddressingMode::Relative, instruction: Instruction::BEQ, clock: 2 }, //+1or2
    /*F1*/
    InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::SBC, clock: 5 }, //+1
    /*F2*/ U,
    /*F3*/ InstructionDefinition { mode: AddressingMode::IndirectY, instruction: Instruction::ISB, clock: 8 },
    /*F4*/ NOP_ZPX,
    /*F5*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::SBC, clock: 4 },
    /*F6*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::INC, clock: 6 },
    /*F7*/ InstructionDefinition { mode: AddressingMode::ZeroPageX, instruction: Instruction::ISB, clock: 6 },
    /*F8*/ InstructionDefinition { mode: AddressingMode::Implied, instruction: Instruction::SED, clock: 2 },
    /*F9*/
    InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::SBC, clock: 4 }, //+1
    /*FA*/ NOP_IMP,
    /*FB*/ InstructionDefinition { mode: AddressingMode::AbsoluteY, instruction: Instruction::ISB, clock: 7 },
    /*FC*/ NOP_ABX,
    /*FD*/
    InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::SBC, clock: 4 }, //+1
    /*FE*/
    InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::INC, clock: 6 }, //+1
    /*FF*/ InstructionDefinition { mode: AddressingMode::AbsoluteX, instruction: Instruction::ISB, clock: 7 },
];

#[derive(Default)]
struct Divider {
    period: u8,
}
impl Divider {
    fn clock(&mut self, decay_level_counter: &mut Option<&mut DecayLevelCounter>, loop_flag: bool, period: u8) {
        if self.period == 0 {
            self.load(period);
            if let Some(ref mut dlc) = decay_level_counter {
                dlc.clock(loop_flag);
            }
        } else {
            self.period -= 1;
        }
    }
    fn load(&mut self, period: u8) {
        self.period = period;
    }
}

#[derive(Default)]
struct DecayLevelCounter {
    count: u8,
}
impl DecayLevelCounter {
    fn clock(&mut self, loop_flag: bool) {
        if self.count == 0 {
            if loop_flag {
                self.reset();
            }
        } else {
            self.count -= 1;
        }
    }
    fn reset(&mut self) {
        self.count = 15;
    }
}

#[derive(Default)]
struct Envelope {
    start: bool,
    divider: Divider,
    decay_level_counter: DecayLevelCounter,
    output: u8,
    volume: u8,
    constant_volume: bool,
}
impl Envelope {
    fn clock(&mut self, loop_flag: bool) {
        if self.start {
            self.start = false;
            self.decay_level_counter.reset();
            self.divider.load(self.volume);
        } else {
            self.divider
                .clock(&mut Some(&mut self.decay_level_counter), loop_flag, self.volume);
        }
        self.output = if self.constant_volume {
            self.volume
        } else {
            //println!("{}", self.decay_level_counter.count);
            self.decay_level_counter.count
        }
    }
}

#[derive(Default)]
struct Sweep {
    divider: Divider,
    reload_flag: bool,
    enabled_flag: bool,
    divider_period: u8,
    negate_flag: bool,
    shift_count: u8,
    mute: bool,
}
impl Sweep {
    fn clock(&mut self, timer: u16, is_pulse_1: bool) -> u16 {
        let mut change_amount = (timer >> self.shift_count) as i16;
        if self.negate_flag {
            change_amount = if is_pulse_1 { -change_amount - 1 } else { -change_amount }
        }
        let target_period = timer.wrapping_add(change_amount as u16);

        self.mute = (timer < 8) || (target_period > 0x7FF);

        let result = if self.enabled_flag && self.shift_count != 0 && self.divider.period == 0 && !self.mute {
            target_period
        } else {
            timer
        };

        if self.divider.period == 0 || self.reload_flag {
            self.divider.load(self.divider_period);
            self.reload_flag = false;
        } else {
            self.divider.clock(&mut None, false, self.divider_period); //TODO: 引数検討
        }

        result
    }
    fn setup(&mut self, enabled: bool, divider_period: u8, negate: bool, shift_count: u8) {
        self.enabled_flag = enabled;
        self.divider_period = divider_period;
        self.negate_flag = negate;
        self.shift_count = shift_count;
        self.reload_flag = true;
    }
}

#[derive(Default)]
struct Triangle {
    length_counter_halt: bool,
    liner_counter_reload_value: u8,
    liner_counter: u8,
    timer: u16,
    current_time: u16,
    current_sequencer_position: u8,
    length_counter: LengthCounter,
    liner_counter_reload_flag: bool,
}
impl Triangle {
    const OUTPUT: [u8; 32] = [
        15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, //
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
    ];
    fn clock_linear_counter(&mut self) {
        if self.liner_counter_reload_flag {
            self.liner_counter = self.liner_counter_reload_value;
        } else if self.liner_counter != 0 {
            self.liner_counter -= 1;
        }
        if !self.length_counter_halt {
            self.liner_counter_reload_flag = false;
        }
    }
    fn liner_counter_setup(&mut self, control_flag: bool, counter_reload_value: u8) {
        self.length_counter_halt = control_flag;
        self.liner_counter_reload_value = counter_reload_value;
    }
    #[inline(always)]
    fn clock(&mut self) {
        if self.current_time == 0 {
            self.current_time = self.timer;
            if self.liner_counter != 0 && self.length_counter.length != 0 {
                self.current_sequencer_position += 1;
                self.current_sequencer_position %= 32;
            }
        } else {
            self.current_time -= 1;
        }
    }
    #[inline(always)]
    fn get_value(&self) -> u8 {
        if self.timer >= 2 {
            Triangle::OUTPUT[self.current_sequencer_position as usize]
        } else {
            0
        }
    }
    fn set_timer_low(&mut self, value: u8) {
        self.timer &= 0xFF00;
        self.timer |= value as u16;
    }
    fn set_timer_high(&mut self, value: u8) {
        self.timer &= 0x00FF;
        self.timer |= (value as u16) << 8;
    }
}

struct LinearFeedbackShiftRegister {
    register: u16,
    mode_flag: bool,
}
impl LinearFeedbackShiftRegister {
    #[inline(always)]
    fn clock(&mut self) {
        let feedback = ((self.register & 0x01)
            ^ if self.mode_flag {
                (self.register & 0x40) >> 6
            } else {
                (self.register & 0x02) >> 1
            })
            == 0x01;
        self.register >>= 1;
        if feedback {
            self.register |= 1 << 14;
        }
    }
}
impl Default for LinearFeedbackShiftRegister {
    fn default() -> Self {
        Self { register: 1, mode_flag: false }
    }
}

#[derive(Default)]
struct Noise {
    envelope: Envelope,
    shift_register: LinearFeedbackShiftRegister,
    timer: u16,
    current_time: u16,
    length_counter: LengthCounter,
    length_counter_halt: bool,
}
impl Noise {
    const TIMER_PERIOD: [u16; 0x10] = [
        4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
    ];
    #[inline(always)]
    fn clock(&mut self) {
        if self.current_time == 0 {
            self.current_time = self.timer;
            self.shift_register.clock();
        } else {
            self.current_time -= 1;
        }
    }
    fn set_timer_period(&mut self, rate: u8) {
        self.timer = Noise::TIMER_PERIOD[rate as usize];
    }
    fn clock_envelope(&mut self) {
        self.envelope.clock(self.length_counter_halt);
    }
    fn clock_length_counter(&mut self) {
        self.length_counter.clock(self.length_counter_halt);
    }
    #[inline(always)]
    fn get_value(&self) -> u8 {
        if (self.shift_register.register & 0x01) == 0x01 || self.length_counter.length == 0 {
            0
        } else {
            self.envelope.output
        }
    }
}

#[derive(Default)]
struct Pulse {
    duty: u8,
    length_counter_halt: bool,
    timer: u16,
    current_time: u16,
    current_sequencer_position: u8,
    envelope: Envelope,
    sweep: Sweep,
    length_counter: LengthCounter,
    last_output: u8,
    is_pulse_1: bool,
}
impl Pulse {
    const OUTPUT: [[bool; 8]; 4] = [
        [false, true, false, false, false, false, false, false],
        [false, true, true, false, false, false, false, false],
        [false, true, true, true, true, false, false, false],
        [true, false, false, true, true, true, true, true],
    ];
    fn clock_envelope(&mut self) {
        self.envelope.clock(self.length_counter_halt);
    }
    fn clock_length_counter(&mut self) {
        self.length_counter.clock(self.length_counter_halt);
    }
    fn clock_sweep(&mut self) {
        self.timer = self.sweep.clock(self.timer, self.is_pulse_1);
    }
    fn set_sweep(&mut self, enabled: bool, divider_period: u8, negate: bool, shift_count: u8) {
        self.sweep.setup(enabled, divider_period, negate, shift_count);
    }
    fn reset_sequencer(&mut self) {
        self.current_sequencer_position = 0;
    }
    #[inline(always)]
    fn clock(&mut self) {
        let envelope = self.envelope.output;
        let length = self.length_counter.length;
        if self.current_time == 0 {
            self.current_time = self.timer;
            self.current_sequencer_position += 1;
            self.current_sequencer_position %= 8;
        } else {
            self.current_time -= 1;
        }
        let sequencer = Pulse::OUTPUT[self.duty as usize][self.current_sequencer_position as usize];
        self.last_output = if self.sweep.mute || !sequencer || length == 0 || self.timer < 8 {
            0
        } else {
            envelope
        };
    }
    #[inline(always)]
    fn get_value(&self) -> u8 {
        self.last_output
    }
    fn set_timer_low(&mut self, value: u8) {
        self.timer &= 0xFF00;
        self.timer |= value as u16;
    }
    fn set_timer_high(&mut self, value: u8) {
        self.timer &= 0x00FF;
        self.timer |= (value as u16) << 8;
    }
}

/// DMC (Delta Modulation Channel)
struct Dmc {
    irq_enabled: bool,
    loop_flag: bool,
    rate_index: u8,
    timer: u16,
    current_time: u16,
    output_level: u8,
    sample_address: u16,
    sample_length: u16,
    current_address: u16,
    bytes_remaining: u16,
    sample_buffer: Option<u8>,
    shift_register: u8,
    bits_remaining: u8,
    silence_flag: bool,
    interrupt_flag: bool,
}
impl Default for Dmc {
    fn default() -> Self {
        Self {
            irq_enabled: false,
            loop_flag: false,
            rate_index: 0,
            timer: Self::RATE_TABLE[0],
            current_time: Self::RATE_TABLE[0],
            output_level: 0,
            sample_address: 0xC000,
            sample_length: 1,
            current_address: 0xC000,
            bytes_remaining: 0,
            sample_buffer: None,
            shift_register: 0,
            bits_remaining: 0,
            silence_flag: true,
            interrupt_flag: false,
        }
    }
}
impl Dmc {
    const RATE_TABLE: [u16; 16] = [
        428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
    ];

    fn clock(&mut self, prg_rom: &[u8]) {
        if self.current_time == 0 {
            self.current_time = self.timer;
            if !self.silence_flag {
                if self.shift_register & 1 != 0 {
                    if self.output_level <= 125 {
                        self.output_level += 2;
                    }
                } else if self.output_level >= 2 {
                    self.output_level -= 2;
                }
                self.shift_register >>= 1;
            }
            self.bits_remaining = self.bits_remaining.saturating_sub(1);
            if self.bits_remaining == 0 {
                self.bits_remaining = 8;
                if let Some(buffer) = self.sample_buffer.take() {
                    self.silence_flag = false;
                    self.shift_register = buffer;
                } else {
                    self.silence_flag = true;
                }
                if self.sample_buffer.is_none() && self.bytes_remaining > 0 {
                    self.fetch_sample(prg_rom);
                }
            }
        } else {
            self.current_time -= 1;
        }
    }

    fn fetch_sample(&mut self, prg_rom: &[u8]) {
        if self.bytes_remaining > 0 {
            let addr = self.current_address;
            let mut rom_addr = (addr.wrapping_sub(0x8000)) as usize;
            if !prg_rom.is_empty() && rom_addr >= prg_rom.len() {
                rom_addr %= prg_rom.len();
            }
            self.sample_buffer = if prg_rom.is_empty() {
                Some(0)
            } else {
                Some(prg_rom[rom_addr])
            };
            self.current_address = self.current_address.wrapping_add(1);
            if self.current_address == 0 {
                self.current_address = 0x8000;
            }
            self.bytes_remaining -= 1;
            if self.bytes_remaining == 0 {
                if self.loop_flag {
                    self.restart();
                } else if self.irq_enabled {
                    self.interrupt_flag = true;
                }
            }
        }
    }

    fn restart(&mut self) {
        self.current_address = self.sample_address;
        self.bytes_remaining = self.sample_length;
    }

    #[inline(always)]
    fn get_value(&self) -> u8 {
        self.output_level
    }
}

struct FrameCounter {
    mode: bool,              //true: 5-step sequence, false: 4-step sequence
    interrupt_inhibit: bool, //割り込み禁止フラグ
    count: u16,
    interrupt_flag: bool,
}
impl FrameCounter {
    fn clock(&mut self, pulse1: &mut Pulse, pulse2: &mut Pulse, triangle: &mut Triangle, noise: &mut Noise) {
        match self.count {
            3728 => {
                //エンベローブ, 三角波線形カウンタ
                pulse1.clock_envelope();
                pulse2.clock_envelope();
                noise.clock_envelope();
                triangle.clock_linear_counter();
            }
            7456 => {
                //エンベローブ, 三角波線形カウンタ
                //長さカウンタ, スイープユニット
                pulse1.clock_envelope();
                pulse2.clock_envelope();
                noise.clock_envelope();
                triangle.clock_linear_counter();
                pulse1.clock_length_counter();
                pulse2.clock_length_counter();
                noise.clock_length_counter();
                pulse1.clock_sweep();
                pulse2.clock_sweep();
            }
            11185 => {
                //エンベローブ, 三角波線形カウンタ
                pulse1.clock_envelope();
                pulse2.clock_envelope();
                noise.clock_envelope();
                triangle.clock_linear_counter();
            }
            14914 => {
                if !self.mode {
                    //エンベローブ, 三角波線形カウンタ
                    //長さカウンタ, スイープユニット
                    //割り込み
                    pulse1.clock_envelope();
                    pulse2.clock_envelope();
                    noise.clock_envelope();
                    triangle.clock_linear_counter();
                    pulse1.clock_length_counter();
                    pulse2.clock_length_counter();
                    noise.clock_length_counter();
                    pulse1.clock_sweep();
                    pulse2.clock_sweep();
                    self.count = 0;
                    self.interrupt_flag |= !self.interrupt_inhibit;
                    return;
                }
            }
            18640 => {
                //エンベローブ, 三角波線形カウンタ
                //長さカウンタ, スイープユニット
                pulse1.clock_envelope();
                pulse2.clock_envelope();
                noise.clock_envelope();
                triangle.clock_linear_counter();
                pulse1.clock_length_counter();
                pulse2.clock_length_counter();
                noise.clock_length_counter();
                pulse1.clock_sweep();
                pulse2.clock_sweep();
                self.count = 0;
                return;
            }
            _ => {}
        }
        self.count += 1;
    }
    fn set(
        &mut self,
        mode: bool,
        interrupt_inhibit: bool,
        pulse1: &mut Pulse,
        pulse2: &mut Pulse,
        triangle: &mut Triangle,
        noise: &mut Noise,
    ) {
        self.mode = mode;
        self.interrupt_inhibit = interrupt_inhibit;
        self.count = 0;
        if mode {
            //エンベローブ, 三角波線形カウンタ
            //長さカウンタ, スイープユニット
            pulse1.clock_envelope();
            pulse2.clock_envelope();
            noise.clock_envelope();
            triangle.clock_linear_counter();
            pulse1.clock_length_counter();
            pulse2.clock_length_counter();
            noise.clock_length_counter();
            pulse1.clock_sweep();
            pulse2.clock_sweep();
        }
    }
}

#[derive(Default)]
struct LengthCounter {
    length: u8,
    enable: bool,
}
impl LengthCounter {
    const LENGTH_TABLE: [u8; 0x20] = [
        10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, //00-0F
        12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30, //10-1F
    ];
    fn clock(&mut self, length_counter_halt: bool) {
        if self.enable {
            if !length_counter_halt && self.length != 0 {
                self.length -= 1;
            }
        } else {
            self.length = 0;
        }
    }
    fn set_length(&mut self, length_counter_load: u8) {
        if self.enable {
            self.length = LengthCounter::LENGTH_TABLE[length_counter_load as usize];
        }
    }
    fn set_enable(&mut self, value: bool) {
        self.enable = value;
        if !value {
            self.length = 0;
        }
    }
}

struct Mixer {
    pulse_table: [f32; 31],
    tnd_table: [f32; 203],
}
impl Mixer {
    fn new() -> Self {
        let mut pulse_table = [0.0f32; 31];
        for i in 0..31 {
            pulse_table[i] = if i == 0 {
                0.0
            } else {
                95.52 / (8128.0 / (i as f32) + 100.0)
            };
        }
        let mut tnd_table = [0.0f32; 203];
        for i in 0..203 {
            tnd_table[i] = if i == 0 {
                0.0
            } else {
                163.67 / (24329.0 / (i as f32) + 100.0)
            };
        }
        Mixer { pulse_table, tnd_table }
    }
    #[inline(always)]
    fn mix(&self, pulse1: u8, pulse2: u8, triangle: u8, noise: u8, dmc: u8) -> f32 {
        self.pulse_table[(pulse1 + pulse2) as usize]
            + self.tnd_table[(3 * triangle as u16 + 2 * noise as u16 + dmc as u16) as usize]
    }
}

pub struct Apu {
    pulse1: Pulse,
    pulse2: Pulse,
    triangle: Triangle,
    noise: Noise,
    dmc: Dmc,
    frame_counter: FrameCounter,
    mixer: Mixer,
    clock_count: u8,
}

impl Apu {
    pub fn new() -> Self {
        Apu {
            pulse1: Pulse { is_pulse_1: true, ..Default::default() },
            pulse2: Pulse { ..Default::default() },
            triangle: Triangle { ..Default::default() },
            noise: Default::default(),
            dmc: Default::default(),
            frame_counter: FrameCounter { mode: false, interrupt_inhibit: false, count: 0, interrupt_flag: false },
            mixer: Mixer::new(),
            clock_count: 0,
        }
    }

    /// Single CPU cycle clock - returns the raw mixed output
    #[inline(always)]
    pub fn clock(&mut self, prg_rom: &[u8]) -> f32 {
        if self.clock_count == 0 {
            self.frame_counter
                .clock(&mut self.pulse1, &mut self.pulse2, &mut self.triangle, &mut self.noise);
            self.pulse1.clock();
            self.pulse2.clock();
            self.noise.clock();
            self.dmc.clock(prg_rom);
        }
        self.triangle.clock();

        let pulse1 = self.pulse1.get_value();
        let pulse2 = self.pulse2.get_value();
        let triangle = self.triangle.get_value();
        let noise = self.noise.get_value();
        let dmc = self.dmc.get_value();

        self.clock_count ^= 1;
        self.mixer.mix(pulse1, pulse2, triangle, noise, dmc)
    }

    pub fn write(&mut self, addr: u8, value: u8) {
        match addr {
            0x00 => {
                self.pulse1.duty = (value & 0b1100_0000) >> 6;
                self.pulse1.length_counter_halt = (value & 0b0010_0000) == 0b0010_0000;
                self.pulse1.envelope.constant_volume = (value & 0b0001_0000) == 0b0001_0000;
                self.pulse1.envelope.volume = value & 0x0F;
            }
            0x04 => {
                self.pulse2.duty = (value & 0b1100_0000) >> 6;
                self.pulse2.length_counter_halt = (value & 0b0010_0000) == 0b0010_0000;
                self.pulse2.envelope.constant_volume = (value & 0b0001_0000) == 0b0001_0000;
                self.pulse2.envelope.volume = value & 0x0F;
            }
            0x08 => self.triangle.liner_counter_setup(value & 0x80 == 0x80, value & 0x7F),
            0x0C => {
                self.noise.length_counter_halt = value & 0x20 == 0x20;
                self.noise.envelope.constant_volume = value & 0x10 == 0x10;
                self.noise.envelope.volume = value & 0x0F;
            }
            0x01 => {
                let enabled = (value & 0b1000_0000) == 0b1000_0000;
                let period = (value & 0b0111_0000) >> 4;
                let negate = (value & 0b0000_1000) == 0b0000_1000;
                let shift_count = value & 0b0000_0111;
                self.pulse1.set_sweep(enabled, period, negate, shift_count);
            }
            0x05 => {
                let enabled = (value & 0b1000_0000) == 0b1000_0000;
                let period = (value & 0b0111_0000) >> 4;
                let negate = (value & 0b0000_1000) == 0b0000_1000;
                let shift_count = value & 0b0000_0111;
                self.pulse2.set_sweep(enabled, period, negate, shift_count);
            }
            0x02 => self.pulse1.set_timer_low(value),
            0x06 => self.pulse2.set_timer_low(value),
            0x0A => self.triangle.set_timer_low(value),
            0x0E => {
                self.noise.shift_register.mode_flag = value & 0x80 == 0x80;
                self.noise.set_timer_period(value & 0x0F);
            }
            0x03 => {
                self.pulse1.length_counter.set_length((value & 0xF8) >> 3);
                self.pulse1.set_timer_high(value & 0b111);
                self.pulse1.envelope.start = true;
                self.pulse1.reset_sequencer();
            }
            0x07 => {
                self.pulse2.length_counter.set_length((value & 0xF8) >> 3);
                self.pulse2.set_timer_high(value & 0b111);
                self.pulse2.envelope.start = true;
                self.pulse2.reset_sequencer();
            }
            0x0B => {
                self.triangle.length_counter.set_length((value & 0xF8) >> 3);
                self.triangle.set_timer_high(value & 0b111);
                self.triangle.liner_counter_reload_flag = true;
            }
            0x0F => {
                self.noise.length_counter.set_length((value & 0xF8) >> 3);
                self.noise.envelope.start = true;
            }
            // DMC registers
            0x10 => {
                self.dmc.irq_enabled = value & 0x80 != 0;
                self.dmc.loop_flag = value & 0x40 != 0;
                self.dmc.rate_index = value & 0x0F;
                self.dmc.timer = Dmc::RATE_TABLE[self.dmc.rate_index as usize];
                if !self.dmc.irq_enabled {
                    self.dmc.interrupt_flag = false;
                }
            }
            0x11 => {
                self.dmc.output_level = value & 0x7F;
            }
            0x12 => {
                self.dmc.sample_address = 0xC000 + (value as u16) * 64;
            }
            0x13 => {
                self.dmc.sample_length = (value as u16) * 16 + 1;
            }
            0x15 => {
                if value & 0b10000 != 0 {
                    if self.dmc.bytes_remaining == 0 {
                        self.dmc.restart();
                    }
                } else {
                    self.dmc.bytes_remaining = 0;
                }
                self.dmc.interrupt_flag = false;
                self.noise.length_counter.set_enable((value & 0b1000) == 0b1000);
                self.triangle.length_counter.set_enable((value & 0b100) == 0b100);
                self.pulse2.length_counter.set_enable((value & 0b10) == 0b10);
                self.pulse1.length_counter.set_enable((value & 0b1) == 0b1);
            }
            0x17 => {
                self.frame_counter.set(
                    (value & 0x80) == 0x80,
                    (value & 0x40) == 0x40,
                    &mut self.pulse1,
                    &mut self.pulse2,
                    &mut self.triangle,
                    &mut self.noise,
                );
            }
            _ => {}
        }
    }

    pub fn read(&mut self, addr: u8) -> u8 {
        match addr {
            0x15 => {
                let mut value: u8 = 0;
                if self.dmc.interrupt_flag {
                    value |= 1 << 7;
                }
                if self.frame_counter.interrupt_flag {
                    value |= 1 << 6;
                }
                self.frame_counter.interrupt_flag = false;
                if self.dmc.bytes_remaining > 0 {
                    value |= 1 << 4;
                }
                if self.noise.length_counter.length > 0 {
                    value |= 1 << 3;
                }
                if self.triangle.length_counter.length > 0 {
                    value |= 1 << 2;
                }
                if self.pulse2.length_counter.length > 0 {
                    value |= 1 << 1;
                }
                if self.pulse1.length_counter.length > 0 {
                    value |= 1;
                }
                value
            }
            _ => 0,
        }
    }

    pub fn check_irq(&self) -> bool {
        self.frame_counter.interrupt_flag || self.dmc.interrupt_flag
    }
}

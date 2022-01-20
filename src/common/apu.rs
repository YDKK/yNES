trait Channel {
  fn clock(&mut self);
  fn get_value(&self) -> u8;
  fn set_length(&mut self, value: u8);
  fn set_enable(&mut self, value: bool);
  fn set_duty(&mut self, value: u8);
  fn set_length_counter_halt(&mut self, value: bool);
  fn set_constant_volume(&mut self, value: bool);
  fn set_volume(&mut self, value: u8);
  fn set_timer_low(&mut self, value: u8);
  fn set_timer_high(&mut self, value: u8);
  fn reset_sequencer(&mut self);
  fn set_sweep(&mut self, enabled: bool, divider_period: u8, negate: bool, shift_count: u8);
}
struct Divider {
  period: u8,
}
impl Divider {
  fn clock(&mut self, decay_level_counter: &mut Option<&mut DecayLevelCounter>, loop_flag: bool, period: u8) {
    if self.period == 0 {
      self.load(period);
      if decay_level_counter.is_some() {
        decay_level_counter.as_mut().unwrap().clock(loop_flag);
      }
    } else {
      self.period -= 1;
    }
  }
  fn load(&mut self, period: u8) {
    self.period = period;
  }
}
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
struct Envelope {
  start: bool,
  divider: Divider,
  decay_level_counter: DecayLevelCounter,
  output: u8,
}
impl Envelope {
  fn clock(&mut self, loop_flag: bool, volume: u8, constant_volume: bool) {
    if self.start {
      self.start = false;
      self.decay_level_counter.reset();
      self.divider.load(volume);
    } else {
      self
        .divider
        .clock(&mut Some(&mut self.decay_level_counter), loop_flag, volume);
    }
    self.output = if constant_volume {
      volume
    } else {
      //println!("{}", self.decay_level_counter.count);
      self.decay_level_counter.count
    }
  }
}
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

    let result = if self.enabled_flag && self.shift_count != 0 && self.divider.period == 0 && self.mute == false {
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
struct Pulse {
  duty: u8,
  length_counter_halt: bool,
  constant_volume: bool,
  volume: u8,
  timer: u16,
  current_time: u16,
  current_sequencer_position: u8,
  enable: bool,
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
    self
      .envelope
      .clock(self.length_counter_halt, self.volume, self.constant_volume);
  }
  fn clock_length_counter(&mut self) {
    self.length_counter.clock(self.enable, self.length_counter_halt);
  }
  fn clock_sweep(&mut self) {
    self.timer = self.sweep.clock(self.timer, self.is_pulse_1);
  }
}
impl Default for Pulse {
  fn default() -> Self {
    Pulse {
      duty: 0,
      length_counter_halt: false,
      constant_volume: false,
      volume: 0,
      timer: 0,
      current_time: 0,
      current_sequencer_position: 0,
      enable: false,
      envelope: Envelope {
        start: false,
        divider: Divider { period: 0 },
        decay_level_counter: DecayLevelCounter { count: 0 },
        output: 0,
      },
      sweep: Sweep {
        divider: Divider { period: 0 },
        reload_flag: false,
        divider_period: 0,
        enabled_flag: false,
        negate_flag: false,
        shift_count: 0,
        mute: false,
      },
      length_counter: LengthCounter { length: 0 },
      last_output: 0,
      is_pulse_1: false,
    }
  }
}
impl Channel for Pulse {
  fn set_length(&mut self, length: u8) {
    if self.enable {
      self.length_counter.set_length(length);
    }
  }
  fn clock(&mut self) {
    let envelope = self.envelope.output;
    let length = self.length_counter.length;
    if self.current_time == 0 {
      self.current_time = self.timer;
      self.current_sequencer_position += 1;
      self.current_sequencer_position %= 8;
    } else {
      self.current_time -= 1;
    };
    let sequencer = Pulse::OUTPUT[self.duty as usize][self.current_sequencer_position as usize];

    self.last_output = if self.sweep.mute || sequencer == false || length == 0 || self.timer < 8 {
      0
    } else {
      envelope
    };
  }
  fn get_value(&self) -> u8 {
    self.last_output
  }
  fn set_enable(&mut self, value: bool) {
    self.enable = value;
    if value == false {
      self.length_counter.length = 0;
    }
  }
  fn set_duty(&mut self, value: u8) {
    self.duty = value;
  }
  fn set_length_counter_halt(&mut self, value: bool) {
    self.length_counter_halt = value;
  }
  fn set_constant_volume(&mut self, value: bool) {
    self.constant_volume = value;
  }
  fn set_volume(&mut self, value: u8) {
    self.volume = value;
  }
  fn set_timer_low(&mut self, value: u8) {
    self.timer &= 0xFF00;
    self.timer |= value as u16;
  }
  fn set_timer_high(&mut self, value: u8) {
    self.timer &= 0x00FF;
    self.timer |= (value as u16) << 8;
  }
  fn reset_sequencer(&mut self) {
    self.current_sequencer_position = 0;
  }
  fn set_sweep(&mut self, enabled: bool, divider_period: u8, negate: bool, shift_count: u8) {
    self.sweep.setup(enabled, divider_period, negate, shift_count);
  }
}
struct FrameCounter {
  mode: bool,              //true: 5-step sequence, false: 4-step sequence
  interrupt_inhibit: bool, //割り込み禁止フラグ
  count: u16,
  interrupt_flag: bool,
}
impl FrameCounter {
  fn clock(&mut self, pulse1: &mut Pulse, pulse2: &mut Pulse) {
    match self.count {
      3728 => {
        //エンベローブ, 三角波線形カウンタ
        pulse1.clock_envelope();
        pulse2.clock_envelope();
      }
      7456 => {
        //エンベローブ, 三角波線形カウンタ
        //長さカウンタ, スイープユニット
        pulse1.clock_envelope();
        pulse2.clock_envelope();
        pulse1.clock_length_counter();
        pulse2.clock_length_counter();
        pulse1.clock_sweep();
        pulse2.clock_sweep();
      }
      11185 => {
        //エンベローブ, 三角波線形カウンタ
        pulse1.clock_envelope();
        pulse2.clock_envelope();
      }
      14914 => {
        if self.mode == false {
          //エンベローブ, 三角波線形カウンタ
          //長さカウンタ, スイープユニット
          //割り込み
          pulse1.clock_envelope();
          pulse2.clock_envelope();
          pulse1.clock_length_counter();
          pulse2.clock_length_counter();
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
        pulse1.clock_length_counter();
        pulse2.clock_length_counter();
        pulse1.clock_sweep();
        pulse2.clock_sweep();
        self.count = 0;
        return;
      }
      _ => {}
    }
    self.count += 1;
  }
  fn set(&mut self, mode: bool, interrupt_inhibit: bool, pulse1: &mut Pulse, pulse2: &mut Pulse) {
    self.mode = mode;
    self.interrupt_inhibit = interrupt_inhibit;
    self.count = 0;
    if mode {
      //エンベローブ, 三角波線形カウンタ
      //長さカウンタ, スイープユニット
      pulse1.clock_envelope();
      pulse2.clock_envelope();
      pulse1.clock_length_counter();
      pulse2.clock_length_counter();
      pulse1.clock_sweep();
      pulse2.clock_sweep();
    }
  }
}
struct LengthCounter {
  length: u8,
}
impl LengthCounter {
  const LENGTH_TABLE: [u8; 0x20] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, //00-0F
    12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30, //10-1F
  ];
  fn clock(&mut self, enable: bool, length_counter_halt: bool) {
    if enable {
      if length_counter_halt == false && self.length != 0 {
        self.length -= 1;
      }
    } else {
      self.length = 0;
    }
  }
  fn set_length(&mut self, length_counter_load: u8) {
    self.length = LengthCounter::LENGTH_TABLE[length_counter_load as usize];
  }
}
struct Mixer {
  pulse_table: [f32; 31],
}
impl Mixer {
  fn new() -> Self {
    Mixer {
      pulse_table: (0..=30)
        .map(|i| {
          if i == 0 {
            0.0
          } else {
            95.52 / (8128.0 / (i as f32) + 100.0)
          }
        })
        .collect::<Vec<f32>>()
        .as_slice()
        .try_into()
        .unwrap(),
    }
  }
  fn mix(&self, pulse1: u8, pulse2: u8) -> f32 {
    self.pulse_table[(pulse1 + pulse2) as usize]
  }
}
pub struct Apu {
  pulse1: Pulse,
  pulse2: Pulse,
  frame_counter: FrameCounter,
  mixer: Mixer,
  clock_count: u8,
}
impl Apu {
  pub fn new() -> Self {
    Apu {
      pulse1: Pulse { is_pulse_1: true, ..Default::default() },
      pulse2: Pulse { ..Default::default() },
      frame_counter: FrameCounter { mode: false, interrupt_inhibit: false, count: 0, interrupt_flag: false },
      mixer: Mixer::new(),
      clock_count: 0,
    }
  }
  pub fn clock(&mut self) -> f32 {
    if self.clock_count % 2 == 0 {
      self.frame_counter.clock(&mut self.pulse1, &mut self.pulse2);
      self.pulse1.clock();
      self.pulse2.clock();
    }
    let pulse1 = self.pulse1.get_value();
    let pulse2 = self.pulse2.get_value();

    self.clock_count += 1;
    self.clock_count %= 2;
    self.mixer.mix(pulse1, pulse2)
  }
  pub fn write(&mut self, addr: u8, value: u8) {
    let mut target = match addr {
      0x00..=0x03 => Some(&mut self.pulse1),
      0x04..=0x07 => Some(&mut self.pulse2),
      0x08..=0x0B => None, //TODO: Triangle
      0x0C..=0x0F => None, //TODO: Noise
      0x10..=0x13 => None, //TODO: DMC
      _ => None,
    };

    match addr {
      0x00 | 0x04 => {
        let target = target.as_mut().unwrap();
        target.set_duty((value & 0b1100_0000) >> 6);
        target
          .set_length_counter_halt((value & 0b0010_0000) == 0b0010_0000);
        target
          .set_constant_volume((value & 0b0001_0000) == 0b0001_0000);
        target.set_volume(value & 0x0F);
      }
      0x01 | 0x05 => {
        let target = target.as_mut().unwrap();
        let enabled = (value & 0b1000_0000) == 0b1000_0000;
        let period = (value & 0b0111_0000) >> 4;
        let negate = (value & 0b0000_1000) == 0b0000_1000;
        let shift_count = value & 0b0000_0111;
        target.set_sweep(enabled, period, negate, shift_count);
      }
      0x02 | 0x06 /* | 0x0A //TODO: Triangle*/ => {
        //Timer low
        let target = target.as_mut().unwrap();
        target.set_timer_low(value);
      }
      0x03 | 0x07 /*| 0x0B | 0x0F*/ => {
        let target = target.as_mut().unwrap();
        target.set_length((value & 0xF8) >> 3);
        if addr != 0x0F {//Noiseにはタイマーがない
          target.set_timer_high(value & 0b111);
        }
        target.envelope.start = true;
        target.reset_sequencer();
      }
      0x15 => {
        //TODO: DMC
        //TODO: Noise
        //TODO: Triangle
        self.pulse2.set_enable((value & 0b10) == 0b10);
        self.pulse1.set_enable((value & 0b1) == 0b1);
      }
      0x17 => {
        self.frame_counter.set((value & 0x80) == 0x80, (value & 0x40) == 0x40, &mut self.pulse1, &mut self.pulse2);
      }
      _ =>{}//TODO
    }
    // println!(
    //   "4000: {:02b}{}{}{:04b}",
    //   self.pulse1.duty,
    //   if self.pulse1.length_counter_halt { 1 } else { 0 },
    //   if self.pulse1.constant_volume { 1 } else { 0 },
    //   self.pulse1.volume
    // );
    // println!(
    //   "4001: {}{:03b}{}{:03b}",
    //   if self.pulse1.sweep.enabled_flag { 1 } else { 0 },
    //   self.pulse1.sweep.divider_period,
    //   if self.pulse1.sweep.negate_flag { 1 } else { 0 },
    //   self.pulse1.sweep.shift_count
    // );
    // println!("4002: {:08b}", self.pulse1.timer & 0xFF);
    // println!(
    //   "4003: {:05b}{:03b}",
    //   self.pulse1.length_counter.length & 0x1F,
    //   (self.pulse1.timer & 0x700) >> 8
    // );
    // println!("----------------");
  }
  pub fn read(&mut self, addr: u8) -> u8 {
    match addr {
      0x15 => {
        let mut value: u8 = 0;
        //TODO
        value |= if self.frame_counter.interrupt_flag { 1 } else { 0 } << 6;
        self.frame_counter.interrupt_flag = false;
        value |= if self.pulse2.length_counter.length > 0 { 1 } else { 0 } << 1;
        value |= if self.pulse1.length_counter.length > 0 { 1 } else { 0 };
        value
      }
      _ => panic!(),
    }
  }
  pub fn check_irq(&self) -> bool {
    self.frame_counter.interrupt_flag
  }
  pub fn set_irq(&mut self, value: bool) {
    self.frame_counter.interrupt_flag = value;
  }
}

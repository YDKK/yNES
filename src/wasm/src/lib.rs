use wasm_bindgen::prelude::*;
use y_nes::nes::*;
use y_nes::util::NES_PALETTE;
extern crate console_error_panic_hook;

#[wasm_bindgen]
pub struct WasmNes {
    instance: Nes,
    /// RGBA pixel buffer (256×240×4 = 245760 bytes)
    pixel_buffer: Vec<u8>,
}

#[wasm_bindgen]
#[derive(Default)]
pub struct WasmPadInput {
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl From<&WasmPadInput> for PadInput {
    fn from(input: &WasmPadInput) -> Self {
        PadInput {
            a: input.a,
            b: input.b,
            select: input.select,
            start: input.start,
            up: input.up,
            down: input.down,
            left: input.left,
            right: input.right,
        }
    }
}

#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").into()
}

#[wasm_bindgen]
pub fn get_core_version() -> String {
    Nes::get_version()
}

#[wasm_bindgen]
pub fn pad_new() -> WasmPadInput {
    WasmPadInput { ..Default::default() }
}

#[wasm_bindgen]
pub fn nes_new(rom: Vec<u8>) -> WasmNes {
    console_error_panic_hook::set_once();
    WasmNes { instance: Nes::new(rom.as_slice()).unwrap(), pixel_buffer: vec![0u8; 256 * 240 * 4] }
}

/// Execute one full frame and return audio samples (f32 array at 44100 Hz).
/// The screen can then be retrieved via nes_get_screen_rgba.
#[wasm_bindgen]
pub fn nes_clock_frame(nes: &mut WasmNes, pad1: &WasmPadInput) -> Vec<f32> {
    let native_pad = PadInputs { pad1: PadInput::from(pad1), pad2: Default::default() };
    nes.instance.clock_frame(&native_pad).to_vec()
}

/// Get the current screen as RGBA pixels (256×240×4 bytes).
/// Returns a pointer and length suitable for use with ImageData.
#[wasm_bindgen]
pub fn nes_get_screen_rgba(nes: &mut WasmNes) -> Vec<u8> {
    let screen = nes.instance.get_screen();
    let buf = &mut nes.pixel_buffer;
    for (i, &color_index) in screen.iter().enumerate() {
        let color = NES_PALETTE[color_index as usize & 0x3F];
        let offset = i * 4;
        buf[offset] = color[0]; // R
        buf[offset + 1] = color[1]; // G
        buf[offset + 2] = color[2]; // B
        buf[offset + 3] = 255; // A
    }
    buf.clone()
}

/// Legacy per-clock API
#[wasm_bindgen]
pub struct WasmClockResult {
    pub end_frame: bool,
    pub apu_out: Option<f32>,
}

#[wasm_bindgen]
pub fn nes_clock(nes: &mut WasmNes, pad1: &WasmPadInput) -> WasmClockResult {
    let native_pad = PadInputs { pad1: PadInput::from(pad1), pad2: Default::default() };
    let result = nes.instance.clock(&native_pad);
    WasmClockResult { end_frame: result.0, apu_out: result.1 }
}

/// Legacy screen API (returns color index array)
#[wasm_bindgen]
pub fn nes_get_screen(nes: &mut WasmNes) -> Vec<u8> {
    nes.instance.get_screen().to_vec()
}

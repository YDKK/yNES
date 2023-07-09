use wasm_bindgen::prelude::*;
use y_nes::nes::*;
extern crate console_error_panic_hook;

#[wasm_bindgen]
pub struct WasmNes {
    instance: Nes,
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
pub struct WasmClockResult {
    pub end_frame: bool,
    pub apu_out: Option<f32>,
}

#[wasm_bindgen]
pub fn nes_new(rom: Vec<u8>) -> WasmNes {
    console_error_panic_hook::set_once();
    WasmNes { instance: Nes::new(rom.as_slice()).unwrap() }
}

#[wasm_bindgen]
pub fn nes_clock(nes: &mut WasmNes, pad1: &WasmPadInput) -> WasmClockResult {
    let native_pad = PadInputs { pad1: PadInput::from(pad1), pad2: Default::default() };
    let result = Nes::clock(&mut nes.instance, &native_pad);
    WasmClockResult { end_frame: result.0, apu_out: result.1 }
}

#[wasm_bindgen]
pub fn nes_get_screen(nes: &mut WasmNes) -> Vec<u8> {
    Nes::get_screen(&nes.instance).to_vec()
}

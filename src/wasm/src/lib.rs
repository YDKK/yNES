use wasm_bindgen::prelude::*;
use y_nes::nes::*;
extern crate console_error_panic_hook;

#[wasm_bindgen]
pub struct WasmNes {
    instance: Nes,
}

#[wasm_bindgen]
pub struct WasmPadInputs {
    data: PadInputs,
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
pub fn nes_clock(nes: &mut WasmNes) -> WasmClockResult {
    let native_pad = PadInputs { pad1: Default::default(), pad2: Default::default() };
    let result = Nes::clock(&mut nes.instance, &native_pad);
    WasmClockResult { end_frame: result.0, apu_out: result.1 }
}

#[wasm_bindgen]
pub fn nes_get_screen(nes: &mut WasmNes) -> Vec<u8> {
    Nes::get_screen(&nes.instance).to_vec()
}

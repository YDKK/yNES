use super::apu::*;
use super::nes::*;
use super::ppu::*;
use super::rom::*;
use super::util::*;

/// Creates a minimal valid iNES ROM for testing
fn make_test_rom(prg: &[u8], chr: &[u8], vertical_mirroring: bool) -> Vec<u8> {
    let prg_banks = if prg.len() <= 0x4000 {
        1
    } else {
        (prg.len() + 0x3FFF) / 0x4000
    };
    let chr_banks = if chr.is_empty() {
        0
    } else {
        (chr.len() + 0x1FFF) / 0x2000
    };
    let flags6 = if vertical_mirroring { 1u8 } else { 0u8 };
    let mut rom_data = vec![
        0x4E,
        0x45,
        0x53,
        0x1A, // "NES\x1A"
        prg_banks as u8,
        chr_banks as u8,
        flags6,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    // Pad PRG
    let mut prg_padded = vec![0u8; prg_banks * 0x4000];
    prg_padded[..prg.len()].copy_from_slice(prg);
    rom_data.extend_from_slice(&prg_padded);
    // Pad CHR
    if chr_banks > 0 {
        let mut chr_padded = vec![0u8; chr_banks * 0x2000];
        chr_padded[..chr.len()].copy_from_slice(chr);
        rom_data.extend_from_slice(&chr_padded);
    }
    rom_data
}

#[test]
fn test_rom_parse_valid() {
    let rom_data = make_test_rom(&[0u8; 0x4000], &[0u8; 0x2000], true);
    let rom = Rom::load(&rom_data);
    assert!(rom.is_ok());
    let rom = rom.unwrap();
    assert_eq!(rom.mirroring, MirroringMode::Vertical);
    assert_eq!(rom.get_prog().len(), 0x4000);
    assert_eq!(rom.get_chr().len(), 0x2000);
}

#[test]
fn test_rom_parse_horizontal_mirroring() {
    let rom_data = make_test_rom(&[0u8; 0x4000], &[0u8; 0x2000], false);
    let rom = Rom::load(&rom_data).unwrap();
    assert_eq!(rom.mirroring, MirroringMode::Horizontal);
}

#[test]
fn test_rom_chr_ram_flag() {
    // 0 CHR banks → has_chr_ram should be true
    let rom_data = make_test_rom(&[0u8; 0x4000], &[], false);
    let rom = Rom::load(&rom_data).unwrap();
    assert!(rom.has_chr_ram());
    // CHR ROM is empty for CHR RAM ROMs (PPU owns the RAM now)
    assert_eq!(rom.get_chr().len(), 0);
}

#[test]
fn test_rom_invalid() {
    let result = Rom::load(&[0, 1, 2, 3]);
    assert!(result.is_err());
}

#[test]
fn test_nes_palette_length() {
    // NES_PALETTE should have 64 colors
    assert_eq!(NES_PALETTE.len(), 64);
}

#[test]
fn test_nes_palette_first_color() {
    // First color in NES palette is (84, 84, 84)
    assert_eq!(NES_PALETTE[0], [84, 84, 84]);
}

#[test]
fn test_get_addr() {
    assert_eq!(get_addr(0x12, 0x34), 0x1234);
    assert_eq!(get_addr(0xFF, 0x00), 0xFF00);
    assert_eq!(get_addr(0x00, 0xFF), 0x00FF);
}

#[test]
fn test_ppu_new_horizontal() {
    let ppu = Ppu::new(MirroringMode::Horizontal, false);
    let screen = ppu.get_screen();
    // Initially all pixels should be 0
    assert!(screen.iter().all(|&p| p == 0));
}

#[test]
fn test_ppu_new_vertical() {
    let ppu = Ppu::new(MirroringMode::Vertical, false);
    let screen = ppu.get_screen();
    assert!(screen.iter().all(|&p| p == 0));
}

#[test]
fn test_apu_new() {
    let apu = Apu::new();
    // Initial state: reading $4015 should return 0
    // (channels disabled by default)
    let _ = apu; // Just check it constructs without panic
}

#[test]
fn test_apu_clock_produces_output() {
    let mut apu = Apu::new();
    let prg = vec![0u8; 0x4000];
    let output = apu.clock(&prg);
    // Output should be a finite f32
    assert!(output.is_finite());
}

#[test]
fn test_nes_clock_frame_sample_count() {
    // Create a minimal ROM with reset vector pointing to an infinite loop (JMP $8000)
    let mut prg = vec![0u8; 0x4000];
    // Put JMP $8000 at $8000
    prg[0] = 0x4C; // JMP
    prg[1] = 0x00;
    prg[2] = 0x80;
    // Set reset vector at $FFFC/$FFFD to $8000
    prg[0x3FFC] = 0x00;
    prg[0x3FFD] = 0x80;
    let rom_data = make_test_rom(&prg, &[0u8; 0x2000], false);
    let mut nes = Nes::new(&rom_data).unwrap();
    let pad = PadInputs { pad1: Default::default(), pad2: Default::default() };
    // ~29781 CPU cycles per frame → ~735 samples at 44100 Hz
    let samples = nes.clock_frame(&pad);
    assert!(
        samples.len() >= 700 && samples.len() <= 800,
        "Expected ~735 samples, got {}",
        samples.len()
    );
}

#[test]
fn test_nes_clock_frame_samples_are_finite() {
    let mut prg = vec![0u8; 0x4000];
    prg[0] = 0x4C;
    prg[1] = 0x00;
    prg[2] = 0x80; // JMP $8000
    prg[0x3FFC] = 0x00;
    prg[0x3FFD] = 0x80; // reset vector
    let rom_data = make_test_rom(&prg, &[0u8; 0x2000], false);
    let mut nes = Nes::new(&rom_data).unwrap();
    let pad = PadInputs { pad1: Default::default(), pad2: Default::default() };
    let samples = nes.clock_frame(&pad);
    for (i, &s) in samples.iter().enumerate() {
        assert!(s.is_finite(), "Sample {} is not finite: {}", i, s);
    }
}

#[test]
fn test_struct_sizes() {
    use super::cpu::*;
    println!("=== STRUCT SIZES ===");
    println!("Ppu:          {} bytes", std::mem::size_of::<Ppu>());
    println!("Nes:          {} bytes", std::mem::size_of::<Nes>());
    println!("Option<Nes>:  {} bytes", std::mem::size_of::<Option<Nes>>());
    println!("Cpu:          {} bytes", std::mem::size_of::<Cpu>());
    println!("Apu:          {} bytes", std::mem::size_of::<Apu>());
    println!("Rom:          {} bytes", std::mem::size_of::<Rom>());
}

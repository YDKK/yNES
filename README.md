# yNES

Rust製ファミコンエミュレータ

## Demo

### Windows

|nestest|SMB|
|-------|---|
|![image](https://github.com/YDKK/yNES/assets/3415240/7e789264-8499-453d-954f-4e521f26a9e7)|![capture](https://github.com/YDKK/yNES/assets/3415240/d28fa428-218c-4ac3-91b1-0795f3887855)|
||[YouTube (with audio)](https://www.youtube.com/watch?v=1uFGKheUacY)|

## Implementation

### Emulator Core

- CPU
  - [x] Official Opcodes
  - [ ] Unofficial Opcodes
- APU
  - [x] Pulse Channel (1, 2)
  - [x] Triangle Channel
  - [x] Noise Channel
  - [ ] DMC
- PPU
  - [x] nestestやSMBが正常に動作する程度
    - その他の細かい挙動は怪しい
  - Nametable Mirroring
    - [x] Horizontal
    - [x] Vertical
    - [ ] Single-Screen
    - [ ] 4-Screen
    - [ ] Other
- ROM
  - [x] iNES Format
 
### Frontend

- Windows
  - [x] 画面出力
  - [x] 音声出力
  - [x] Pad入力
  - [x] 実行速度変更
- Browser
  - [x] 画面出力
  - [x] 音声出力
  - [x] Pad入力

## Build

### Emulator Core

```
cd src/common
cargo build --release
```

### Frontend

#### Windows

```
cd src/win
cargo build --release
```

#### Browser

```
cd src/wasm
cargo install wasm-pack
wasm-pack build --target web --release
```

## License

MIT

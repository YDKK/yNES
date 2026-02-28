# yNES

Rust製ファミコンエミュレータ

## Demo

### Windows

|nestest|SMB|
|-------|---|
|![image](https://github.com/YDKK/yNES/assets/3415240/be6a7e97-f0fb-476a-a473-f4fd5f73d4ef)|![capture](https://github.com/YDKK/yNES/assets/3415240/47a0ca36-23f9-46cb-9012-76423169eb7f)|
||[YouTube (with audio)](https://www.youtube.com/watch?v=1uFGKheUacY)|

### Browser

https://ydkk.github.io/yNES/

## Implementation

### Emulator Core

- CPU
  - [x] Official Opcodes
  - [x] Unofficial Opcodes
- APU
  - [x] Pulse Channel (1, 2)
  - [x] Triangle Channel
  - [x] Noise Channel
  - [x] DMC
- PPU
  - [x] nestestやSMBが正常に動作する程度
    - その他の細かい挙動は怪しい
  - Nametable Mirroring
    - [x] Horizontal
    - [x] Vertical
    - [x] Single-Screen
    - [x] 4-Screen
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

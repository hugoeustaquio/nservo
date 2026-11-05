# Estratégia de Empacotamento Multiplataforma — NServo

O NServo é projetado para compilar e executar em:
- **Linux** (X11 / Wayland)
- **Windows** (x86_64 / AArch64)
- **macOS** (Apple Silicon / Intel)
- **Android** (AArch64 / x86_64)

Compartilhando mais de 95% do código central em Rust puro.

---

## 1. Linux

### 1.1 Binário Standalone
```bash
cargo build --release --target x86_64-unknown-linux-gnu
strip target/x86_64-unknown-linux-gnu/release/nservo
```

### 1.2 Flatpak
O manifesto Flatpak empacota as dependências de GPU e fontes:
```yaml
app-id: org.nservo.NServo
runtime: org.freedesktop.Platform
runtime-version: '23.08'
sdk: org.freedesktop.Sdk
sdk-extensions:
  - org.freedesktop.Sdk.Extension.rust-stable
command: nservo
finish-args:
  - --share=ipc
  - --socket=fallback-x11
  - --socket=wayland
  - --share=network
  - --device=dri
```

### 1.3 Arch Linux (PKGBUILD)
```bash
pkgname=nservo-bin
pkgver=0.1.0
arch=('x86_64')
depends=('lua54' 'sqlite')
```

---

## 2. Windows

### 2.1 Compilação com MSVC
```bash
cargo build --release --target x86_64-pc-windows-msvc
```

### 2.2 Instalador MSI com WiX Toolset
Utilizando o plugin `cargo-wix`:
```bash
cargo install cargo-wix
cargo wix init
cargo wix
```
Gera um instalador `.msi` moderno com integração ao Menu Iniciar e registro no PATH.

---

## 3. macOS

### 3.1 Compilação Universal (Apple Silicon + Intel)
```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin

cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

lipo -create \
  target/aarch64-apple-darwin/release/nservo \
  target/x86_64-apple-darwin/release/nservo \
  -output nservo-universal
```

### 3.2 Bundle `.app` e Imagem DMG
Estrutura:
```text
NServo.app/
└── Contents/
    ├── Info.plist
    ├── MacOS/
    │   └── nservo
    └── Resources/
        └── nservo.icns
```
Criação do instalador `.dmg`:
```bash
hdiutil create -volname "NServo" -srcfolder NServo.app -ov -format UDZO NServo-0.1.0.dmg
```

---

## 4. Android

### 4.1 Requisitos
- Android NDK (r25c+)
- `cargo-ndk`
```bash
cargo install cargo-ndk
rustup target add aarch64-linux-android armv7-linux-androideabi
```

### 4.2 Compilação da Biblioteca JNI
```bash
cargo ndk -t aarch64-linux-android -o android/app/src/main/jniLibs/ build --release
```

### 4.3 Arquitetura no Android
- A interface é acoplada a uma `NativeActivity` ou SurfaceView.
- Teclados físicos conectados via Bluetooth ou USB funcionam diretamente mapeados pelo módulo `src/platform/android.rs`.
- Caminho de configuração resolvido para `/data/data/org.nservo.browser/files/`.
- Todos os plugins Lua, regras de adblock e Greasemonkey executam de forma idêntica ao desktop.

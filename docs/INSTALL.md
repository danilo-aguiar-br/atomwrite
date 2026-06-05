# Installation Guide

- Complete instructions for installing atomwrite on Linux, macOS, and Windows
- Sections ordered by platform, with prerequisites and troubleshooting


## Linux

### Quick Install (Ubuntu/Debian)

```bash
# Install Rust 1.85 or later via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Install atomwrite from crates.io
cargo install atomwrite --locked

# Verify
atomwrite --version
```

### Quick Install (Fedora/RHEL)

```bash
# Install build tools
sudo dnf install rust cargo gcc

# Install atomwrite
cargo install atomwrite --locked
```

### Quick Install (Arch)

```bash
sudo pacman -S rust
cargo install atomwrite --locked
```


## macOS

### Quick Install

```bash
# Install Rust 1.85+ via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Install atomwrite
cargo install atomwrite --locked

# Allow in Gatekeeper if prompted
xattr -d com.apple.quarantine $(which atomwrite) 2>/dev/null || true
```


## Windows 10 / Windows 11

### Prerequisites

1. **Rust 1.85 or later** — install via [rustup.rs](https://rustup.rs)
2. **Visual Studio Build Tools 2019 or later** with the "Desktop development with C++" workload — required for linking. Download from [visualstudio.microsoft.com](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
3. **Windows Terminal** or **PowerShell 7+** (Windows Terminal recommended for UTF-8 rendering)
4. **Git for Windows** if installing from source

### Quick Install (from crates.io)

```powershell
# Open PowerShell 7+ or Windows Terminal
rustup default stable
rustup target add x86_64-pc-windows-msvc

# Install atomwrite
cargo install atomwrite --locked

# Verify (expect NDJSON output)
atomwrite --version
```

### Quick Install (from source)

```powershell
git clone https://github.com/daniloaguiarbr/atomwrite.git
cd atomwrite
cargo install --path . --locked
```

### Troubleshooting

#### "error: linker 'link.exe' not found"

Install Visual Studio Build Tools with the C++ workload. The installer is at
<https://visualstudio.microsoft.com/visual-cpp-build-tools/>. After installation,
restart your terminal so the updated PATH is picked up.

#### "error[E0433]: failed to resolve: use of undeclared type `AtomwriteError`"

This was a bug in v0.1.3 (GAP 14) fixed in v0.1.4. Make sure you install
v0.1.4 or later:

```powershell
cargo install atomwrite --locked --version "^0.1.4"
```

If you cannot upgrade, compile from source with the fix applied.

#### "error: linking with `link.exe` failed: exit code: 1125"

Antivirus (Windows Defender, McAfee, etc.) is blocking the linker. Add an
exclusion for `%USERPROFILE%\.cargo` and `%USERPROFILE%\.rustup`, then retry.

#### Mojibake in console output

Windows legacy console (`cmd.exe`) does not support UTF-8 by default. Use
**Windows Terminal** (which auto-enables UTF-8) or run:

```powershell
chcp 65001
```

before invoking atomwrite. The `init_console` function in atomwrite also
enables UTF-8 and ANSI escape sequences on Windows console handles when run
from a properly configured terminal.

#### "operation not permitted" during atomic write

The atomic write pipeline retries up to 3 times on `PermissionDenied`. If
a file is held open by another process (e.g., editor without proper file
release, antivirus scanner, indexed search), the retry may exhaust. Close
the file in the locking application and retry.


## Installing Specific Versions

```bash
# Latest
cargo install atomwrite --locked

# Specific version
cargo install atomwrite --locked --version 0.1.4

# Force reinstall
cargo install atomwrite --locked --force
```

The `--locked` flag ensures `Cargo.lock` is honored, guaranteeing a reproducible
build that matches what the maintainers tested.


## Verifying Installation

```bash
atomwrite --version
# Expect NDJSON output with version, build date, and platform info
```

If the command is not found, ensure `~/.cargo/bin` (Linux/macOS) or
`%USERPROFILE%\.cargo\bin` (Windows) is in your `PATH`.


## Building from Source (all platforms)

```bash
git clone https://github.com/daniloaguiarbr/atomwrite.git
cd atomwrite
cargo build --release
./target/release/atomwrite --version
```

The release binary is at `target/release/atomwrite` (or `atomwrite.exe` on
Windows).


## Cross-Compile Validation (for contributors)

The project includes a cross-compile gate to detect Windows-only compilation
errors before release. Run:

```bash
# Install Windows targets
rustup target add x86_64-pc-windows-gnu
rustup target add x86_64-pc-windows-msvc

# On Linux, install mingw-w64 for the GNU target
sudo dnf install mingw64-gcc mingw64-gcc-c++  # Fedora
sudo apt install mingw-w64                     # Ubuntu

# Run the cross-compile gate
cargo test --test cross_compile_check -- --ignored
```

This is required for any release that touches `#[cfg(windows)]` code paths,
per the GAP 14 fix in v0.1.4.

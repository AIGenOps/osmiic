# OSMIIC Publishing & Distribution Guide

This guide details compiling, packaging, and distributing OSMIIC across Linux distributions, Windows package managers, open-source repositories, and software discovery platforms.

---

## 1. Compiling Production Release Packages

OSMIIC uses Tauri's built-in bundler to create native installers and standalone binaries for multiple desktop operating systems.

### Step 1: Navigate to the Tauri Core Directory
```bash
cd src-tauri
```

### Step 2: Build the Bundles
```bash
cargo tauri build
```

### Release Binary & Installer Locations
- **Linux AppImage**: `src-tauri/target/release/bundle/appimage/OSMIIC.AppImage`
- **Linux Debian (.deb)**: `src-tauri/target/release/bundle/deb/osmiic_1.0.0_amd64.deb`
- **Windows Setup (.exe)**: `src-tauri/target/release/bundle/nsis/OSMIIC_1.0.0_x64-setup.exe`
- **Windows Installer (.msi)**: `src-tauri/target/release/bundle/msi/OSMIIC_1.0.0_x64_en-US.msi`

---

## 2. Linux Distribution Platforms

### A. Flathub (Universal Linux App Store)
Flathub is the standard way to reach users on Fedora, Arch, Ubuntu, Debian, Pop!_OS, and SteamOS.

1. Fork the [Flathub repository](https://github.com/flathub/flathub).
2. Create a flatpak manifest `com.osmiic.downloader.json` declaring dependencies (`ffmpeg`, `yt-dlp`).
3. Submit a Pull Request.
4. Once merged, users can install OSMIIC with:
   ```bash
   flatpak install flathub com.osmiic.downloader
   ```

### B. Snapcraft / Canonical Snap Store
Default store on Ubuntu with millions of active desktop users.

1. Register an account at [snapcraft.io](https://snapcraft.io/).
2. Install snapcraft CLI:
   ```bash
   sudo snap install snapcraft --classic
   ```
3. Create a `snapcraft.yaml` pointing to the built binary.
4. Build and upload:
   ```bash
   snapcraft upload --release=stable osmiic_1.0.0_amd64.snap
   ```

### C. Arch User Repository (AUR)
Direct integration for Arch Linux, Manjaro, and EndeavourOS users.

1. Create an AUR account at [aur.archlinux.org](https://aur.archlinux.org/).
2. Set up a `PKGBUILD` for `osmiic-bin` referencing the AppImage or `.deb` from GitHub releases.
3. Push to AUR git:
   ```bash
   git clone ssh://aur@aur.archlinux.org/osmiic-bin.git
   ```
4. Users install with:
   ```bash
   yay -S osmiic-bin
   ```

### D. Ubuntu / Debian PPA (Personal Package Archive)
1. Register on [Launchpad](https://launchpad.net/).
2. Upload Debian source packages for automatic building.
3. Users install via APT:
   ```bash
   sudo add-apt-repository ppa:aigenops/osmiic
   sudo apt update && sudo apt install osmiic
   ```

---

## 3. Windows Distribution Platforms

### A. Windows Package Manager (`winget`)
Native CLI package manager preinstalled on Windows 10 & 11.

1. Install `winget-create`:
   ```powershell
   winget install Microsoft.WingetCreate
   ```
2. Submit your GitHub release `.exe` or `.msi`:
   ```powershell
   winget-create submit https://github.com/AIGenOps/osmiic/releases/download/v1.0.0/OSMIIC_1.0.0_x64-setup.exe
   ```
3. Once approved, users install with:
   ```powershell
   winget install OSMIIC
   ```

### B. Chocolatey
Popular package manager for Windows power users and DevOps teams.

1. Create a `.nuspec` package manifest.
2. Push package:
   ```powershell
   choco push OSMIIC.1.0.0.nupkg --api-key YOUR_KEY
   ```

---

## 4. GitHub Releases

1. Create a release tag `v1.0.0` on GitHub.
2. Draft release notes detailing key features.
3. Attach compiled installer files:
   - `OSMIIC.AppImage`
   - `osmiic_1.0.0_amd64.deb`
   - `OSMIIC_1.0.0_x64-setup.exe`
   - `OSMIIC_1.0.0_x64_en-US.msi`
4. Publish release.

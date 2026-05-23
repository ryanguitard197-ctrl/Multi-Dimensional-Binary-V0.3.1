#!/usr/bin/env bash
# ============================================================
#  MDB-OS — Bootable ISO Build Script
# ============================================================
#
#  Builds a bootable Linux ISO with MDB-OS pre-installed:
#    - MDBFS auto-mounted at boot
#    - MDB Desktop as the default Wayland session
#    - All MDB tools pre-installed
#
#  Requirements:
#    - Debian/Ubuntu build host (or Docker)
#    - debootstrap, squashfs-tools, xorriso, grub-pc-bin,
#      grub-efi-amd64-bin, mtools, dosfstools
#    - Rust toolchain (for building MDB crates)
#
#  Usage:
#    ./build.sh                    # Build everything
#    ./build.sh --skip-compile     # Reuse previously compiled binaries
#    ./build.sh --output my.iso    # Custom output path

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="${PROJECT_ROOT}/build"
ROOTFS="${BUILD_DIR}/rootfs"
ISO_DIR="${BUILD_DIR}/iso"
OUTPUT="${PROJECT_ROOT}/mdb-os.iso"
SKIP_COMPILE=false
ARCH="amd64"
CODENAME="bookworm"  # Debian stable

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --skip-compile)  SKIP_COMPILE=true; shift ;;
        --output)        OUTPUT="$2"; shift 2 ;;
        *)               echo "Unknown arg: $1"; exit 1 ;;
    esac
done

echo "╔══════════════════════════════════════╗"
echo "║     MDB-OS ISO Build System          ║"
echo "║     Building bootable image...       ║"
echo "╚══════════════════════════════════════╝"

# ============================================================
# Step 1: Compile MDB components
# ============================================================
if [ "$SKIP_COMPILE" = false ]; then
    echo ""
    echo "▸ Step 1: Compiling MDB components..."

    cd "$PROJECT_ROOT"

    # Build core engine and process model
    cargo build --release -p mdb-core -p mdb-process
    echo "  ✓ mdb-core"
    echo "  ✓ mdb-process"

    # Build MDBFS (requires libfuse3-dev)
    cargo build --release -p mdbfs
    echo "  ✓ mdbfs"

    # Build desktop compositor (requires smithay deps)
    cargo build --release -p mdb-desktop
    echo "  ✓ mdb-desktop"
else
    echo "▸ Step 1: Skipping compilation (--skip-compile)"
fi

# ============================================================
# Step 2: Create root filesystem
# ============================================================
echo ""
echo "▸ Step 2: Creating root filesystem..."

sudo rm -rf "$ROOTFS" "$ISO_DIR"
mkdir -p "$ROOTFS" "$ISO_DIR"

# Bootstrap minimal Debian
sudo debootstrap --arch="$ARCH" --variant=minbase \
    "$CODENAME" "$ROOTFS" http://deb.debian.org/debian

# Install essential packages
sudo chroot "$ROOTFS" apt-get update
sudo chroot "$ROOTFS" apt-get install -y --no-install-recommends \
    linux-image-amd64 \
    systemd systemd-sysv \
    dbus dbus-user-session \
    fuse3 libfuse3-3 \
    wayland-protocols libwayland-client0 libwayland-server0 \
    libxkbcommon0 libinput10 libgbm1 libdrm2 mesa-utils \
    libegl1 libgles2 \
    foot fuzzel thunar \
    swaylock \
    fontconfig fonts-noto-core \
    iproute2 iputils-ping \
    sudo bash-completion \
    ca-certificates

# ============================================================
# Step 3: Install MDB components
# ============================================================
echo ""
echo "▸ Step 3: Installing MDB components..."

# Copy compiled binaries
sudo install -m755 "${PROJECT_ROOT}/target/release/mdbfs" \
    "${ROOTFS}/usr/local/bin/mdbfs"
sudo install -m755 "${PROJECT_ROOT}/target/release/mdb-desktop" \
    "${ROOTFS}/usr/local/bin/mdb-desktop"

# Install systemd services
sudo install -m644 "${SCRIPT_DIR}/systemd/mdbfs.service" \
    "${ROOTFS}/etc/systemd/system/mdbfs.service"
sudo install -m644 "${SCRIPT_DIR}/systemd/mdb-desktop.service" \
    "${ROOTFS}/etc/systemd/system/mdb-desktop.service"

# Enable services
sudo chroot "$ROOTFS" systemctl enable mdbfs.service
sudo chroot "$ROOTFS" systemctl enable mdb-desktop.service
sudo chroot "$ROOTFS" systemctl set-default graphical.target

# ============================================================
# Step 4: Configure the OS
# ============================================================
echo ""
echo "▸ Step 4: Configuring MDB-OS..."

# Create default user
sudo chroot "$ROOTFS" useradd -m -s /bin/bash -G sudo,video,audio,input mdb
echo "mdb:mdb" | sudo chroot "$ROOTFS" chpasswd

# Copy skeleton config
sudo cp -r "${SCRIPT_DIR}/skel/." "${ROOTFS}/home/mdb/"
sudo chroot "$ROOTFS" chown -R mdb:mdb /home/mdb

# Create MDBFS directories
sudo mkdir -p "${ROOTFS}/var/lib/mdbfs"
sudo mkdir -p "${ROOTFS}/home/mdb/mdb"

# Set hostname
echo "mdb-os" | sudo tee "${ROOTFS}/etc/hostname" > /dev/null
echo "127.0.1.1 mdb-os" | sudo tee -a "${ROOTFS}/etc/hosts" > /dev/null

# OS release info
cat << 'EOF' | sudo tee "${ROOTFS}/etc/os-release" > /dev/null
NAME="MDB-OS"
VERSION="0.1.0"
ID=mdb-os
ID_LIKE=debian
PRETTY_NAME="MDB-OS 0.1.0 (Dimensional)"
VERSION_CODENAME=dimensional
HOME_URL="https://github.com/ryanguitard197-ctrl/MDB-OS"
BUG_REPORT_URL="https://github.com/ryanguitard197-ctrl/MDB-OS/issues"
EOF

# Auto-login for the default user
sudo mkdir -p "${ROOTFS}/etc/systemd/system/getty@tty1.service.d"
cat << 'EOF' | sudo tee "${ROOTFS}/etc/systemd/system/getty@tty1.service.d/autologin.conf" > /dev/null
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin mdb --noclear %I $TERM
EOF

# MOTD
cat << 'EOF' | sudo tee "${ROOTFS}/etc/motd" > /dev/null

  ╔══════════════════════════════════════════════╗
  ║              Welcome to MDB-OS               ║
  ║     Multidimensional Binary Operating System ║
  ║                                              ║
  ║   Your files live in dimensional space.      ║
  ║   ~/mdb is your MDB filesystem mount.       ║
  ║                                              ║
  ║   Super+Return  → Terminal                   ║
  ║   Super+D       → App Launcher               ║
  ║   Super+E       → File Manager               ║
  ╚══════════════════════════════════════════════╝

EOF

# ============================================================
# Step 5: Build ISO image
# ============================================================
echo ""
echo "▸ Step 5: Building ISO image..."

# Create squashfs of root filesystem
sudo mksquashfs "$ROOTFS" "${ISO_DIR}/filesystem.squashfs" \
    -comp xz -e boot

# Set up ISO directory structure
mkdir -p "${ISO_DIR}/boot/grub" "${ISO_DIR}/EFI/BOOT"

# Copy kernel and initrd
cp "${ROOTFS}"/boot/vmlinuz-* "${ISO_DIR}/boot/vmlinuz"
cp "${ROOTFS}"/boot/initrd.img-* "${ISO_DIR}/boot/initrd.img"

# GRUB config for BIOS + UEFI boot
cat << 'EOF' > "${ISO_DIR}/boot/grub/grub.cfg"
set default=0
set timeout=5

menuentry "MDB-OS — Multidimensional Binary OS" {
    linux /boot/vmlinuz boot=live quiet splash
    initrd /boot/initrd.img
}

menuentry "MDB-OS (Safe Mode)" {
    linux /boot/vmlinuz boot=live single nomodeset
    initrd /boot/initrd.img
}
EOF

# Build the ISO with both BIOS and UEFI support
xorriso -as mkisofs \
    -iso-level 3 \
    -full-iso9660-filenames \
    -volid "MDBOS" \
    -eltorito-boot boot/grub/bios.img \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --eltorito-catalog boot/grub/boot.cat \
    --grub2-boot-info --grub2-mbr /usr/lib/grub/i386-pc/boot_hybrid.img \
    -eltorito-alt-boot \
        -e EFI/efiboot.img -no-emul-boot \
    -append_partition 2 0xef "${ISO_DIR}/EFI/efiboot.img" \
    -output "$OUTPUT" \
    "${ISO_DIR}"

echo ""
echo "╔══════════════════════════════════════╗"
echo "║     ✅  MDB-OS ISO built!            ║"
echo "║     Output: ${OUTPUT}                ║"
echo "╚══════════════════════════════════════╝"
echo ""
echo "Flash to USB:  sudo dd if=${OUTPUT} of=/dev/sdX bs=4M status=progress"
echo "Or boot in VM: qemu-system-x86_64 -cdrom ${OUTPUT} -m 2G -enable-kvm"

#!/usr/bin/env bash
# Build the Kylin V10 SP1-targeted .deb packages for Linux.
#
#   $DIST/photocraft-<version>-kylin-v10-sp1-<arch>.deb          regular package
#   $DIST/photocraft-<version>-kylin-v10-sp1-<arch>-offline.deb  + bundled runtime dependency closure
# <arch> is x64 or aarch64, matching the naming of the other release assets; the deb's own
# Architecture field stays dpkg's amd64/arm64.
#
# Run inside an Ubuntu 20.04 (focal) container, or on a host with glibc 2.31 — the same
# baseline as Kylin V10 SP1 (glibc 2.31-0kylin, Ubuntu 20.04 blood). Focal is preferred over
# Debian bullseye (same glibc but a different package lineage): the offline closure's package
# names and versions then match the target distro exactly (libicu66 against bullseye's
# libicu63, ...), so the name-mismatch class of failures disappears at the root. Everything
# newer (Kylin V11, UOS/deepin, Debian 11+) accepts the binaries too. The regular distro
# packages (packaging/linux/package.sh) are built on Ubuntu 22.04 (glibc 2.35) and cannot run
# here.
#
# Usage: packaging/linux/kylin/package.sh [--skip-build]
#
# Needs inside the container:
#   - rustup's cargo (focal's rustc 1.75 is below the workspace rust-version),
#   - git, curl, ca-certificates, build-essential, pkg-config,
#   - dpkg-dev (dpkg-deb) and working apt sources for the offline closure.
#
# Two hard boundaries (see ../kylin README and the deepin-screenshot packaging notes):
#   - glibc can never be bundled or replaced: the target system needs glibc >= 2.31.
#   - dpkg holds its lock for the whole run, so no maintainer script may call `dpkg -i`.
#     The offline postinst only detects missing libraries and schedules install-deps.sh
#     through systemd once dpkg is done.
# And the red line for offline installs: a third-party package must never upgrade a component
# the target system already has. install-deps.sh therefore only installs packages that are
# missing entirely, and the build-time closure blacklist keeps system core (systemd, dbus,
# PAM, udev, login, passwd) and glibc out of the bundle.
set -euo pipefail
# shellcheck source=../../env.sh
. "$(dirname "${BASH_SOURCE[0]}")/../../env.sh"
HERE="$ROOT/packaging/linux"
APP_ID=ai.storyteller.photocraft
PACKAGE=photocraft

SKIP_BUILD=0
while [ $# -gt 0 ]; do
  case "$1" in
    --skip-build) SKIP_BUILD=1; shift ;;
    -h | --help) sed -n '2,30p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

ARCH="$(uname -m)"
case "$ARCH" in
  x86_64) OSARCH=amd64; DEB_ARCH=amd64; FILE_ARCH=x64 ;;
  aarch64 | arm64) OSARCH=aarch64; DEB_ARCH=arm64; FILE_ARCH=aarch64 ;;
  *) echo "unsupported architecture $ARCH (this path targets the amd64/aarch64 Kylin V10 SP1 images)" >&2; exit 2 ;;
esac
# A distinct label, so `--version` ("0.1.0 (kylin)") and the file names can't be mistaken for
# the regular distro build, which links a newer glibc.
export PHOTOCRAFT_BUILD_LABEL="${PHOTOCRAFT_BUILD_LABEL:-kylin}"
export PHOTOCRAFT_MAINTAINER="${PHOTOCRAFT_MAINTAINER:-PhotoCraft maintainers <photocraft@storyteller.ai>}"
DEB_VERSION="${VERSION}+kylin"
BASENAME="photocraft-$VERSION-kylin-v10-sp1-$FILE_ARCH"

if [ -z "${CRAFT_FONTS_DIR:-}" ]; then
  warn "CRAFT_FONTS_DIR is not set: the build embeds no fonts (CJK interface text then falls back to a system font at run time, as on every build without it)"
fi
for tool in cargo git strip dpkg-deb ldd; do
  command -v "$tool" >/dev/null 2>&1 || { echo "error: $tool not found (see the header of this script)" >&2; exit 1; }
done

echo "==> PhotoCraft $VERSION (+kylin) for Linux $OSARCH, glibc 2.31 baseline"

if [ "$SKIP_BUILD" = 0 ]; then
  # heif for both binaries; the activation gate (startup registration) only exists on the app.
  (cd "$ROOT" && cargo build --release --locked -p photocraft -p photocraft-cli --features heif,photocraft/activation-gate)
fi
BIN="$CARGO_TARGET_DIR/release"
WORK="$CARGO_TARGET_DIR/kylin-package"
STAGE="$WORK/root"
rm -rf "$WORK"

# ---- stage an FHS tree (mirrors packaging/linux/package.sh; the two differ in the glibc baseline
# and the packaging backend — nfpm with strict Depends there, plain dpkg-deb here — so they stay
# separate on purpose) -----------------------------------------------------------
install -Dm755 "$BIN/photocraft" "$STAGE/usr/bin/photocraft"
install -Dm755 "$BIN/photocraft-cli" "$STAGE/usr/bin/photocraft-cli"
strip "$STAGE/usr/bin/photocraft" "$STAGE/usr/bin/photocraft-cli" 2>/dev/null || true
install -Dm644 "$HERE/$APP_ID.desktop" "$STAGE/usr/share/applications/$APP_ID.desktop"
install -Dm644 "$HERE/$APP_ID.mime.xml" "$STAGE/usr/share/mime/packages/$APP_ID.xml"
mkdir -p "$STAGE/usr/share/metainfo"
sed -e "s/@VERSION@/$VERSION/g" -e "s/@DATE@/$PHOTOCRAFT_BUILD_DATE/g" \
  "$HERE/$APP_ID.metainfo.xml.in" >"$STAGE/usr/share/metainfo/$APP_ID.metainfo.xml"
mkdir -p "$STAGE/usr/share/icons"
cp -R "$ROOT/assets/app-icon/hicolor" "$STAGE/usr/share/icons/"
mkdir -p "$STAGE/usr/share/doc/photocraft"
copy_docs "$STAGE/usr/share/doc/photocraft"
# The activation gate reads its marker from <exe>/../share/photocraft/.license.md, which the
# deb layout provides (/usr/share/photocraft). Nothing to stage: the file appears on first
# activation.

if command -v desktop-file-validate >/dev/null 2>&1; then
  desktop-file-validate "$STAGE/usr/share/applications/$APP_ID.desktop"
fi
if command -v appstreamcli >/dev/null 2>&1; then
  appstreamcli validate --no-net --explain "$STAGE/usr/share/metainfo/$APP_ID.metainfo.xml"
fi

# Only the true ELF requirements are Depends (glibc and libgcc_s, exactly what the binaries link).
# Windowing (X11/Wayland, xkbcommon), the GPU (Vulkan/EGL/GL) and CUPS printing are loaded at run
# time, so — unlike the nfpm deb, which can name them for modern distros — they are not listed
# here: Kylin's package set differs too much for strict names to hold, and the offline variant
# carries them instead.
write_control() {
  local dir="$1" flavour="$2"
  mkdir -p "$dir/DEBIAN"
  cat >"$dir/DEBIAN/control" <<EOF
Package: $PACKAGE
Version: $DEB_VERSION
Architecture: $DEB_ARCH
Maintainer: $PHOTOCRAFT_MAINTAINER
Homepage: https://getartcraft.com/apps/photocraft
Section: graphics
Priority: optional
Depends: libc6 (>= 2.31), libgcc-s1
Description: Open-source, native image editor ($flavour, glibc 2.31 build)
 Layers, masks, adjustments, brushes, text, vectors and smart objects,
 with 1:1 Photoshop parity and PSD fidelity. Includes photocraft-cli,
 a headless converter and command runner.
 .
 Built inside Ubuntu 20.04 (focal) for systems whose glibc is older than
 the regular photocraft deb requires (2.35): Kylin V10 SP1 (Ubuntu 20.04
 base, glibc 2.31-0kylin), UOS/deepin desktops and Debian 11.
 .
 Only libc6 and libgcc_s are Depends. The X11/Wayland, xkbcommon and
 Vulkan/GL libraries are loaded at run time and a desktop system has
 them; on a minimal or offline system install the -offline deb instead,
 which bundles them.
EOF
}

# ---- regular deb --------------------------------------------------------------------------------
PKGDIR="$WORK/${PACKAGE}_${DEB_VERSION}_${DEB_ARCH}"
mkdir -p "$PKGDIR"
cp -R "$STAGE/." "$PKGDIR/"
write_control "$PKGDIR" "Kylin V10 SP1 build"
install -m755 "$HERE/postinst.sh" "$PKGDIR/DEBIAN/postinst"
dpkg-deb --build --root-owner-group "$PKGDIR" "$DIST/$BASENAME.deb"
# A packaging step that silently drops the staged tree would produce a small, confident,
# useless package, so every deb must prove it carries the application before it ships.
# No `grep -q` on the pipe: it exits at the first match and SIGPIPEs dpkg-deb's tar,
# which pipefail reports as a failure even on a match.
dpkg-deb -c "$DIST/$BASENAME.deb" | grep "usr/bin/photocraft" >/dev/null || { echo "error: $BASENAME.deb does not contain the application" >&2; exit 1; }
echo "wrote $DIST/$BASENAME.deb"

# ---- offline deb --------------------------------------------------------------------------------
# Bundle the runtime dependency closure so `dpkg -i` alone leaves a working install on a machine
# without network. The closure is expanded recursively from Depends: target distros package some
# libraries under different names than other distros, and a single `ldd` round only catches
# direct needs. With the focal build base the names match Kylin V10 SP1 exactly.
PKGDIR="$WORK/${PACKAGE}_${DEB_VERSION}_${DEB_ARCH}-offline"
DEPS_DIR="$PKGDIR/usr/share/photocraft/deps"
CACHE="$WORK/offline-cache"
mkdir -p "$PKGDIR/DEBIAN" "$DEPS_DIR" "$CACHE"
cp -R "$STAGE/." "$PKGDIR/"

LIBS="$(ldd "$STAGE/usr/bin/photocraft" "$STAGE/usr/bin/photocraft-cli" | awk '{print $1}' | sort -u)"
echo "linked libraries: $LIBS"
TOP=""
# Package names are plain tokens; word splitting is the point.
# shellcheck disable=SC2086
for lib in $LIBS; do
  # dpkg -S exits 1 for loader-internal names (linux-vdso, the loader itself); || true keeps
  # set -e from treating the lookup as a failure
  p="$(dpkg -S "$lib" 2>/dev/null | head -1 | cut -d: -f1 || true)"
  [ -n "$p" ] && TOP="$TOP $p"
done
# shellcheck disable=SC2086
TOP="$(printf '%s\n' $TOP | sed '/^$/d' | sort -u | tr '\n' ' ')"
if [ -z "${TOP// /}" ]; then
  echo "error: could not map any linked library to a package" >&2
  exit 1
fi
echo "offline deps direct closure:$TOP"

PKGS="$TOP"
for round in 1 2 3 4 5; do
  rm -rf "$CACHE"
  mkdir -p "$CACHE"
  # shellcheck disable=SC2086
  apt-get install --reinstall --download-only --no-install-recommends -y \
    -o Dir::Cache::archives="$CACHE" $PKGS >/dev/null
  MORE=""
  for f in "$CACHE"/*.deb; do
    [ -e "$f" ] || continue
    # One candidate per alternative (drop "pkg (relation version)" and "| alternatives"): the
    # bundle needs at least one provider of each, and the first is the preferred one.
    deps="$(dpkg-deb -f "$f" Depends 2>/dev/null | sed 's/ ([^)]*)//g' | tr ',' '\n' | sed 's/|.*//; s/^ *//; s/ *$//' | sed '/^$/d' || true)"
    MORE="$MORE $deps"
  done
  # shellcheck disable=SC2086
  NEW="$(printf '%s\n' $PKGS $MORE | sed '/^$/d' | sort -u | tr '\n' ' ')"
  # shellcheck disable=SC2086
  CUR="$(printf '%s\n' $PKGS | sed '/^$/d' | sort -u | tr '\n' ' ')"
  if [ "$NEW" = "$CUR" ]; then
    # the closure is a plain token list; word splitting is the point
    # shellcheck disable=SC2086
    echo "closure stable after round $round ($(printf '%s\n' $NEW | sed '/^$/d' | wc -l) packages)"
    break
  fi
  PKGS="$NEW"
  # shellcheck disable=SC2086
  echo "closure round $round grew to $(printf '%s\n' $NEW | sed '/^$/d' | wc -l) packages"
done

# The blacklist is a red line, not a size optimisation: bundling any of these and "upgrading"
# the target system's copy has bricked machines (systemd 245 -> 247 deepin-screenshot
# incident). Essential=yes, glibc and the system core never enter the bundle; install-deps.sh
# additionally only ever installs what is missing entirely.
is_system_core() {
  case "$1" in
    systemd* | dbus* | libpam* | udev | login | passwd) return 0 ;;
    *) return 1 ;;
  esac
}
COUNT=0
for f in "$CACHE"/*.deb; do
  [ -e "$f" ] || continue
  COUNT=$((COUNT + 1))
  p="$(dpkg-deb -f "$f" Package)"
  # glibc must never be replaced by a bundled copy; Essential packages are on every system;
  # the system core is untouchable (see the red line above).
  [ "$(dpkg-deb -f "$f" Essential)" = "yes" ] && continue
  [ "$p" = "libc6" ] && continue
  if is_system_core "$p"; then
    echo "skipped $p (system core, never bundled)"
    continue
  fi
  cp "$f" "$DEPS_DIR/"
done
if [ "$COUNT" -eq 0 ]; then
  echo "error: the offline closure downloaded no packages — are the apt sources working? (see the header of this script)" >&2
  exit 1
fi
echo "bundled $(find "$DEPS_DIR" -maxdepth 1 -name '*.deb' | wc -l) dependency packages into $DEPS_DIR"

cat >"$PKGDIR/usr/share/photocraft/install-deps.sh" <<'SH'
#!/bin/sh
# Install the runtime dependencies bundled in ./deps — but ONLY the ones missing on this
# system. A package the system already has is kept, whatever its version: a third-party
# offline package must never upgrade a component the target system ships (the systemd-245 ->
# 247 incident bricked machines). Qt-era ABI worries do not apply here — PhotoCraft bundles
# no Qt — and the ldd check at the end is what actually decides whether the app can run.
set -e
BINS="/usr/bin/photocraft /usr/bin/photocraft-cli"
DIR="$(cd "$(dirname "$0")" && pwd)/deps"
cd "$DIR"
MISSING=""
for f in *.deb; do
  pkg="$(dpkg-deb -f "$f" Package)"
  ver="$(dpkg-deb -f "$f" Version)"
  stat="$(dpkg-query -W -f='${db:Status-Abbrev}' "$pkg" 2>/dev/null || true)"
  case "$stat" in
    ii*)
      inst="$(dpkg-query -W -f='${Version}' "$pkg" 2>/dev/null || true)"
      echo "$pkg: system has $inst, keeping it (never upgrading)"
      ;;
    *)
      echo "$pkg: missing, will install $f ($ver)"
      MISSING="$MISSING $f"
      ;;
  esac
done
if [ -n "$MISSING" ]; then
  # a single dpkg call, so the packages configure in dependency order
  # shellcheck disable=SC2086
  dpkg -i $MISSING
else
  echo "nothing to install"
fi
echo ""
STILL=""
for b in $BINS; do
  [ -f "$b" ] || continue
  STILL="$STILL $(ldd "$b" 2>/dev/null | awk '/not found/ {print $1}')"
done
# shellcheck disable=SC2086
if [ -n "$(printf '%s' $STILL | tr -d '[:space:]')" ]; then
  echo "WARNING: these libraries are still missing, the app will not start:"
  # shellcheck disable=SC2086
  printf '%s\n' $STILL | sed '/^$/d'
  exit 1
fi
echo "OK: all runtime libraries resolved. photocraft is ready."
SH
chmod 755 "$PKGDIR/usr/share/photocraft/install-deps.sh"

cat >"$PKGDIR/DEBIAN/postinst" <<'EOF'
#!/bin/sh
# dpkg holds its lock while this script runs, so the bundled dependency debs cannot be installed
# here (a nested dpkg -i fails on the lock). Refresh the desktop caches, detect missing runtime
# libraries, and schedule the installer through systemd once dpkg is done.
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database -q /usr/share/applications || true
fi
if command -v update-mime-database >/dev/null 2>&1; then
  update-mime-database /usr/share/mime || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
fi
# The dependency check below is meaningless without the application: a package that lost
# its own payload must say so instead of reporting every library present.
if [ ! -f /usr/bin/photocraft ] && [ ! -f /usr/bin/photocraft-cli ]; then
  echo "photocraft: the application files are missing from this package; reinstall it." >&2
  exit 1
fi
MISSING=""
for b in /usr/bin/photocraft /usr/bin/photocraft-cli; do
  [ -f "$b" ] || continue
  MISSING="$MISSING $(ldd "$b" 2>/dev/null | awk '/not found/ {print $1}')"
done
if [ -z "$(printf '%s' "$MISSING" | tr -d '[:space:]')" ]; then
  echo "photocraft: all runtime libraries present, nothing to do."
  exit 0
fi
echo "photocraft: missing runtime libraries:"
printf '%s\n' $MISSING | sed '/^$/d; s/^/  /'
if command -v systemd-run >/dev/null 2>&1; then
  echo "photocraft: auto-installing the missing ones from the bundled offline deps in 10s"
  echo "  (same as running: sudo /usr/share/photocraft/install-deps.sh)"
  echo "  (result: journalctl -u photocraft-deps)"
  systemd-run --on-active=10 --quiet --unit=photocraft-deps \
    /usr/share/photocraft/install-deps.sh 2>/dev/null \
    || echo "photocraft: auto-install could not be scheduled; run the command above manually."
else
  echo "photocraft: run this once to install them from the bundled offline deps:"
  echo "  sudo /usr/share/photocraft/install-deps.sh"
fi
exit 0
EOF
chmod 755 "$PKGDIR/DEBIAN/postinst"
write_control "$PKGDIR" "Kylin V10 SP1 offline build"
dpkg-deb --build --root-owner-group "$PKGDIR" "$DIST/$BASENAME-offline.deb"
CONTENTS="$(dpkg-deb -c "$DIST/$BASENAME-offline.deb")"
check_in_offline() {
  echo "$CONTENTS" | grep "$1" >/dev/null || { echo "error: $BASENAME-offline.deb does not contain $1" >&2; exit 1; }
}
check_in_offline "usr/bin/photocraft"
check_in_offline "usr/share/photocraft/install-deps.sh"
check_in_offline "usr/share/photocraft/deps/.*\.deb"
echo "wrote $DIST/$BASENAME-offline.deb"

"$STAGE/usr/bin/photocraft-cli" --version
echo "==> done"
ls -lh "$DIST"

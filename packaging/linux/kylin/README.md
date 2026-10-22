# Kylin V10 SP1 packages (glibc 2.31 baseline)

Deb packages for Kylin V10 SP1 desktop (2503, amd64 + aarch64) and systems of the same
blood: UOS/deepin, Debian 11. The regular distro packages ([`../package.sh`](../package.sh))
are built on Ubuntu 22.04 and their binaries need glibc >= 2.35, which Kylin V10 SP1
(glibc 2.31-0kylin, Ubuntu 20.04 base) cannot provide — glibc can never be bundled, so the
baseline has to come from the build machine.

## Why Ubuntu 20.04 (focal), not Debian bullseye

Bullseye has the same glibc (2.31) but a different package lineage. Focal is Kylin V10 SP1's
direct ancestor, so the offline closure's package names and versions match the target distro
exactly (`libicu66`, not bullseye's `libicu63`), which removes the name-mismatch class of
failures at the root. Build with the workflow (`.github/workflows/kylin-v10-sp1-deb.yml`) or,
locally:

```sh
docker run -it --rm -v "$PWD":/src -w /src ubuntu:20.04 bash -c '
  apt-get update && apt-get install -y build-essential pkg-config git curl ca-certificates dpkg-dev
  curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  . "$HOME/.cargo/env"
  packaging/linux/kylin/package.sh'
```

`--locked` requires the lockfile to include the activation crate (it does, since the gate
landed); a fresh checkout always matches.

## Products

| File | What it is |
|---|---|
| `photocraft-<version>-kylin-<osarch>.deb` | regular package; a desktop system provides the runtime libraries the app loads at run time (X11/Wayland, xkbcommon, Vulkan/GL, fontconfig) |
| `photocraft-<version>-kylin-<osarch>-offline.deb` | for minimal or offline machines: bundles the runtime dependency closure under `/usr/share/photocraft/deps` |

`<osarch>` is `amd64` or `aarch64`. Both build natively (no cross compilation) inside a
focal container. The version carries a `+kylin` Debian revision and the binaries report
`photocraft <version> (kylin; …)` via `PHOTOCRAFT_BUILD_LABEL` (see
`crates/engine/src/build_info.rs`), so they can't be mistaken for the regular distro build.

## The offline package and the red line

After `dpkg -i photocraft-…-offline.deb`:

- the postinst refreshes the desktop caches, runs `ldd` on both binaries, and — when
  something is missing — schedules `/usr/share/photocraft/install-deps.sh` through systemd
  10 seconds later (`journalctl -u photocraft-deps`); without systemd it prints the command.
- `install-deps.sh` **only installs packages that are missing entirely**. A package the
  system already has is kept whatever its version. A third-party offline package must never
  upgrade a component the target system ships — the deepin-screenshot incident (bundled
  systemd 247 replacing Kylin's 245, `/sbin/init` and all) bricked machines.
- the build-time closure never even downloads the danger: `Essential=yes` packages, `libc6`
  and the system core (`systemd*`, `dbus*`, `libpam*`, `udev`, `login`, `passwd`) are
  blacklisted in `package.sh`.
- every deb asserts its own payload after building (`dpkg-deb -c`), and the postinst refuses
  to report success when the application files themselves are missing.

The CI verify job installs the offline deb into the real Kylin V10 SP1 base container
(`ghcr.io/distrotwin/kylin:v10sp1-base-amd64-*`, pinned by date) and, beyond the `ldd` gate,
asserts the zero-upgrade invariant: the dpkg database may gain new packages, but none may be
upgraded or replaced.

## Activation

Official builds ship with the startup activation gate (`--features photocraft/activation-gate`):
the app shows a one-time registration page (machine code + activation code) and stores the
answer beside the installed files — for the deb layout, `/usr/share/photocraft/.license.md`
(root-owned, hence the sudo hint the UI shows when it can't write there). It is a free local
marker, not a licence server. `photocraft-cli activation` answers the machine code words, and
`packaging/activation-tool.html` converts machine codes to activation codes offline.

## Checks before shipping

```sh
# glibc symbol requirement must be <= 2.31 (run on the built binaries):
objdump -T usr/bin/photocraft | grep -oE 'GLIBC_[0-9.]+' | sort -uV | tail -1
```

`cargo xtask parity`/tests do not apply here: this directory only packages what the workspace
already builds and tests.

## Troubleshooting

- The Packaging lint workflow checks everything under `packaging/` and the workflow files on
  every push (workflow syntax via actionlint, the shell/PowerShell scripts, desktop and
  AppStream metadata). If it fails without any job having run, it was a transient starter
  error: re-run it from the Actions page (*Re-run failed jobs*) or push again.
- A verify-job failure prints the last 30 log lines as check annotations; the zero-upgrade
  assertion failing means some package got upgraded during dependency installation, which on
  Kylin can brick the desktop (see the focal-baseline rationale above) — fix the closure
  blacklist, never loosen the assertion.

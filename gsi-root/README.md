# gsi-root

Rust GSI analyzer / future root engine (see `../../ROADMAP.md` §6 and the
project spec: CLI + Slint GUI later, scripts stay launchers).

## Status (honest)

- **Works today:** `analyze`/`inspect` (container + sparse detection, engine
  verdict), `version`. `cargo test` green.
- **EXPERIMENTAL (refused, exit 3):** `patch`, `verify` — no hardware POC.
- **Later phases:** ext4/CPIO writers, AVB, init parser, SELinux, ADB/Fastboot,
  Slint GUI (Phase 9), updater, hardware harness.

## Layout

```text
gsi-root/
  Cargo.toml            workspace
  crates/
    gsi-image/          container + sparse parsing
    gsi-android/        build.prop parsing
    gsi-root-core/      profiles, engines, plans, manifests
    gsi-root-cli/       `gsi-root` binary (thin CLI)
  devices/
    huawei-p10/         static device facts (profile.toml)
```

## Build / test (Dry Run Tests — no device touched)

```bash
cargo build -p gsi-root-cli
cargo test --workspace
./target/debug/gsi-root analyze <image>
```

No runtime tools required (no adb/fastboot/python/bash for core functions).
New Rust dependencies go into `../../../.vendors/` first (cargo manifest).

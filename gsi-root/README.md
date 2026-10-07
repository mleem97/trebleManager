# gsi-root

Rust GSI analyzer / future root engine (see `../../ROADMAP.md` §6 and the
project spec: CLI + Slint GUI, scripts stay launchers).

## Status (honest)

- **Works today:** `analyze`/`inspect` (container + sparse detection, engine
  verdict), `version`, `workflow list/run` (safe steps), `device slot` +
  switch planning, `device detect|info`, `adb devices`,
  `fastboot devices|getvar`, `tools`, `update check/download/install`,
  `config show`, self-`install` to PATH, Slint GUI (Dashboard/GSI/Updates/
  Logs/Settings). `cargo test` green.
- **EXPERIMENTAL (refused, exit 3):** `patch`, `verify`, flash execution —
  no hardware POC. Native ADB/fastboot protocols: Phase 8.
- **Later phases:** ext4/CPIO writers, AVB, init parser, SELinux, updater
  signatures, hardware harness.

## Layout

```text
gsi-root/
  Cargo.toml            workspace
  crates/
    gsi-image/          container + sparse parsing
    gsi-android/        build.prop parsing
    gsi-root-core/      profiles, engines, plans, manifests
    gsi-root-cli/       `gsi-root` binary (thin CLI)
    gsi-config/         platform directories
    gsi-workflow/       typed workflows, progress events
    gsi-update/         GitHub updater (check/download/verify/install)
    gsi-device/         slot state + switch plans + device parsers
    gsi-tool/           managed external-tool binding
    gsi-fs/             ext4 reader (writer next)
    gsi-root-gui/       Slint GUI (same core as CLI)
  devices/
    huawei-p10/         static device facts (profile.toml)
  docs/
    STATUS.md           spec coverage mapping
```

## Build / test (Dry Run Tests — no device touched)

```bash
cargo build -p gsi-root-cli
cargo test --workspace
./target/debug/gsi-root analyze <image>
```

No runtime tools required (no adb/fastboot/python/bash for core functions).
New Rust dependencies go into `../../../.vendors/` first (cargo manifest).

---

<a id="de"></a>
## Deutsch — gsi-root

Rust-GSI-Analyzer / künftige Root-Engine (siehe `../../ROADMAP.md` §6 und
Projekt-Spec: CLI + Slint-GUI, Scripts bleiben Launcher).

## Stand (ehrlich)

- **Geht heute:** `analyze`/`inspect` (Container + Sparse-Erkennung, Engine-
  Urteil), `version`, `workflow list/run` (sichere Steps), `device slot` +
  Switch-Planung, `device detect|info`, `adb devices`,
  `fastboot devices|getvar`, `tools`, `update check/download/install`,
  `config show`, Self-`install` in PATH, Slint-GUI (Dashboard/GSI/Updates/
  Logs/Settings). `cargo test` grün.
- **EXPERIMENTAL (abgelehnt, Exit 3):** `patch`, `verify`, Flash-Ausführung —
  kein Hardware-POC. Native ADB/Fastboot-Protokolle: Phase 8.
- **Spätere Phasen:** ext4/CPIO-Writer, AVB, Init-Parser, SELinux,
  Updater-Signaturen, Hardware-Harness.

## Bauen / Testen (Dry Run Tests — ohne Gerät)

```bash
cargo build -p gsi-root-cli
cargo test --workspace
./target/debug/gsi-root analyze <image>
```

Keine Runtime-Tools nötig (kein adb/fastboot/python/bash für Core-
Funktionen). Neue Rust-Dependencies zuerst nach `../../../.vendors/`
(Cargo-Manifest).

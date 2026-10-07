# CLI — command reference (PowerShell + Rust `gsi-root`)

Two CLIs exist. The PowerShell CLI (`scripts/Treble-Toolkit.ps1`, mirrored by `scripts/treble-toolkit.sh`) is the full toolkit CLI. The Rust `gsi-root` CLI is a native subset over the same domain crates; `patch`/`verify` there refuse as EXPERIMENTAL.

Related: `docs/WORKFLOWS.md`, `docs/TUI.md`, `docs/UI-ARCHITECTURE.md`.

## 1. PowerShell CLI basics

Usage pattern (source: `Show-TTHelp`):

```powershell
.\scripts\Treble-Toolkit.ps1 <command> [--goal <id>] [--mode safe|unattended|developer] [--json] [--yes] [--image <path>] [--firmware-file <url|path>] [--anonymize] [--no-reboot]
```

No args starts the TUI. Multi-device target is `ANDROID_SERIAL` env, not a flag. Bash port uses the same command names.

## 2. Requested global flags — what actually exists

| Flag | PowerShell CLI | Rust `gsi-root` CLI |
|---|---|---|
| `--json` | Yes (per-command JSON output) | Yes (stable `{"tool","command","status","data"}` envelope) |
| `--yes` | Yes (documented consent, never skips gates) | Yes (destructive gate only; never skips other gates) |
| `--image <path>` | Yes | No (image is a positional arg) |
| `--firmware-file <url\|path>` | Yes | No |
| `--goal <id>` | Yes | No (`goal show <goal>`, `goal screen <step>`) |
| `--mode safe\|unattended\|developer` | Yes | No |
| `--anonymize` | Yes (`diagnostic`) | No |
| `--no-reboot` | Yes | No |
| `--quiet` | No | Yes (errors only) |
| `--verbose` | No | Yes (debug to stderr) |
| `--dry-run` | No | Yes (prints plan, zero executor calls, even with `--yes`) |
| `--no-color` | No | Yes (renders are ANSI-free) |
| `--device` | No (use `ANDROID_SERIAL`) | Yes (`--device <serial>`) |
| `--output` | No | Yes (`--output <path>` writes result to file) |

Rust JSON commands: detect, device info/diagnostics, image analyze/verify, compatibility check, root methods/plan/verify, flash plan/recovery/system, backup list/create/verify/restore, firmware list/download/extract, tools list, update check, workflow list/plan/run, version.

## 3. PowerShell examples (every major command)

Read-only first:

```powershell
.\scripts\Treble-Toolkit.ps1 detect --json
.\scripts\Treble-Toolkit.ps1 devices
.\scripts\Treble-Toolkit.ps1 analyze
.\scripts\Treble-Toolkit.ps1 firmware
.\scripts\Treble-Toolkit.ps1 recon --json
.\scripts\Treble-Toolkit.ps1 status --json
.\scripts\Treble-Toolkit.ps1 preflight --json
.\scripts\Treble-Toolkit.ps1 compat
.\scripts\Treble-Toolkit.ps1 root-methods
.\scripts\Treble-Toolkit.ps1 validate
.\scripts\Treble-Toolkit.ps1 verify
.\scripts\Treble-Toolkit.ps1 help
```

Plan / wizard / goals:

```powershell
.\scripts\Treble-Toolkit.ps1 workflow --goal root
.\scripts\Treble-Toolkit.ps1 workflow --goal root --json
.\scripts\Treble-Toolkit.ps1 workflow --goal custom_rom
.\scripts\Treble-Toolkit.ps1 wizard
.\scripts\Treble-Toolkit.ps1 resume
.\scripts\Treble-Toolkit.ps1 root
```

Goal ids (all nine, source `$WorkflowGoals`): `root`, `custom_rom`, `stock_rom`, `root_custom_rom`, `root_stock_rom`, `root_custom_rom_recovery`, `root_stock_rom_recovery`, `restore_original`, `full_reinstall`.

Files, images, firmware (consent-gated where noted):

```powershell
.\scripts\Treble-Toolkit.ps1 download --firmware-file "<https-URL-to-full-ZIP>" --yes
.\scripts\Treble-Toolkit.ps1 extract
.\scripts\Treble-Toolkit.ps1 export --image ".\data\roms\custom-rom.zip"
.\scripts\Treble-Toolkit.ps1 patch
.\scripts\Treble-Toolkit.ps1 backup
.\scripts\Treble-Toolkit.ps1 flash --image ".\data\magisk\magisk_patched.img" --yes
.\scripts\Treble-Toolkit.ps1 flash-system --image ".\data\roms\system.img" --yes
.\scripts\Treble-Toolkit.ps1 twrp --image ".\data\recovery\twrp.img" --yes
.\scripts\Treble-Toolkit.ps1 restore --yes
.\scripts\Treble-Toolkit.ps1 wipe --yes
.\scripts\Treble-Toolkit.ps1 reinstall --image ".\data\roms\system.img" --yes
.\scripts\Treble-Toolkit.ps1 persist --yes
.\scripts\Treble-Toolkit.ps1 rom --image ".\data\roms\custom-rom.zip"
.\scripts\Treble-Toolkit.ps1 download-rom --image ".\data\roms\custom-rom.zip"
.\scripts\Treble-Toolkit.ps1 diagnostic --anonymize
.\scripts\Treble-Toolkit.ps1 diagnostic --anonymize --json
```

Developer dumps (`--mode developer` unlocks these):

```powershell
.\scripts\Treble-Toolkit.ps1 dump-partitions --mode developer
.\scripts\Treble-Toolkit.ps1 dump-properties --mode developer
.\scripts\Treble-Toolkit.ps1 dump-vendor --mode developer
.\scripts\Treble-Toolkit.ps1 dump-logs --mode developer
```

Bash equivalents: replace the PS call with `./scripts/treble-toolkit.sh <same args>` (for example `./scripts/treble-toolkit.sh detect --json`).

## 4. Rust `gsi-root` examples

Binary help is the reference (`gsi-root --help`; no args starts the GUI, not help).

```sh
gsi-root version
gsi-root analyze ./lineage-20.img
gsi-root inspect ./lineage-20.img.gz
gsi-root patch ./lineage-20.img
gsi-root verify ./lineage-20.img
gsi-root workflow list
gsi-root workflow run p10-lineage20
gsi-root workflow run p10-lineage20 --yes
gsi-root update check --repo mleem97/trebleManager
gsi-root update download --repo mleem97/trebleManager --asset v2.22.0/gsi-root --out /tmp/gsi-root --sha256 <hex>
gsi-root update install --staged /tmp/gsi-root --target ~/.local/bin/gsi-root
gsi-root update rollback --target ~/.local/bin/gsi-root
gsi-root device detect
gsi-root device info
gsi-root device slot
gsi-root device slot --state-file ./workflow-state.json
gsi-root device switch --to magisk --magisk ./magisk_patched.img --state-file ./workflow-state.json
gsi-root device switch --to twrp --twrp ./twrp.img
gsi-root adb devices
gsi-root fastboot devices
gsi-root fastboot getvar all
gsi-root tools
gsi-root tools config --config ./data/config.json
gsi-root tools save-config --out ./tool-config.json --adb /usr/bin/adb --fastboot /usr/bin/fastboot
gsi-root tools path-plan --dir ./tools
gsi-root tools link-plan --src ./dl --central ./tools
gsi-root tools install-plan --registry ./data/compatibility/huawei/p10/VTR-L29.json --os linux --dest ./tools
gsi-root config show
gsi-root config tool-root --script-dir ./scripts
gsi-root select rom --profile VTR-L29
gsi-root select image --profile VTR-L29 --android 13 --system LineageOS --variant bgN
gsi-root select device --pick 1
gsi-root select root-target --pick 1
gsi-root goal show root
gsi-root goal screen firmware
gsi-root progress demo
gsi-root verdict flash --what flash --file ./flash.log
gsi-root verdict readiness --all-ok
gsi-root preflight
gsi-root preflight --adb /usr/bin/adb --fastboot /usr/bin/fastboot --writable ./data --file ./x.img --expect-hash ./x.img=<hex>
gsi-root firmware-baseline --display 9.1.0.275 --incremental VTR-L29 --baseline EMUI9.1
gsi-root install
gsi-root install --yes
```

`patch` and `verify` always print `EXPERIMENTAL — refused` (exit 3). `workflow run` without `--yes` marks Flash/Reboot/Test as skipped; with `--yes` they are still refused without a device layer (`gsi-exec` stub path, exit 3 on failure).

## 5. JSON schema (PowerShell only)

No schema file exists. `--json` prints per-command `ConvertTo-Json` objects:

- `preflight --json`: `{go, blocks, adb, fastboot}` device/tool states.
- `status --json`: full state (preflight, devices, mode, goal, saved steps) from `logs/workflow-state.json` plus live values.
- `recon --json`: props plus by-name plus getvar findings.
- `workflow --goal <id> --json`: ordered plan steps with live gate results (plan only, no execution).
- `diagnostic --json`: `{zip}` path of the 10-file ZIP in `logs/`.
- `firmware`, `download`, `devices`, `validate`, `verify`, `restore` equivalents print their result objects; without `--json` the same data renders as text lines.
- Rust CLI output is plain text lines (`file:`, `container:`, `engine:`, `reason:`, `plan:`, `warn:`); there is no `--json` there.

## 6. Safety (`--yes` semantics)

- `--yes` is documented consent, not a bypass. Gates, readiness checks, hash checks, backup checks, and typed-token rules still run.
- PowerShell: download/flash/restore/wipe/reinstall/persist/resume/root refuse without `--yes` (or interactive typed confirm in TUI). Example: `download` without `--yes` prints the candidate and `Confirm with --yes`; `resume`/`root` tell you to re-run with `--yes`.
- Rust: only `workflow run` and `install` read `--yes`. Destructive `gsi-exec` runners additionally require both confirm tokens; the CLI passes `FLASH`/`YES` (or `RESTORE`/`YES`) only after validation.
- Never scripted around: no force/wipe/format flags exist; `flashing unlock`, auto `erase/format userdata`, and verity-disable flags are out of scope.

## 7. Exit codes

PowerShell (`scripts/Treble-Toolkit.ps1`, observed):

| Code | Meaning |
|---|---|
| 0 | OK / help shown |
| 1 | Blocked or failed (preflight blocked, goal failed, flash failed, install failed) |
| 2 | Verified not-rooted (`verify` without `uid=0`) or report failure |
| 3 | Missing/invalid input (unknown goal pick, no ROM package, no stock/patched image, image check FAIL) |
| 4 | Consent missing (needs `--yes`; download/flash/restore/wipe/reinstall/persist/resume/root plan shown, nothing executed) |

Rust (`gsi-root`, observed):

| Code | Meaning |
|---|---|
| 0 | OK (analyze supported, workflow done, install/rollback done) |
| 1 | Generic failure (analyze Unsupported, device/tool missing, download/verify/install error, select/parse error) |
| 2 | Usage error (unknown command/args, missing required flags, `need input` switch plans) |
| 3 | Honest refusal (EXPERIMENTAL patch/verify, unverified download without hash, device flash plan without Phase-8 layer) |

GUI init/GUI runtime errors return 1.

---

<a id="de"></a>
## Deutsch — CLI (Kurzspiegel)

Zwei CLIs: PowerShell-CLI (voll) und Rust-`gsi-root` (Subset; `patch`/`verify` EXPERIMENTAL-abgelehnt).

- PowerShell-Muster: `<command> [--goal] [--mode] [--json] [--yes] [--image] [--firmware-file] [--anonymize] [--no-reboot]`; ohne Args TUI; Multi-Device via `ANDROID_SERIAL`.
- Angefragte Global-Flags: nur `--json` (PS), `--yes` (beide, eng begrenzt), `--image`/`--firmware-file`/`--goal`/`--mode`/`--anonymize`/`--no-reboot` (PS) existieren. `--quiet`, `--verbose`, `--dry-run`, `--no-color`, `--device`, `--output` existieren in keiner CLI (Rust nutzt `--out` nur bei `update download`).
- Beispiele oben je Command (read-only, Plan/Goals, Dateien/Firmware mit `--yes`, Developer-Dumps mit `--mode developer`; Rust je Subcommand mit seinen Flags).
- JSON: keine Schema-Datei; PS-`--json` printet Command-Objekte (`preflight`, `status`, `recon`, `workflow`-Plan, `diagnostic`-ZIP u.a.); Rust-CLI nur Klartext, kein `--json`.
- Safety: `--yes` ist dokumentierte Zustimmung, kein Bypass; ohne `--yes` (oder getippte TUI-Bestätigung) wird nichts Destruktives ausgeführt. Keine Force/Wipe/Unlock-Flags.
- Exit-Codes: PS 0 OK, 1 blockiert/fehlgeschlagen, 2 nicht-gerootet/Report-Fehler, 3 Eingabe fehlt/ungültig, 4 Zustimmung fehlt. Rust 0 OK, 1 Fehler, 2 Usage, 3 ehrliche Refusal (EXPERIMENTAL/ohne Device-Layer).

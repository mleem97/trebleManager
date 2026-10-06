# Security Policy — trebleManager

## Reporting

Please do **not** report vulnerabilities as public issues, but via email to the maintainer
([mleem97](https://github.com/mleem97)) with subject `[SECURITY] trebleManager` and the following information:

- affected version (`VERSION` / commit),
- steps to reproduce,
- possible impact.

Response target: confirmation within 72 hours. Details will only be published after a fix
(coordinated disclosure).

## Supported Versions

| Version | Support |
|---|---|
| Latest release (`VERSION`) | ✅ |
| Older | ❌ (best-effort only) |

## Tool Safety Model

This tool modifies the boot chain. Safety beats automation:

- 9-point safety gate before every flash (model, partition, image, hash, size, firmware
  compat, backup, fastboot link, patched `!=` stock). Any FAIL → `DO NOT FLASH`.
- Flash/restore need double confirmation (`FLASH`+`YES` / `RESTORE`+`YES`, CLI: `--yes`).
- Backup of the original image is mandatory before flashing (with `metadata.json` + hashes).
- Never automatic: `erase/format userdata`, `flashing unlock`, bootloader unlock,
  `system`/`vendor`/`userdata` flashes. The GSI stays intact.
- Never any `--force`, `--disable-verity`, `--disable-verification`, or unknown
  fastboot flags — neither in code nor in guides. A refusing device is data,
  never an obstacle to force through.
- No fake success paths: impossible operations return real errors and exit codes
  (flash=1, verify=2, extract/backup/export=3, download-confirm-missing=4).
- Diagnostic ZIPs may contain device-specific values (`getvar all`) — anonymize
  (`--anonymize`) or redact serials before sharing.

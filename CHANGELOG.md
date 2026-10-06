# Changelog — trebleManager

## Unreleased

## v2.1.0

- Bilingual TUI (English default, German if system language is German)
- Recovery export from compatible custom ROMs (direct `.img`, ROM `.zip`, `payload.bin` via payload-dumper-go; GSI system images refused honestly) + CLI `export`
- Stock firmware downloader with progress (BITS resume + WebClient fallback) and mandatory confirmation + CLI `download`
- Full CLI: `detect|analyze|firmware|download|extract|export|patch|backup|flash|verify|restore|diagnostic|wizard|help` with `--json`
- 9-point safety gate with `DO NOT FLASH`, derived flash command, double confirmation
- Backup/restore with `metadata.json` + SHA-256/512, diagnostic ZIP (10 files, `--anonymize`)
- Docs in gregCore style: README, INSTRUCTIONS, QUICKSTART, ARCHITECTURE, SECURITY, TROUBLESHOOTING, CONTRIBUTING, CODE_OF_CONDUCT

## v2.0.0

- Complete PowerShell TUI + CLI wizard (steps 1–9) for VTR-L29/VTR-L09/VKY-L29
- OS-independent analysis (Stock EMUI, TrebleDroid/Lineage GSI, custom ROMs)
- Fixed prerequisite checker BAT (auto Android/fastboot mode, desktop log)
- Smart launchers (local elevation + direct-from-GitHub loader)

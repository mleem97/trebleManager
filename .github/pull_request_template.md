# Pull request

## What / why

## Test evidence

- [ ] `bash tests/test-parsers.sh` green (Linux) and/or `tests/Test-Parsers.ps1` green (Windows)
- [ ] Manual run noted (TUI path or CLI command + device state: Android/fastboot/none)
- [ ] No destructive default added (flash/restore still need explicit confirmation)

## Docs

- [ ] `README.md` / `INSTRUCTIONS.md` / `QUICKSTART.md` updated if behavior changed
- [ ] `CHANGELOG.md` (Unreleased) updated
- [ ] UI strings bilingual via `L "en" "de"` (PowerShell) / `L 'en' 'de'` (bash); repo files stay English

## Safety (tool changes only)

- [ ] Safety gate still blocks on any FAIL (`DO NOT FLASH`)
- [ ] No new `erase`/`format`/`flashing unlock`/bootloader-unlock path
- [ ] No fake success: impossible operations error honestly with exit codes

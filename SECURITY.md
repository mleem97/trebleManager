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

---

<a id="de"></a>
## Deutsch — Sicherheitsrichtlinie

### Meldung

Bitte Schwachstellen **nicht** als öffentliche Issues melden, sondern per
E-Mail an den Maintainer ([mleem97](https://github.com/mleem97)) mit Betreff
`[SECURITY] trebleManager` und folgenden Angaben:

- betroffene Version (`VERSION` / Commit),
- Schritte zum Reproduzieren,
- mögliche Auswirkung.

Antwort-Ziel: Bestätigung innerhalb 72 Stunden. Details werden erst nach Fix
veröffentlicht (coordinated disclosure).

### Unterstützte Versionen

| Version | Support |
|---|---|
| Neuestes Release (`VERSION`) | ✅ |
| Ältere | ❌ (nur best-effort) |

### Tool-Sicherheitsmodell

Dieses Tool verändert die Bootchain. Sicherheit schlägt Automatisierung:

- 9-Punkt-Safety-Gate vor jedem Flash (Modell, Partition, Image, Hash, Größe,
  Firmware-Kompat, Backup, Fastboot-Link, gepatcht `!=` Stock). Jedes FAIL →
  `DO NOT FLASH`.
- Flash/Restore brauchen Doppel-Bestätigung (`FLASH`+`YES` / `RESTORE`+`YES`,
  CLI: `--yes`).
- Backup des Original-Images ist Pflicht vor Flash (mit `metadata.json` + Hashes).
- Niemals automatisch: `erase/format userdata`, `flashing unlock`,
  Bootloader-Unlock, `system`/`vendor`/`userdata`-Flashs. Das GSI bleibt intakt.
- Niemals `--force`, `--disable-verity`, `--disable-verification` oder
  unbekannte Fastboot-Flags — weder im Code noch in Anleitungen. Ein
  verweigerndes Gerät ist ein Datum, nie ein Hindernis zum Durchzwingen.
- Keine Fake-Erfolgs-Pfade: Unmögliche Operationen liefern echte Fehler und
  Exit-Codes (Flash=1, Verify=2, Extract/Backup/Export=3,
  Download-Bestätigung-fehlt=4).
- Diagnose-ZIPs können gerätespezifische Werte enthalten (`getvar all`) —
  anonymisieren (`--anonymize`) oder Serials vor Teilen schwärzen.

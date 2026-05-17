# ValoTracker v$VERSION

Real-time VALORANT match tracker — ranks, stats, and encounter history for all 10 players, live.

---

## Installation

| Method | File |
|--------|------|
| **Wizard (recommended)** | `ValoTracker-Setup-$VERSION.exe` |
| **MSI (silent / enterprise)** | `ValoTracker-$VERSION-x86_64.msi` |
| **Portable TUI** | `ValoTracker.exe` |
| **Portable GUI** | `ValoTracker-gui.exe` |
| **pip** | `pip install valotracker==$VERSION` |

### Scoop
```powershell
scoop bucket add valotracker https://github.com/Londopy/ValoTracker
scoop install valotracker
```

---

## SHA-256 Checksums

```
$SHA_TUI  ValoTracker.exe
$SHA_GUI  ValoTracker-gui.exe
$SHA_MSI  ValoTracker-$VERSION-x86_64.msi
$SHA_WHL  valotracker-$VERSION-py3-none-win_amd64.whl
```

Verify before running:
```powershell
Get-FileHash .\ValoTracker.exe -Algorithm SHA256
```

---

## Requirements

- Windows 10 / 11 (x64)
- VALORANT installed and running

---

*ValoTracker reads only from `127.0.0.1`. No account login, no API keys, no data leaves your machine.*

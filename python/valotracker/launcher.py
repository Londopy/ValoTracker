"""
ValoTracker binary launchers.

These functions locate the pre-compiled Rust binaries bundled inside the
installed wheel and launch them as subprocesses, forwarding all CLI arguments.

Entry points registered in pyproject.toml:
  valotracker      →  valotracker.launcher:run_tui
  valotracker-gui  →  valotracker.launcher:run_gui
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


def run_tui() -> None:
    """Launch ValoTracker.exe (terminal UI) with forwarded sys.argv."""
    _launch("ValoTracker.exe")


def run_gui() -> None:
    """Launch ValoTracker-gui.exe (desktop GUI) with forwarded sys.argv."""
    _launch("ValoTracker-gui.exe")


def _launch(binary_name: str) -> None:
    """
    Locate *binary_name* in the ``bin/`` subdirectory next to this file,
    then exec it with the remaining ``sys.argv[1:]`` arguments.

    Exits with the subprocess return code, or with code 1 and a helpful
    message if the binary is missing.
    """
    bin_dir = Path(__file__).parent / "bin"
    binary = bin_dir / binary_name

    if not binary.exists():
        print(
            f"Error: {binary_name} not found in {bin_dir}\n"
            "The binary may be missing from your installation.\n"
            "Reinstall with:\n"
            "  pip install --force-reinstall valotracker",
            file=sys.stderr,
        )
        sys.exit(1)

    if sys.platform != "win32":
        print(
            f"Error: ValoTracker only supports Windows. "
            f"'{binary_name}' cannot run on {sys.platform}.",
            file=sys.stderr,
        )
        sys.exit(1)

    try:
        result = subprocess.run([str(binary)] + sys.argv[1:])
        sys.exit(result.returncode)
    except KeyboardInterrupt:
        # Clean exit on Ctrl-C — no traceback
        sys.exit(0)

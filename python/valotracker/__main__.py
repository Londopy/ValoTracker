"""
Allow ``python -m valotracker`` to launch the TUI binary.

Usage:
  python -m valotracker            # launch TUI
  python -m valotracker --help     # forward --help to the binary
"""

from valotracker.launcher import run_tui

if __name__ == "__main__":
    run_tui()

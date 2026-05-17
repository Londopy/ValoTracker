"""
Basic import and class-existence tests.

These tests do NOT require a running VALORANT instance — they only verify
that the Python layer is importable and structured correctly.
"""

import sys
import pytest


def test_module_importable():
    import valotracker  # noqa: F401


def test_vtclient_class_exists():
    from valotracker import ValoTrackerClient
    assert callable(ValoTrackerClient)


def test_pypplayer_class_exists():
    from valotracker import PyPlayer
    assert callable(PyPlayer)


def test_client_wrapper_importable():
    from valotracker.client import Client
    assert callable(Client)


def test_player_model_importable():
    from valotracker.models import Player
    assert callable(Player)


def test_client_instantiation_raises_without_valorant():
    """ValoTrackerClient.__init__ should raise RuntimeError when VALORANT is not running."""
    from valotracker import ValoTrackerClient
    with pytest.raises(RuntimeError):
        ValoTrackerClient()


# ── Launcher tests ────────────────────────────────────────────────────────────

def test_launcher_importable():
    """launcher.py must be importable without any side-effects."""
    import valotracker.launcher  # noqa: F401


def test_run_tui_callable():
    from valotracker.launcher import run_tui
    assert callable(run_tui)


def test_run_gui_callable():
    from valotracker.launcher import run_gui
    assert callable(run_gui)


def test_launch_internal_callable():
    from valotracker.launcher import _launch
    assert callable(_launch)


def test_main_importable():
    """__main__.py must be importable (it just calls run_tui at __main__ guard)."""
    # Import the module itself — the `if __name__ == '__main__'` guard prevents execution.
    import valotracker.__main__  # noqa: F401


def test_bin_directory_exists():
    """The bin/ subdirectory must exist inside the valotracker package."""
    from pathlib import Path
    import valotracker
    pkg_dir = Path(valotracker.__file__).parent
    bin_dir = pkg_dir / "bin"
    assert bin_dir.exists(), (
        f"bin/ directory not found at {bin_dir}. "
        "Run the CI pipeline or manually copy ValoTracker.exe and "
        "ValoTracker-gui.exe into python/valotracker/bin/ before testing."
    )


def test_missing_binary_prints_helpful_message(monkeypatch, capsys):
    """_launch() should print a clear error and exit(1) if the binary is missing."""
    from pathlib import Path
    from valotracker import launcher

    # Point _launch at a bin dir that definitely has no .exe files
    fake_bin = Path("/nonexistent/bin")
    monkeypatch.setattr(launcher, "__file__", str(fake_bin.parent / "launcher.py"))

    with pytest.raises(SystemExit) as exc_info:
        launcher._launch("ValoTracker.exe")

    assert exc_info.value.code == 1
    captured = capsys.readouterr()
    assert "ValoTracker.exe" in captured.err
    assert "pip install --force-reinstall valotracker" in captured.err


@pytest.mark.skipif(sys.platform != "win32", reason="Windows-only binary")
def test_tui_binary_is_executable():
    """On Windows, ValoTracker.exe must be present and non-zero in size."""
    from pathlib import Path
    import valotracker
    exe = Path(valotracker.__file__).parent / "bin" / "ValoTracker.exe"
    if not exe.exists():
        pytest.skip("ValoTracker.exe not staged yet — run CI first")
    assert exe.stat().st_size > 0, "ValoTracker.exe is empty"


@pytest.mark.skipif(sys.platform != "win32", reason="Windows-only binary")
def test_gui_binary_is_executable():
    """On Windows, ValoTracker-gui.exe must be present and non-zero in size."""
    from pathlib import Path
    import valotracker
    exe = Path(valotracker.__file__).parent / "bin" / "ValoTracker-gui.exe"
    if not exe.exists():
        pytest.skip("ValoTracker-gui.exe not staged yet — run CI first")
    assert exe.stat().st_size > 0, "ValoTracker-gui.exe is empty"

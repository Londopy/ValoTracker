"""
Basic import and class-existence tests.

These tests do NOT require a running VALORANT instance — they only verify
that the Python layer is importable and structured correctly.
"""

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

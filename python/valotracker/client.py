"""
High-level Python wrapper around the compiled :class:`ValoTrackerClient` extension type.
"""

from __future__ import annotations

from typing import List

from valotracker._valotracker import ValoTrackerClient as _ValoTrackerClient, PyPlayer


class Client:
    """Thin Python wrapper around the Rust-backed :class:`valotracker._valotracker.ValoTrackerClient`.

    All network calls are performed synchronously on an internal Tokio runtime.
    For async usage, run :meth:`get_players` inside a thread-pool executor.

    Examples
    --------
    >>> client = Client()
    >>> state = client.get_game_state()
    >>> print("Game state:", state)
    >>> players = client.get_players()
    >>> for p in players:
    ...     print(p)
    """

    def __init__(self) -> None:
        """Connect to the running VALORANT instance.

        Raises
        ------
        RuntimeError
            If VALORANT is not running or authentication fails.
        """
        self._inner = _ValoTrackerClient()

    def get_game_state(self) -> str:
        """Return the current game phase.

        Returns
        -------
        str
            One of ``"Menu"``, ``"Pregame"``, ``"Ingame"``, or
            ``"Disconnected"``.
        """
        return self._inner.get_game_state()

    def get_players(self) -> List[PyPlayer]:
        """Fetch all players in the current match.

        Works for both the agent-select (Pregame) and live (Ingame) phases.

        Returns
        -------
        List[PyPlayer]
            One entry per player in the match.

        Raises
        ------
        RuntimeError
            If the local player is not currently in a match.
        """
        return self._inner.get_players()

    def wait_for_match(self) -> None:
        """Block until the local player enters a match (Pregame or Ingame).

        Polls the game state every two seconds.  Returns immediately if the
        player is already in a match phase.
        """
        self._inner.wait_for_match()

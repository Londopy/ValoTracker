"""
Pure-Python :class:`Player` model that wraps a :class:`vt._vt.PyPlayer`.
"""

from __future__ import annotations

from vt._vt import PyPlayer


class Player:
    """A fully resolved VALORANT player.

    Wraps a :class:`PyPlayer` (the Rust extension type) and exposes every
    field as a regular Python property so that type checkers and IDEs can
    introspect them.

    Parameters
    ----------
    raw:
        The :class:`PyPlayer` instance returned by :meth:`vt.Client.get_players`.
    """

    def __init__(self, raw: PyPlayer) -> None:
        self._raw = raw

    # ── Identity ──────────────────────────────────────────────────────────────

    @property
    def name(self) -> str:
        """Display name, e.g. ``"SomePlayer#NA1"`` (or ``"[S]"`` in streamer mode)."""
        return self._raw.name

    @property
    def tag(self) -> str:
        """Tag line — the part after ``#``."""
        return self._raw.tag

    @property
    def incognito(self) -> bool:
        """``True`` if the player has streamer mode enabled."""
        return self._raw.incognito

    @property
    def account_level(self) -> int:
        """Account level (``0`` when hidden)."""
        return self._raw.account_level

    # ── Agent ─────────────────────────────────────────────────────────────────

    @property
    def agent(self) -> str:
        """Agent display name, e.g. ``"Jett"``."""
        return self._raw.agent

    # ── Rank ──────────────────────────────────────────────────────────────────

    @property
    def rank_tier(self) -> int:
        """Competitive tier index (0 = Unranked … 27 = Radiant)."""
        return self._raw.rank_tier

    @property
    def rank_name(self) -> str:
        """Human-readable rank name, e.g. ``"Gold 2"``."""
        return self._raw.rank_name

    @property
    def rr(self) -> int:
        """Current ranked rating."""
        return self._raw.rr

    @property
    def peak_tier(self) -> int:
        """All-time peak tier index."""
        return self._raw.peak_tier

    # ── Stats ─────────────────────────────────────────────────────────────────

    @property
    def headshot_pct(self) -> float:
        """Average headshot percentage over recent games (0.0–1.0)."""
        return self._raw.headshot_pct

    @property
    def kd_ratio(self) -> float:
        """Kill/death ratio over recent games."""
        return self._raw.kd_ratio

    @property
    def win_rate(self) -> float:
        """Win rate over recent games (0.0–1.0)."""
        return self._raw.win_rate

    # ── Team / Party ──────────────────────────────────────────────────────────

    @property
    def team(self) -> str:
        """Team ID: ``"Blue"`` (ally) or ``"Red"`` (enemy)."""
        return self._raw.team

    @property
    def is_ally(self) -> bool:
        """``True`` if the player is on the same team as the local player."""
        return self._raw.is_ally

    @property
    def party_id(self) -> str:
        """Opaque party identifier."""
        return self._raw.party_id

    @property
    def party_size(self) -> int:
        """Number of players in this player's party."""
        return self._raw.party_size

    @property
    def party_icon(self) -> str:
        """Single-character Unicode party icon."""
        return self._raw.party_icon

    # ── History ───────────────────────────────────────────────────────────────

    @property
    def times_seen(self) -> int:
        """Number of previous matches shared with the local player."""
        return self._raw.times_seen

    # ── Dunder ───────────────────────────────────────────────────────────────

    def __repr__(self) -> str:
        return (
            f"Player(name={self.name!r}, agent={self.agent!r}, "
            f"rank={self.rank_name!r}, team={self.team!r})"
        )

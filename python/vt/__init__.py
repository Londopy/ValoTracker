"""
vt — Valorant match tracker Python bindings.

Re-exports the core classes from the compiled Rust extension ``vt._vt``.
"""

from vt._vt import VtClient, PyPlayer  # noqa: F401

__all__ = ["VtClient", "PyPlayer"]

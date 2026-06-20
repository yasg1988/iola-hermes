"""``hermes postinstall`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_postinstall_parser(subparsers, *, cmd_postinstall: Callable) -> None:
    """Attach the ``postinstall`` subcommand to ``subparsers``."""
    # =========================================================================
    # postinstall command
    # =========================================================================
    postinstall_parser = subparsers.add_parser(
        "postinstall",
        help="Подготовить не-Python зависимости для pip-установки (node, browser, ripgrep, ffmpeg)",
        description="Одноразовый post-install для пользователей pip. Устанавливает "
        "системные зависимости, которые pip не может поставить, затем при необходимости запускает setup.",
    )
    postinstall_parser.set_defaults(func=cmd_postinstall)

"""``hermes logout`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_logout_parser(subparsers, *, cmd_logout: Callable) -> None:
    """Attach the ``logout`` subcommand to ``subparsers``."""
    # =========================================================================
    # logout command
    # =========================================================================
    logout_parser = subparsers.add_parser(
        "logout",
        help="Очистить авторизацию для inference-провайдера",
        description="Удалить сохраненные учетные данные и сбросить конфигурацию провайдера",
    )
    logout_parser.add_argument(
        "--provider",
        choices=["nous", "openai-codex", "xai-oauth", "spotify"],
        default=None,
        help="Провайдер для выхода (по умолчанию активный провайдер)",
    )
    logout_parser.set_defaults(func=cmd_logout)

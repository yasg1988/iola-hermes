"""``hermes acp`` subcommand parser.

Extracted from ``hermes_cli/main.py:main()`` (god-file Phase 2 follow-up).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable

from hermes_cli.subcommands._shared import add_accept_hooks_flag


def build_acp_parser(subparsers, *, cmd_acp: Callable) -> None:
    """Attach the ``acp`` subcommand to ``subparsers``."""
    acp_parser = subparsers.add_parser(
        "acp",
        help="Запустить Hermes RU Iola как ACP-сервер",
        description="Запустить Hermes RU Iola в ACP-режиме для интеграции с редакторами",
    )
    add_accept_hooks_flag(acp_parser)
    acp_parser.add_argument(
        "--version",
        action="store_true",
        dest="acp_version",
        help="Показать версию Hermes ACP и выйти",
    )
    acp_parser.add_argument(
        "--check",
        action="store_true",
        help="Проверить ACP-зависимости и импорты адаптера, затем выйти",
    )
    acp_parser.add_argument(
        "--setup",
        action="store_true",
        help="Запустить интерактивную настройку провайдера/модели для ACP",
    )
    acp_parser.add_argument(
        "--setup-browser",
        action="store_true",
        help="Установить agent-browser и Playwright Chromium для browser tools.",
    )
    acp_parser.add_argument(
        "--yes",
        "-y",
        action="store_true",
        dest="assume_yes",
        help="Автоматически принять все запросы подтверждения.",
    )
    acp_parser.set_defaults(func=cmd_acp)

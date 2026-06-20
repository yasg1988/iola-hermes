"""``hermes prompt-size`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_prompt_size_parser(subparsers, *, cmd_prompt_size: Callable) -> None:
    """Attach the ``prompt-size`` subcommand to ``subparsers``."""
    # =========================================================================
    # prompt-size command
    # =========================================================================
    prompt_size_parser = subparsers.add_parser(
        "prompt-size",
        help="Показать разбивку системного промпта и схем инструментов по байтам",
        description=(
            "Показывает фиксированный бюджет промпта для новой сессии: весь "
            "системный промпт, индекс навыков, память, профиль пользователя и "
            "JSON-схемы инструментов. Работает офлайн (без API-вызова)."
        ),
    )
    prompt_size_parser.add_argument(
        "--platform",
        default="cli",
        help="Платформа для симуляции (cli, telegram, discord, ...). По умолчанию: cli",
    )
    prompt_size_parser.add_argument(
        "--json",
        action="store_true",
        help="Вывести разбивку в JSON",
    )
    prompt_size_parser.set_defaults(func=cmd_prompt_size)

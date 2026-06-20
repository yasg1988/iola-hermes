"""``hermes insights`` subcommand parser.

Extracted from ``hermes_cli/main.py:main()`` (god-file Phase 2 follow-up).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_insights_parser(subparsers, *, cmd_insights: Callable) -> None:
    """Attach the ``insights`` subcommand to ``subparsers``."""
    insights_parser = subparsers.add_parser(
        "insights",
        help="Показать аналитику использования",
        description="Анализ истории сессий: токены, стоимость, паттерны инструментов и активность",
    )
    insights_parser.add_argument(
        "--days", type=int, default=30, help="Сколько дней анализировать (по умолчанию: 30)"
    )
    insights_parser.add_argument(
        "--source", help="Фильтр по платформе (cli, telegram, discord и т.п.)"
    )
    insights_parser.set_defaults(func=cmd_insights)

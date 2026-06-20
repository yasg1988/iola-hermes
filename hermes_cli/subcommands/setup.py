"""``hermes setup`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_setup_parser(subparsers, *, cmd_setup: Callable) -> None:
    """Attach the ``setup`` subcommand to ``subparsers``."""
    # =========================================================================
    # setup command
    # =========================================================================
    setup_parser = subparsers.add_parser(
        "setup",
        help="Интерактивный мастер настройки",
        description="Настроить Hermes RU Iola через интерактивный мастер. "
        "Можно запустить секцию: hermes setup model|tts|terminal|gateway|tools|agent",
    )
    setup_parser.add_argument(
        "section",
        nargs="?",
        choices=["model", "tts", "terminal", "gateway", "tools", "agent"],
        default=None,
        help="Запустить отдельную секцию настройки вместо полного мастера",
    )
    setup_parser.add_argument(
        "--non-interactive",
        action="store_true",
        help="Неинтерактивный режим: использовать значения по умолчанию и env vars",
    )
    setup_parser.add_argument(
        "--reset", action="store_true", help="Сбросить конфигурацию по умолчанию"
    )
    setup_parser.add_argument(
        "--reconfigure",
        action="store_true",
        help="Повторно запустить полный мастер с текущими значениями как defaults.",
    )
    setup_parser.add_argument(
        "--quick",
        action="store_true",
        help="На существующей установке спрашивать только отсутствующие значения.",
    )
    setup_parser.add_argument(
        "--portal",
        action="store_true",
        help="Быстрая настройка Nous Portal через OAuth, выбор модели и Tool Gateway.",
    )
    setup_parser.set_defaults(func=cmd_setup)

"""``hermes config`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_config_parser(subparsers, *, cmd_config: Callable) -> None:
    """Attach the ``config`` subcommand to ``subparsers``."""
    # =========================================================================
    # config command
    # =========================================================================
    config_parser = subparsers.add_parser(
        "config",
        help="Просмотр и редактирование конфигурации",
        description="Управление конфигурацией Hermes RU Iola",
    )
    config_subparsers = config_parser.add_subparsers(dest="config_command")

    # config show (default)
    config_subparsers.add_parser("show", help="Показать текущую конфигурацию")

    # config edit
    config_subparsers.add_parser("edit", help="Открыть config-файл в редакторе")

    # config set
    config_set = config_subparsers.add_parser("set", help="Задать значение конфигурации")
    config_set.add_argument(
        "key", nargs="?", help="Ключ конфигурации, например model или terminal.backend"
    )
    config_set.add_argument("value", nargs="?", help="Значение")

    # config path
    config_subparsers.add_parser("path", help="Вывести путь к config-файлу")

    # config env-path
    config_subparsers.add_parser("env-path", help="Вывести путь к .env-файлу")

    # config check
    config_subparsers.add_parser("check", help="Проверить отсутствующие или устаревшие настройки")

    # config migrate
    config_subparsers.add_parser("migrate", help="Обновить config новыми опциями")

    config_parser.set_defaults(func=cmd_config)

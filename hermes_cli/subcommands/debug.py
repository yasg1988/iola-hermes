"""``hermes debug`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

import argparse
from typing import Callable


def build_debug_parser(subparsers, *, cmd_debug: Callable) -> None:
    """Attach the ``debug`` subcommand to ``subparsers``."""
    # =========================================================================
    # debug command
    # =========================================================================
    debug_parser = subparsers.add_parser(
        "debug",
        help="Debug-инструменты: загрузить логи и сведения о системе",
        description="Debug-утилиты Hermes RU Iola. Используйте 'hermes debug share', "
        "чтобы загрузить debug-отчёт и получить ссылку.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""\
Примеры:
    hermes debug share              Загрузить debug-отчёт и вывести URL
    hermes debug share --lines 500  Включить больше строк логов
    hermes debug share --expire 30  Хранить paste 30 дней
    hermes debug share --local      Вывести отчёт локально без загрузки
    hermes debug share --no-redact  Отключить редактирование секретов
    hermes debug delete <url>       Удалить ранее загруженный paste
""",
    )
    debug_sub = debug_parser.add_subparsers(dest="debug_command")
    share_parser = debug_sub.add_parser(
        "share",
        help="Загрузить debug-отчёт в paste-сервис и вывести ссылку",
    )
    share_parser.add_argument(
        "--lines",
        type=int,
        default=200,
        help="Количество строк на каждый лог-файл",
    )
    share_parser.add_argument(
        "--expire",
        type=int,
        default=7,
        help="Срок хранения paste в днях",
    )
    share_parser.add_argument(
        "--local",
        action="store_true",
        help="Вывести отчёт локально вместо загрузки",
    )
    share_parser.add_argument(
        "--no-redact",
        action="store_true",
        help=(
            "Отключить редактирование секретов перед загрузкой. По умолчанию "
            "секреты удаляются из логов перед отправкой."
        ),
    )
    delete_parser = debug_sub.add_parser(
        "delete",
        help="Удалить paste, загруженный через 'hermes debug share'",
    )
    delete_parser.add_argument(
        "urls",
        nargs="*",
        default=[],
        help="Один или несколько paste URL для удаления",
    )
    debug_parser.set_defaults(func=cmd_debug)

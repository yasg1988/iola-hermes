"""``hermes update`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_update_parser(subparsers, *, cmd_update: Callable) -> None:
    """Attach the ``update`` subcommand to ``subparsers``."""
    # =========================================================================
    # update command
    # =========================================================================
    update_parser = subparsers.add_parser(
        "update",
        help="Обновить Hermes RU Iola до последней версии",
        description="Загрузить последние изменения из git и переустановить зависимости",
    )
    update_parser.add_argument(
        "--gateway",
        action="store_true",
        default=False,
        help="Gateway-режим: использовать файловый IPC вместо stdin",
    )
    update_parser.add_argument(
        "--check",
        action="store_true",
        default=False,
        help="Проверить наличие обновления без установки",
    )
    update_parser.add_argument(
        "--no-backup",
        action="store_true",
        default=False,
        help="Пропустить backup перед обновлением для этого запуска",
    )
    update_parser.add_argument(
        "--backup",
        action="store_true",
        default=False,
        help="Принудительно сделать backup перед обновлением",
    )
    update_parser.add_argument(
        "--yes",
        "-y",
        action="store_true",
        default=False,
        help="Автоматически отвечать yes на интерактивные запросы",
    )
    update_parser.add_argument(
        "--branch",
        default=None,
        metavar="NAME",
        help=(
            "Обновляться из указанной ветки вместо main. Если локальный checkout "
            "на другой ветке, Hermes сначала переключится на нужную ветку."
        ),
    )
    update_parser.add_argument(
        "--force",
        action="store_true",
        default=False,
        help="Windows: продолжить обновление, даже если обнаружен другой hermes.exe.",
    )
    update_parser.set_defaults(func=cmd_update)

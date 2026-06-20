"""``hermes backup`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_backup_parser(subparsers, *, cmd_backup: Callable) -> None:
    """Attach the ``backup`` subcommand to ``subparsers``."""
    # =========================================================================
    # backup command
    # =========================================================================
    backup_parser = subparsers.add_parser(
        "backup",
        help="Создать zip-резервную копию домашней папки Hermes",
        description="Создать zip-архив всей конфигурации Hermes, навыков, "
        "сессий и данных (без кодовой базы hermes-agent). "
        "Используйте --quick для быстрого снимка только критичных файлов состояния.",
    )
    backup_parser.add_argument(
        "-o",
        "--output",
        help="Путь для zip-файла (по умолчанию: ~/hermes-backup-<timestamp>.zip)",
    )
    backup_parser.add_argument(
        "-q",
        "--quick",
        action="store_true",
        help="Быстрый снимок: только критичные файлы состояния (config, state.db, .env, auth, cron)",
    )
    backup_parser.add_argument(
        "-l", "--label", help="Метка снимка (используется только с --quick)"
    )
    backup_parser.set_defaults(func=cmd_backup)

"""``hermes claw`` subcommand parser.

Extracted from ``hermes_cli/main.py:main()`` (god-file Phase 2 follow-up).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_claw_parser(subparsers, *, cmd_claw: Callable) -> None:
    """Attach the ``claw`` subcommand to ``subparsers``."""
    claw_parser = subparsers.add_parser(
        "claw",
        help="Инструменты миграции OpenClaw",
        description="Миграция настроек, памяти, навыков и API-ключей из OpenClaw в Hermes",
    )
    claw_subparsers = claw_parser.add_subparsers(dest="claw_action")

    # claw migrate
    claw_migrate = claw_subparsers.add_parser(
        "migrate",
        help="Мигрировать из OpenClaw в Hermes",
        description="Импортировать настройки, память, навыки и API-ключи из установки OpenClaw. "
        "Всегда показывает preview перед изменениями.",
    )
    claw_migrate.add_argument(
        "--source", help="Путь к папке OpenClaw (по умолчанию: ~/.openclaw)"
    )
    claw_migrate.add_argument(
        "--dry-run",
        action="store_true",
        help="Только preview: остановиться после показа плана миграции",
    )
    claw_migrate.add_argument(
        "--preset",
        choices=["user-data", "full"],
        default="full",
        help="Пресет миграции (по умолчанию: full). Ни один пресет не импортирует secrets: "
        "передайте --migrate-secrets, чтобы включить API-ключи.",
    )
    claw_migrate.add_argument(
        "--overwrite",
        action="store_true",
        help="Перезаписать существующие файлы (по умолчанию план с конфликтами не применяется)",
    )
    claw_migrate.add_argument(
        "--migrate-secrets",
        action="store_true",
        help="Включить allowlisted secrets (TELEGRAM_BOT_TOKEN, API-ключи и т.п.). "
        "Требуется даже при --preset full.",
    )
    claw_migrate.add_argument(
        "--no-backup",
        action="store_true",
        help="Пропустить zip-снимок ~/.hermes/ перед миграцией (по умолчанию "
        "перед применением создается единый restore-point archive в ~/.hermes/backups/; "
        "восстанавливается через 'hermes import').",
    )
    claw_migrate.add_argument(
        "--workspace-target", help="Абсолютный путь, куда копировать инструкции workspace"
    )
    claw_migrate.add_argument(
        "--skill-conflict",
        choices=["skip", "overwrite", "rename"],
        default="skip",
        help="Как обрабатывать конфликты имен навыков (по умолчанию: skip)",
    )
    claw_migrate.add_argument(
        "--yes", "-y", action="store_true", help="Пропустить подтверждения"
    )

    # claw cleanup
    claw_cleanup = claw_subparsers.add_parser(
        "cleanup",
        aliases=["clean"],
        help="Архивировать оставшиеся папки OpenClaw после миграции",
        description="Найти и архивировать оставшиеся папки OpenClaw, чтобы избежать фрагментации состояния",
    )
    claw_cleanup.add_argument(
        "--source", help="Путь к конкретной папке OpenClaw для очистки"
    )
    claw_cleanup.add_argument(
        "--dry-run",
        action="store_true",
        help="Показать, что будет архивировано, без внесения изменений",
    )
    claw_cleanup.add_argument(
        "--yes", "-y", action="store_true", help="Пропустить подтверждения"
    )
    claw_parser.set_defaults(func=cmd_claw)

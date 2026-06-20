"""``hermes uninstall`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_uninstall_parser(subparsers, *, cmd_uninstall: Callable) -> None:
    """Attach the ``uninstall`` subcommand to ``subparsers``."""
    # =========================================================================
    # uninstall command
    # =========================================================================
    uninstall_parser = subparsers.add_parser(
        "uninstall",
        help="Удалить Hermes RU Iola",
        description="Удалить Hermes RU Iola из системы. Конфиги и данные можно сохранить для повторной установки.",
    )
    uninstall_parser.add_argument(
        "--full",
        action="store_true",
        help="Полное удаление: удалить всё, включая конфиги и данные",
    )
    uninstall_parser.add_argument(
        "--gui",
        action="store_true",
        help="Удалить только desktop GUI, оставив агента",
    )
    uninstall_parser.add_argument(
        "--gui-summary",
        action="store_true",
        help="Вывести JSON-сводку установленных GUI/agent артефактов и выйти",
    )
    uninstall_parser.add_argument(
        "--yes", "-y", action="store_true", help="Пропустить подтверждения"
    )
    uninstall_parser.set_defaults(func=cmd_uninstall)

"""``hermes dump`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_dump_parser(subparsers, *, cmd_dump: Callable) -> None:
    """Attach the ``dump`` subcommand to ``subparsers``."""
    # =========================================================================
    # dump command
    # =========================================================================
    dump_parser = subparsers.add_parser(
        "dump",
        help="Вывести сводку установки для поддержки/отладки",
        description="Вывести компактную plain-text сводку настройки Hermes, "
        "которую можно вставить в Discord/GitHub для контекста поддержки",
    )
    dump_parser.add_argument(
        "--show-keys",
        action="store_true",
        help="Показать замаскированные префиксы API-ключей (первые/последние 4 символа) вместо set/not set",
    )
    dump_parser.set_defaults(func=cmd_dump)

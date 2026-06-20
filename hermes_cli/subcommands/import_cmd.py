"""``hermes import`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_import_cmd_parser(subparsers, *, cmd_import: Callable) -> None:
    """Attach the ``import`` subcommand to ``subparsers``."""
    # =========================================================================
    # import command
    # =========================================================================
    import_parser = subparsers.add_parser(
        "import",
        help="Восстановить резервную копию Hermes из zip-файла",
        description="Распаковать ранее созданную резервную копию Hermes в "
        "домашнюю папку Hermes, восстановив конфигурацию, навыки, "
        "сессии и данные",
    )
    import_parser.add_argument("zipfile", help="Путь к zip-файлу резервной копии")
    import_parser.add_argument(
        "--force",
        "-f",
        action="store_true",
        help="Перезаписывать существующие файлы без подтверждения",
    )
    import_parser.set_defaults(func=cmd_import)

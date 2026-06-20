"""``hermes gui`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_gui_parser(subparsers, *, cmd_gui: Callable) -> None:
    """Attach the ``gui`` subcommand to ``subparsers``."""
    # =========================================================================
    gui_parser = subparsers.add_parser(
        "desktop",
        aliases=["gui"],
        help="Собрать и запустить нативное desktop-приложение",
        description=(
            "Запустить desktop-приложение Hermes на Electron. По умолчанию "
            "устанавливает Node-зависимости workspace, собирает unpacked "
            "Electron-приложение для текущей ОС и запускает собранный артефакт."
        ),
    )
    gui_parser.add_argument(
        "--source",
        action="store_true",
        help="Запустить через `electron .` из apps/desktop/dist вместо packaged app",
    )
    gui_parser.add_argument(
        "--build-only",
        action="store_true",
        help="Собрать desktop-приложение, но не запускать (для installer's --update flow)",
    )
    gui_parser.add_argument(
        "--fake-boot",
        action="store_true",
        help="Включить детерминированные задержки старта для проверки startup UI",
    )
    gui_parser.add_argument(
        "--ignore-existing",
        action="store_true",
        help="Заставить Desktop игнорировать hermes CLI из PATH при выборе backend",
    )
    gui_parser.add_argument(
        "--hermes-root",
        help="Переопределить корень исходников Hermes для Desktop (задает HERMES_DESKTOP_HERMES_ROOT)",
    )
    gui_parser.add_argument(
        "--cwd",
        help="Начальная папка проекта для desktop-сессий чата (задает HERMES_DESKTOP_CWD)",
    )
    gui_parser.add_argument(
        "--skip-build",
        action="store_true",
        help="Пропустить npm install/package и запустить существующее unpacked app из apps/desktop/release",
    )
    gui_parser.add_argument(
        "--force-build",
        action="store_true",
        help="Принудительно выполнить полную пересборку, даже если content stamp совпадает",
    )
    gui_parser.set_defaults(func=cmd_gui)

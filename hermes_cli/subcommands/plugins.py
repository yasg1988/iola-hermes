"""``hermes plugins`` subcommand parser.

Extracted from ``hermes_cli/main.py:main()`` (god-file Phase 2 follow-up).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_plugins_parser(subparsers, *, cmd_plugins: Callable) -> None:
    """Attach the ``plugins`` subcommand to ``subparsers``."""
    plugins_parser = subparsers.add_parser(
        "plugins",
        help="Управление плагинами: установка, обновление, удаление, список",
        description="Установка плагинов из Git-репозиториев, обновление, удаление и просмотр.",
    )
    plugins_subparsers = plugins_parser.add_subparsers(dest="plugins_action")

    plugins_install = plugins_subparsers.add_parser(
        "install", help="Установить плагин из Git URL или owner/repo"
    )
    plugins_install.add_argument(
        "identifier",
        help="Git URL или короткая форма owner/repo (например anpicasso/hermes-plugin-chrome-profiles)",
    )
    plugins_install.add_argument(
        "--force",
        "-f",
        action="store_true",
        help="Удалить существующий плагин и установить заново",
    )
    _install_enable_group = plugins_install.add_mutually_exclusive_group()
    _install_enable_group.add_argument(
        "--enable",
        action="store_true",
        help="Автоматически включить плагин после установки (без подтверждения)",
    )
    _install_enable_group.add_argument(
        "--no-enable",
        action="store_true",
        help="Установить отключенным (без подтверждения); включить позже через `hermes plugins enable <name>`",
    )

    plugins_update = plugins_subparsers.add_parser(
        "update", help="Скачать последние изменения установленного плагина"
    )
    plugins_update.add_argument("name", help="Имя плагина для обновления")

    plugins_remove = plugins_subparsers.add_parser(
        "remove", aliases=["rm", "uninstall"], help="Удалить установленный плагин"
    )
    plugins_remove.add_argument("name", help="Имя папки плагина для удаления")

    plugins_list = plugins_subparsers.add_parser(
        "list", aliases=["ls"], help="Показать установленные плагины"
    )
    plugins_list.add_argument(
        "--enabled",
        action="store_true",
        help="Показать только включенные плагины",
    )
    plugins_list.add_argument(
        "--user",
        action="store_true",
        help="Показать только пользовательские плагины (включая git-плагины)",
    )
    plugins_list.add_argument(
        "--no-bundled",
        action="store_true",
        help="Скрыть встроенные плагины",
    )
    plugins_list.add_argument(
        "--plain",
        action="store_true",
        help="Вывести компактный plain-text вместо Rich-таблицы",
    )
    plugins_list.add_argument(
        "--json",
        action="store_true",
        help="Вывести машинно-читаемый JSON",
    )

    plugins_enable = plugins_subparsers.add_parser(
        "enable", help="Включить отключенный плагин"
    )
    plugins_enable.add_argument("name", help="Имя плагина для включения")

    plugins_disable = plugins_subparsers.add_parser(
        "disable", help="Отключить плагин без удаления"
    )
    plugins_disable.add_argument("name", help="Имя плагина для отключения")
    plugins_parser.set_defaults(func=cmd_plugins)

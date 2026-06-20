"""``hermes mcp`` subcommand parser.

Extracted from ``hermes_cli/main.py:main()`` (god-file Phase 2 follow-up).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

import argparse
from typing import Callable

from hermes_cli.subcommands._shared import add_accept_hooks_flag


def build_mcp_parser(subparsers, *, cmd_mcp: Callable) -> None:
    """Attach the ``mcp`` subcommand to ``subparsers``."""
    mcp_parser = subparsers.add_parser(
        "mcp",
        help="Управлять MCP-серверами и запускать Hermes как MCP-сервер",
        description=(
            "Управление подключениями MCP-серверов и запуск Hermes как MCP-сервера.\n\n"
            "MCP-серверы предоставляют дополнительные инструменты через Model Context Protocol.\n"
            "Используйте 'hermes mcp add' для подключения нового сервера или\n"
            "'hermes mcp serve', чтобы открыть разговоры Hermes через MCP."
        ),
    )
    mcp_sub = mcp_parser.add_subparsers(dest="mcp_action")

    mcp_serve_p = mcp_sub.add_parser(
        "serve",
        help="Запустить Hermes как MCP-сервер (открыть разговоры другим агентам)",
    )
    mcp_serve_p.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Включить подробное логирование в stderr",
    )
    add_accept_hooks_flag(mcp_serve_p)

    mcp_add_p = mcp_sub.add_parser(
        "add", help="Добавить MCP-сервер (установка с discovery-first)"
    )
    mcp_add_p.add_argument("name", help="Имя сервера (используется как ключ config)")
    mcp_add_p.add_argument("--url", help="URL endpoint HTTP/SSE")
    # dest="mcp_command" so this flag does not clobber the top-level
    # subparser's args.command attribute, which the dispatcher reads to
    # route to cmd_mcp.  Without an explicit dest, argparse derives
    # dest="command" from the flag name and sets it to None when the
    # flag is omitted, causing `hermes mcp add ...` to fall through to
    # interactive chat.
    mcp_add_p.add_argument(
        "--command", dest="mcp_command", help="Stdio-команда (например npx)"
    )
    mcp_add_p.add_argument(
        "--args",
        nargs=argparse.REMAINDER,
        default=[],
        help="Аргументы stdio-команды; должны идти последней опцией",
    )
    mcp_add_p.add_argument("--auth", choices=["oauth", "header"], help="Метод авторизации")
    mcp_add_p.add_argument("--preset", help="Имя известного MCP-пресета")
    mcp_add_p.add_argument(
        "--env",
        nargs="*",
        default=[],
        help="Переменные окружения для stdio-серверов (KEY=VALUE)",
    )

    mcp_rm_p = mcp_sub.add_parser("remove", aliases=["rm"], help="Удалить MCP-сервер")
    mcp_rm_p.add_argument("name", help="Имя сервера для удаления")

    mcp_sub.add_parser("list", aliases=["ls"], help="Показать настроенные MCP-серверы")

    mcp_test_p = mcp_sub.add_parser("test", help="Проверить подключение к MCP-серверу")
    mcp_test_p.add_argument("name", help="Имя сервера для проверки")

    mcp_cfg_p = mcp_sub.add_parser(
        "configure", aliases=["config"], help="Настроить выбор инструментов"
    )
    mcp_cfg_p.add_argument("name", help="Имя сервера для настройки")

    mcp_login_p = mcp_sub.add_parser(
        "login",
        help="Принудительно переавторизовать OAuth-based MCP-сервер",
    )
    mcp_login_p.add_argument("name", help="Имя сервера для переавторизации")

    # ── Catalog (Nous-approved MCPs shipped with the repo) ─────────────────
    mcp_sub.add_parser(
        "picker",
        help="Интерактивный выбор из каталога (также по умолчанию для `hermes mcp`)",
    )
    mcp_sub.add_parser(
        "catalog",
        help="Показать одобренные Nous MCP для установки в один клик",
    )
    mcp_install_p = mcp_sub.add_parser(
        "install",
        help="Установить MCP из каталога по имени (например `hermes mcp install n8n`)",
    )
    mcp_install_p.add_argument(
        "identifier",
        help="Имя записи каталога (или `official/<name>`)",
    )

    add_accept_hooks_flag(mcp_parser)
    mcp_parser.set_defaults(func=cmd_mcp)

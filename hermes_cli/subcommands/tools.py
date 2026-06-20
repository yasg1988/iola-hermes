"""``hermes tools`` subcommand parser.

Extracted from ``hermes_cli/main.py:main()`` (god-file Phase 2 follow-up).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_tools_parser(subparsers, *, cmd_tools: Callable) -> None:
    """Attach the ``tools`` subcommand to ``subparsers``."""
    tools_parser = subparsers.add_parser(
        "tools",
        help="Настроить включенные инструменты для каждой платформы",
        description=(
            "Включение, отключение и просмотр инструментов для CLI, Telegram, Discord и т.п.\n\n"
            "Встроенные наборы инструментов используют простые имена (например web, memory).\n"
            "MCP-инструменты используют формат server:tool (например github:create_issue).\n\n"
            "Запустите 'hermes tools' без подкоманды для интерактивной настройки."
        ),
    )
    tools_parser.add_argument(
        "--summary",
        action="store_true",
        help="Показать сводку включенных инструментов по платформам и выйти",
    )
    tools_sub = tools_parser.add_subparsers(dest="tools_action")

    # hermes tools list [--platform cli]
    tools_list_p = tools_sub.add_parser(
        "list",
        help="Показать все инструменты и их статус включения",
    )
    tools_list_p.add_argument(
        "--platform",
        default="cli",
        help="Платформа для просмотра (по умолчанию: cli)",
    )

    # hermes tools disable <name...> [--platform cli]
    tools_disable_p = tools_sub.add_parser(
        "disable",
        help="Отключить наборы инструментов или MCP-инструменты",
    )
    tools_disable_p.add_argument(
        "names",
        nargs="+",
        metavar="NAME",
        help="Имя набора инструментов (например web) или MCP-инструмент в формате server:tool",
    )
    tools_disable_p.add_argument(
        "--platform",
        default="cli",
        help="Платформа для применения (по умолчанию: cli)",
    )

    # hermes tools enable <name...> [--platform cli]
    tools_enable_p = tools_sub.add_parser(
        "enable",
        help="Включить наборы инструментов или MCP-инструменты",
    )
    tools_enable_p.add_argument(
        "names",
        nargs="+",
        metavar="NAME",
        help="Имя набора инструментов или MCP-инструмент в формате server:tool",
    )
    tools_enable_p.add_argument(
        "--platform",
        default="cli",
        help="Платформа для применения (по умолчанию: cli)",
    )

    # hermes tools post-setup <key>
    tools_postsetup_p = tools_sub.add_parser(
        "post-setup",
        help="Запустить post-setup install hook провайдера (npm/pip/binary)",
        description=(
            "Запустить install/bootstrap hook, объявленный backend инструмента,\n"
            "тот же шаг, который `hermes tools` выполняет после выбора провайдера\n"
            "с дополнительными зависимостями (browser Chromium, Camofox, cua-driver,\n"
            "KittenTTS/Piper, ddgs, Spotify, Langfuse, xAI). Стабильная\n"
            "неинтерактивная цель для dashboard, который настраивает backend.\n"
            "Ключи: agent_browser, camofox, cua_driver, kittentts,\n"
            "piper, ddgs, spotify, langfuse, xai_grok."
        ),
    )
    tools_postsetup_p.add_argument(
        "post_setup_key",
        metavar="KEY",
        help="Ключ post-setup hook (например agent_browser, camofox, kittentts)",
    )
    tools_parser.set_defaults(func=cmd_tools)

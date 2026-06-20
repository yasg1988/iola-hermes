"""``hermes logs`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

import argparse
from typing import Callable


def build_logs_parser(subparsers, *, cmd_logs: Callable) -> None:
    """Attach the ``logs`` subcommand to ``subparsers``."""
    # =========================================================================
    # logs command
    # =========================================================================
    logs_parser = subparsers.add_parser(
        "logs",
        help="Просмотр и фильтрация логов Hermes",
        description="Просмотр, tail и фильтрация agent.log / errors.log / gateway.log / gui.log / desktop.log",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""\
Примеры:
    hermes logs                    Последние 50 строк agent.log
    hermes logs -f                 Читать agent.log в реальном времени
    hermes logs errors             Последние 50 строк errors.log
    hermes logs gateway -n 100     Последние 100 строк gateway.log
    hermes logs gui -f             Читать gui.log в реальном времени
    hermes logs desktop -f         Читать desktop.log
    hermes logs --level WARNING    Только WARNING и выше
    hermes logs --session abc123   Фильтр по ID сессии
    hermes logs --component tools  Только строки компонента tools
    hermes logs --since 1h         Строки за последний час
    hermes logs list               Список доступных логов и размеры
""",
    )
    logs_parser.add_argument(
        "log_name",
        nargs="?",
        default="agent",
        help="Лог для просмотра: agent, errors, gateway, gui или list",
    )
    logs_parser.add_argument(
        "-n",
        "--lines",
        type=int,
        default=50,
        help="Количество строк для показа",
    )
    logs_parser.add_argument(
        "-f",
        "--follow",
        action="store_true",
        help="Читать лог в реальном времени",
    )
    logs_parser.add_argument(
        "--level",
        metavar="LEVEL",
        help="Минимальный уровень лога (DEBUG, INFO, WARNING, ERROR)",
    )
    logs_parser.add_argument(
        "--session",
        metavar="ID",
        help="Фильтровать строки по ID сессии",
    )
    logs_parser.add_argument(
        "--since",
        metavar="TIME",
        help="Показать строки за период, например 1h, 30m, 2d",
    )
    logs_parser.add_argument(
        "--component",
        metavar="NAME",
        help="Фильтр по компоненту: gateway, agent, tools, cli, cron, gui",
    )
    logs_parser.set_defaults(func=cmd_logs)

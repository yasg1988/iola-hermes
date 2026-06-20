"""``hermes memory`` subcommand parser.

Extracted from ``hermes_cli/main.py:main()`` (god-file Phase 2 follow-up).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_memory_parser(subparsers, *, cmd_memory: Callable) -> None:
    """Attach the ``memory`` subcommand to ``subparsers``."""
    memory_parser = subparsers.add_parser(
        "memory",
        help="Настроить внешнего провайдера памяти",
        description=(
            "Настройка и управление плагинами внешних провайдеров памяти.\n\n"
            "Доступные провайдеры: honcho, openviking, mem0, hindsight,\n"
            "holographic, retaindb, byterover.\n\n"
            "Одновременно может быть активен только один внешний провайдер.\n"
            "Встроенная память (MEMORY.md/USER.md) всегда активна."
        ),
    )
    memory_sub = memory_parser.add_subparsers(dest="memory_command")
    _setup_parser = memory_sub.add_parser(
        "setup", help="Интерактивный выбор и настройка провайдера"
    )
    _setup_parser.add_argument(
        "provider",
        nargs="?",
        default=None,
        help="Провайдер для прямой настройки (например honcho), без интерактивного выбора",
    )
    memory_sub.add_parser("status", help="Показать текущую конфигурацию провайдера памяти")
    memory_sub.add_parser("off", help="Отключить внешнего провайдера (оставить встроенную память)")
    _reset_parser = memory_sub.add_parser(
        "reset",
        help="Очистить всю встроенную память (MEMORY.md и USER.md)",
    )
    _reset_parser.add_argument(
        "--yes",
        "-y",
        action="store_true",
        help="Пропустить подтверждение",
    )
    _reset_parser.add_argument(
        "--target",
        choices=["all", "memory", "user"],
        default="all",
        help="Что сбросить: 'all' (по умолчанию), 'memory' или 'user'",
    )
    memory_parser.set_defaults(func=cmd_memory)

"""``hermes security`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_security_parser(subparsers, *, cmd_security: Callable) -> None:
    """Attach the ``security`` subcommand to ``subparsers``."""
    # =========================================================================
    security_parser = subparsers.add_parser(
        "security",
        help="Supply-chain аудит (OSV.dev) для venv, плагинов и MCP-серверов",
        description=(
            "Проверка уязвимостей через OSV.dev по требованию. Охватывает Hermes "
            "venv (установленные PyPI-пакеты), Python-зависимости плагинов из "
            "~/.hermes/plugins/ и закрепленные npx/uvx MCP-серверы в config.yaml. "
            "Не сканирует глобально установленные пакеты и расширения редактора/браузера."
        ),
    )
    security_subparsers = security_parser.add_subparsers(
        dest="security_command",
        metavar="<subcommand>",
    )

    audit_parser = security_subparsers.add_parser(
        "audit",
        help="Запустить одноразовый supply-chain аудит",
        description="Запросить OSV.dev на известные уязвимости в установленных компонентах.",
    )
    audit_parser.add_argument(
        "--json",
        action="store_true",
        help="Вывести машинно-читаемый JSON вместо текста для человека",
    )
    audit_parser.add_argument(
        "--fail-on",
        default="critical",
        choices=["low", "moderate", "high", "critical"],
        help="Завершиться с ненулевым кодом, если найдена уязвимость этого уровня (по умолчанию: critical)",
    )
    audit_parser.add_argument(
        "--skip-venv",
        action="store_true",
        help="Пропустить сканирование Python venv Hermes",
    )
    audit_parser.add_argument(
        "--skip-plugins",
        action="store_true",
        help="Пропустить сканирование requirements-файлов плагинов",
    )
    audit_parser.add_argument(
        "--skip-mcp",
        action="store_true",
        help="Пропустить сканирование закрепленных MCP-серверов в config.yaml",
    )
    audit_parser.set_defaults(func=cmd_security)
    security_parser.set_defaults(func=cmd_security)

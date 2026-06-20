"""``hermes doctor`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_doctor_parser(subparsers, *, cmd_doctor: Callable) -> None:
    """Attach the ``doctor`` subcommand to ``subparsers``."""
    # =========================================================================
    # doctor command
    # =========================================================================
    doctor_parser = subparsers.add_parser(
        "doctor",
        help="Проверить конфигурацию и зависимости",
        description="Диагностика проблем установки Hermes RU Iola",
    )
    doctor_parser.add_argument(
        "--fix", action="store_true", help="Попытаться исправить проблемы автоматически"
    )
    doctor_parser.add_argument(
        "--ack",
        metavar="ADVISORY_ID",
        default=None,
        help=(
            "Подтвердить security advisory по ID и выйти. После ack предупреждение "
            "не будет показывать startup banner."
        ),
    )
    doctor_parser.set_defaults(func=cmd_doctor)

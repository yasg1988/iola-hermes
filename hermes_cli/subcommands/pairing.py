"""``hermes pairing`` subcommand parser.

Extracted from ``hermes_cli/main.py:main()`` (god-file Phase 2 follow-up).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_pairing_parser(subparsers, *, cmd_pairing: Callable) -> None:
    """Attach the ``pairing`` subcommand to ``subparsers``."""
    pairing_parser = subparsers.add_parser(
        "pairing",
        help="Управление pairing-кодами DM для авторизации пользователей",
        description="Подтверждение или отзыв доступа пользователей через pairing-коды",
    )
    pairing_sub = pairing_parser.add_subparsers(dest="pairing_action")

    pairing_sub.add_parser("list", help="Показать ожидающих и одобренных пользователей")

    pairing_approve_parser = pairing_sub.add_parser(
        "approve", help="Одобрить pairing-код"
    )
    pairing_approve_parser.add_argument(
        "platform", help="Имя платформы (telegram, discord, slack, whatsapp)"
    )
    pairing_approve_parser.add_argument("code", help="Pairing-код для одобрения")

    pairing_revoke_parser = pairing_sub.add_parser("revoke", help="Отозвать доступ пользователя")
    pairing_revoke_parser.add_argument("platform", help="Имя платформы")
    pairing_revoke_parser.add_argument("user_id", help="ID пользователя для отзыва")

    pairing_sub.add_parser("clear-pending", help="Очистить все ожидающие коды")
    pairing_parser.set_defaults(func=cmd_pairing)

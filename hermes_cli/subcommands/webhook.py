"""``hermes webhook`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_webhook_parser(subparsers, *, cmd_webhook: Callable) -> None:
    """Attach the ``webhook`` subcommand to ``subparsers``."""
    # =========================================================================
    # webhook command
    # =========================================================================
    webhook_parser = subparsers.add_parser(
        "webhook",
        help="Управление динамическими webhook-подписками",
        description="Создание, просмотр и удаление webhook-подписок для событийного запуска агента",
    )
    webhook_subparsers = webhook_parser.add_subparsers(dest="webhook_action")

    wh_sub = webhook_subparsers.add_parser(
        "subscribe", aliases=["add"], help="Создать webhook-подписку"
    )
    wh_sub.add_argument("name", help="Имя маршрута (используется в URL: /webhooks/<name>)")
    wh_sub.add_argument(
        "--prompt", default="", help="Шаблон промпта со ссылками на payload через {dot.notation}"
    )
    wh_sub.add_argument(
        "--events", default="", help="Типы событий через запятую"
    )
    wh_sub.add_argument("--description", default="", help="Что делает эта подписка")
    wh_sub.add_argument(
        "--skills", default="", help="Имена навыков для загрузки через запятую"
    )
    wh_sub.add_argument(
        "--deliver",
        default="log",
        help="Куда доставлять результат: log, telegram, discord, slack и т.п.",
    )
    wh_sub.add_argument(
        "--deliver-chat-id",
        default="",
        help="ID целевого чата для кросс-платформенной доставки",
    )
    wh_sub.add_argument(
        "--secret", default="", help="HMAC secret (будет создан автоматически, если не указан)"
    )
    wh_sub.add_argument(
        "--deliver-only",
        action="store_true",
        help="Пропустить агента: доставить отрендеренный промпт напрямую как "
        "сообщение. Без затрат LLM. Требует реальную цель --deliver "
        "(не 'log').",
    )

    webhook_subparsers.add_parser(
        "list", aliases=["ls"], help="Показать все динамические подписки"
    )

    wh_rm = webhook_subparsers.add_parser(
        "remove", aliases=["rm"], help="Удалить подписку"
    )
    wh_rm.add_argument("name", help="Имя подписки для удаления")

    wh_test = webhook_subparsers.add_parser(
        "test", help="Отправить тестовый POST в webhook-маршрут"
    )
    wh_test.add_argument("name", help="Имя подписки для проверки")
    wh_test.add_argument(
        "--payload", default="", help="JSON payload для отправки (по умолчанию тестовый)"
    )

    webhook_parser.set_defaults(func=cmd_webhook)

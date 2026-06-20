"""``hermes model`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_model_parser(subparsers, *, cmd_model: Callable) -> None:
    """Attach the ``model`` subcommand to ``subparsers``."""
    # =========================================================================
    # model command
    # =========================================================================
    model_parser = subparsers.add_parser(
        "model",
        help="Выбрать модель и провайдера по умолчанию",
        description="Интерактивно выбрать провайдера инференса и модель по умолчанию",
    )
    model_parser.add_argument(
        "--refresh",
        action="store_true",
        help="Очистить кэш picker моделей и заново загрузить /v1/models у провайдеров.",
    )
    model_parser.add_argument(
        "--portal-url",
        help="Базовый URL портала для входа Nous",
    )
    model_parser.add_argument(
        "--inference-url",
        help="Базовый URL inference API для входа Nous",
    )
    model_parser.add_argument(
        "--client-id",
        default=None,
        help="OAuth client id для входа Nous",
    )
    model_parser.add_argument(
        "--scope", default=None, help="OAuth scope для входа Nous"
    )
    model_parser.add_argument(
        "--no-browser",
        action="store_true",
        help="Не открывать браузер автоматически во время входа Nous",
    )
    model_parser.add_argument(
        "--manual-paste",
        action="store_true",
        help=(
            "Для loopback OAuth провайдеров: не запускать локальный callback listener, "
            "а вставить callback URL из браузера вручную."
        ),
    )
    model_parser.add_argument(
        "--timeout",
        type=float,
        default=15.0,
        help="Таймаут HTTP-запросов для входа Nous в секундах",
    )
    model_parser.add_argument("--ca-bundle", help="Путь к PEM CA bundle для TLS-проверки Nous")
    model_parser.add_argument(
        "--insecure",
        action="store_true",
        help="Отключить TLS-проверку для входа Nous (только для тестов)",
    )
    model_parser.set_defaults(func=cmd_model)

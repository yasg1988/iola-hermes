"""``hermes auth`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_auth_parser(subparsers, *, cmd_auth: Callable) -> None:
    """Attach the ``auth`` subcommand to ``subparsers``."""
    auth_parser = subparsers.add_parser(
        "auth",
        help="Управление набором учетных данных провайдеров",
    )
    auth_subparsers = auth_parser.add_subparsers(dest="auth_action")
    auth_add = auth_subparsers.add_parser("add", help="Добавить учетные данные")
    auth_add.add_argument(
        "provider",
        help="ID провайдера (например: anthropic, openai-codex, openrouter)",
    )
    auth_add.add_argument(
        "--type",
        dest="auth_type",
        choices=["oauth", "api-key", "api_key"],
        help="Тип учетных данных",
    )
    auth_add.add_argument("--label", help="Необязательная метка для отображения")
    auth_add.add_argument(
        "--api-key", help="Значение API-ключа (иначе будет запрошено безопасно)"
    )
    auth_add.add_argument("--portal-url", help="Базовый URL портала Nous")
    auth_add.add_argument("--inference-url", help="Базовый URL inference API Nous")
    auth_add.add_argument("--client-id", help="OAuth client id")
    auth_add.add_argument("--scope", help="Переопределить OAuth scope")
    auth_add.add_argument(
        "--no-browser",
        action="store_true",
        help="Не открывать браузер автоматически для OAuth-входа",
    )
    auth_add.add_argument(
        "--manual-paste",
        action="store_true",
        help=(
            "Не запускать локальный callback-listener; вместо этого вставить "
            "URL неудачного callback из браузера. Используйте на удаленных "
            "средах только с браузером (GCP Cloud Shell, GitHub Codespaces, "
            "EC2 Instance Connect и т.п.), где 127.0.0.1 удаленной машины "
            "недоступен с вашего ноутбука. См. #26923."
        ),
    )
    auth_add.add_argument(
        "--timeout", type=float, help="Таймаут OAuth/сети в секундах"
    )
    auth_add.add_argument(
        "--insecure",
        action="store_true",
        help="Отключить проверку TLS для OAuth-входа",
    )
    auth_add.add_argument("--ca-bundle", help="Свой CA bundle для OAuth-входа")
    auth_list = auth_subparsers.add_parser("list", help="Показать учетные данные")
    auth_list.add_argument("provider", nargs="?", help="Необязательный фильтр по провайдеру")
    auth_remove = auth_subparsers.add_parser(
        "remove", help="Удалить учетные данные по индексу, id или метке"
    )
    auth_remove.add_argument("provider", help="ID провайдера")
    auth_remove.add_argument(
        "target", help="Индекс учетных данных, id записи или точная метка"
    )
    auth_reset = auth_subparsers.add_parser(
        "reset", help="Сбросить статус исчерпания для всех учетных данных провайдера"
    )
    auth_reset.add_argument("provider", help="ID провайдера")
    auth_status = auth_subparsers.add_parser(
        "status", help="Показать статус авторизации провайдера"
    )
    auth_status.add_argument("provider", help="ID провайдера")
    auth_logout = auth_subparsers.add_parser(
        "logout", help="Выйти из провайдера и очистить сохраненное состояние входа"
    )
    auth_logout.add_argument("provider", help="ID провайдера")
    auth_spotify = auth_subparsers.add_parser(
        "spotify", help="Авторизовать Hermes в Spotify через PKCE"
    )
    auth_spotify.add_argument(
        "spotify_action",
        nargs="?",
        choices=["login", "status", "logout"],
        default="login",
    )
    auth_spotify.add_argument(
        "--client-id", help="client_id приложения Spotify (или HERMES_SPOTIFY_CLIENT_ID)"
    )
    auth_spotify.add_argument(
        "--redirect-uri",
        help="Разрешенный localhost redirect URI для приложения Spotify",
    )
    auth_spotify.add_argument("--scope", help="Переопределить запрашиваемые scopes Spotify")
    auth_spotify.add_argument(
        "--no-browser",
        action="store_true",
        help="Не пытаться открыть браузер автоматически",
    )
    auth_spotify.add_argument(
        "--timeout", type=float, help="Таймаут callback/обмена токена в секундах"
    )
    auth_parser.set_defaults(func=cmd_auth)

"""``hermes dashboard`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

import argparse
from typing import Callable


def build_dashboard_parser(
    subparsers, *, cmd_dashboard: Callable, cmd_dashboard_register: Callable
) -> None:
    """Attach the ``dashboard`` subcommand (and its ``register`` action)."""
    # =========================================================================
    # dashboard command
    # =========================================================================
    dashboard_parser = subparsers.add_parser(
        "dashboard",
        help="Запустить web-панель управления",
        description="Запустить web-панель Hermes RU Iola для конфигурации, API-ключей и сессий",
    )
    dashboard_parser.add_argument(
        "--port", type=int, default=9119, help="Порт (9119 по умолчанию, 0 для автоназначения)"
    )
    dashboard_parser.add_argument(
        "--host", default="127.0.0.1", help="Host (127.0.0.1 по умолчанию)"
    )
    dashboard_parser.add_argument(
        "--no-open", action="store_true", help="Не открывать браузер автоматически"
    )
    dashboard_parser.add_argument(
        "--insecure",
        action="store_true",
        help="Разрешить bind не только на localhost (ОПАСНО: открывает API-ключи в сеть)",
    )
    dashboard_parser.add_argument(
        "--skip-build",
        action="store_true",
        help=(
            "Пропустить сборку web UI и отдавать существующий dist."
        ),
    )
    dashboard_parser.add_argument(
        "--isolated",
        action="store_true",
        help=(
            "При запуске из именованного профиля поднять отдельный dashboard для этого профиля."
        ),
    )
    # Internal flag set by the unified-launch re-exec (cmd_dashboard) to
    # preselect the launching profile in the SPA switcher. Hidden from
    # --help: users get this behavior automatically via `<profile> dashboard`.
    dashboard_parser.add_argument(
        "--open-profile",
        dest="open_profile",
        default="",
        help=argparse.SUPPRESS,
    )
    # Lifecycle flags — mutually exclusive with each other and with the
    # start-a-server flags above (if both are passed, --stop / --status win
    # because they exit before the server is started).  The dashboard has
    # no service manager and no PID file, so these scan the process table
    # for `hermes dashboard` cmdlines and SIGTERM them directly — the same
    # path `hermes update` uses to clean up stale dashboards.
    dashboard_parser.add_argument(
        "--stop",
        action="store_true",
        help="Остановить все запущенные процессы hermes dashboard и выйти",
    )
    dashboard_parser.add_argument(
        "--status",
        action="store_true",
        help="Показать запущенные процессы hermes dashboard и выйти",
    )
    # Backward-compat shim: older Hermes desktop app shells (<= 0.15.x) spawn the
    # backend as `hermes dashboard --no-open --tui --host ... --port ...`. The
    # `--tui` flag was removed from this subcommand in cae6b5486 (embedded chat is
    # always on now). When a user's CLI updates past that commit but their desktop
    # app binary has not, argparse used to hard-error with "unrecognized arguments:
    # --tui" and exit(2) — the backend died before becoming ready and the GUI just
    # showed "Hermes couldn't start" with no actionable cause. Accept and silently
    # ignore the flag so an old app + new CLI degrades gracefully instead of
    # bricking. Hidden from --help; safe to delete once the floor app version is
    # well past 0.16.0.
    dashboard_parser.add_argument(
        "--tui",
        action="store_true",
        help=argparse.SUPPRESS,
    )
    dashboard_parser.set_defaults(func=cmd_dashboard)

    # `hermes dashboard register` — register a self-hosted dashboard OAuth
    # client with Nous Portal and write the client_id into ~/.hermes/.env.
    # Nested subparser so bare `hermes dashboard` keeps launching the server
    # (set_defaults(func=cmd_dashboard) above remains the default).
    dashboard_subparsers = dashboard_parser.add_subparsers(
        dest="dashboard_subcommand"
    )
    dashboard_register_parser = dashboard_subparsers.add_parser(
        "register",
        help="Зарегистрировать self-hosted dashboard в Nous Portal",
        description=(
            "Зарегистрировать эту установку как self-hosted dashboard, создать OAuth client "
            "и записать HERMES_DASHBOARD_OAUTH_CLIENT_ID в ~/.hermes/.env."
        ),
    )
    dashboard_register_parser.add_argument(
        "--name",
        default=None,
        help="Человекочитаемое имя dashboard",
    )
    dashboard_register_parser.add_argument(
        "--redirect-uri",
        dest="redirect_uri",
        default=None,
        help=(
            "Опциональный публичный HTTPS OAuth redirect URI для dashboard."
        ),
    )
    dashboard_register_parser.add_argument(
        "--portal-url",
        dest="portal_url",
        default=None,
        help=(
            "Переопределить base URL Nous Portal для регистрации."
        ),
    )
    dashboard_register_parser.set_defaults(func=cmd_dashboard_register)

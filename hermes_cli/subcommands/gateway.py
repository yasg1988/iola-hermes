"""``hermes gateway`` and ``hermes proxy`` subcommand parsers.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Both parsers are built together because they shared one inline block (the
``gateway`` section also defined ``proxy``). Handlers injected to avoid
importing ``main``.
"""

from __future__ import annotations

import argparse
from typing import Callable

from hermes_cli.subcommands._shared import add_accept_hooks_flag


def _add_compat_platform_flag(parser: argparse.ArgumentParser) -> None:
    """Accept stale `gateway <verb> --platform X` docs without advertising it.

    Gateway service lifecycle commands operate on the gateway process, not a
    single messaging adapter.  Photon briefly printed a per-platform start
    command during setup; keep that command parseable so users following the
    old hint don't get blocked by argparse before the gateway can start.
    """
    parser.add_argument(
        "--platform",
        dest="platform",
        help=argparse.SUPPRESS,
    )


def build_gateway_parser(
    subparsers, *, cmd_gateway: Callable, cmd_proxy: Callable, cmd_gateway_enroll: Callable
) -> None:
    """Attach the ``gateway`` and ``proxy`` subcommands to ``subparsers``."""
    # =========================================================================
    # gateway command
    # =========================================================================
    gateway_parser = subparsers.add_parser(
        "gateway",
        help="Управление gateway мессенджеров",
        description="Управление gateway для Telegram, Discord, WhatsApp, Weixin и других платформ",
    )
    gateway_subparsers = gateway_parser.add_subparsers(dest="gateway_command")

    # gateway run (default)
    gateway_run = gateway_subparsers.add_parser(
        "run", help="Запустить gateway в foreground-режиме"
    )
    gateway_run.add_argument(
        "-v",
        "--verbose",
        action="count",
        default=0,
        help="Повысить подробность stderr-логов (-v=INFO, -vv=DEBUG)",
    )
    gateway_run.add_argument(
        "-q", "--quiet", action="store_true", help="Отключить вывод stderr-логов"
    )
    gateway_run.add_argument(
        "--replace",
        action="store_true",
        help="Заменить уже запущенный экземпляр gateway",
    )
    gateway_run.add_argument(
        "--force",
        action="store_true",
        help=(
            "Запустить foreground gateway, даже если профиль уже обслуживается "
            "systemd/launchd/s6. Используйте только если понимаете риск второго dispatcher."
        ),
    )
    gateway_run.add_argument(
        "--no-supervise",
        action="store_true",
        help=(
            "Внутри Docker-образа с s6-overlay отключить автоматическую передачу "
            "gateway run под supervised s6 service."
        ),
    )
    add_accept_hooks_flag(gateway_run)
    add_accept_hooks_flag(gateway_parser)

    # gateway start
    gateway_start = gateway_subparsers.add_parser(
        "start", help="Запустить установленный background service systemd/launchd"
    )
    gateway_start.add_argument(
        "--system",
        action="store_true",
        help="Использовать Linux system-level gateway service",
    )
    gateway_start.add_argument(
        "--all",
        action="store_true",
        help="Перед запуском остановить все устаревшие gateway-процессы всех профилей",
    )
    _add_compat_platform_flag(gateway_start)

    # gateway stop
    gateway_stop = gateway_subparsers.add_parser("stop", help="Остановить gateway service")
    gateway_stop.add_argument(
        "--system",
        action="store_true",
        help="Использовать Linux system-level gateway service",
    )
    gateway_stop.add_argument(
        "--all",
        action="store_true",
        help="Остановить все gateway-процессы всех профилей",
    )

    # gateway restart
    gateway_restart = gateway_subparsers.add_parser(
        "restart", help="Перезапустить gateway service"
    )
    gateway_restart.add_argument(
        "--system",
        action="store_true",
        help="Использовать Linux system-level gateway service",
    )
    gateway_restart.add_argument(
        "--all",
        action="store_true",
        help="Перед перезапуском остановить все gateway-процессы всех профилей",
    )
    _add_compat_platform_flag(gateway_restart)

    # gateway status
    gateway_status = gateway_subparsers.add_parser("status", help="Показать статус gateway")
    gateway_status.add_argument("--deep", action="store_true", help="Глубокая проверка статуса")
    gateway_status.add_argument(
        "-l",
        "--full",
        action="store_true",
        help="Показать полный вывод service/log без обрезки, где поддерживается",
    )
    gateway_status.add_argument(
        "--system",
        action="store_true",
        help="Использовать Linux system-level gateway service",
    )
    _add_compat_platform_flag(gateway_status)

    # gateway install
    gateway_install = gateway_subparsers.add_parser(
        "install", help="Установить gateway как background service systemd/launchd"
    )
    gateway_install.add_argument("--force", action="store_true", help="Принудительная переустановка")
    gateway_install.add_argument(
        "--system",
        action="store_true",
        help="Установить как Linux system-level service с запуском при boot",
    )
    gateway_install.add_argument(
        "--run-as-user",
        dest="run_as_user",
        help="Пользователь, от имени которого должен работать Linux system service",
    )
    gateway_install.add_argument(
        "--start-now",
        dest="start_now",
        action="store_true",
        default=None,
        help=argparse.SUPPRESS,
    )
    gateway_install.add_argument(
        "--no-start-now",
        dest="start_now",
        action="store_false",
        help=argparse.SUPPRESS,
    )
    gateway_install.add_argument(
        "--start-on-login",
        dest="start_on_login",
        action="store_true",
        default=None,
        help=argparse.SUPPRESS,
    )
    gateway_install.add_argument(
        "--no-start-on-login",
        dest="start_on_login",
        action="store_false",
        help=argparse.SUPPRESS,
    )
    gateway_install.add_argument(
        "--elevated-handoff",
        dest="elevated_handoff",
        action="store_true",
        help=argparse.SUPPRESS,
    )

    # gateway uninstall
    gateway_uninstall = gateway_subparsers.add_parser(
        "uninstall", help="Удалить gateway service"
    )
    gateway_uninstall.add_argument(
        "--system",
        action="store_true",
        help="Использовать Linux system-level gateway service",
    )

    # gateway list
    gateway_subparsers.add_parser("list", help="Показать все профили и статус их gateway")

    # gateway setup
    gateway_subparsers.add_parser("setup", help="Настроить платформы мессенджеров")

    # gateway migrate-legacy
    gateway_migrate_legacy = gateway_subparsers.add_parser(
        "migrate-legacy",
        help="Удалить legacy hermes.service units от старых установок",
        description=(
            "Остановить, отключить и удалить legacy unit-файлы Hermes gateway "
            "от старых установок. Profile units и сторонние сервисы не трогаются."
        ),
    )
    gateway_migrate_legacy.add_argument(
        "--dry-run",
        dest="dry_run",
        action="store_true",
        help="Показать, что будет удалено, без выполнения",
    )
    gateway_migrate_legacy.add_argument(
        "-y",
        "--yes",
        dest="yes",
        action="store_true",
        help="Пропустить подтверждение",
    )

    # gateway enroll — enroll a self-hosted gateway with a relay connector
    # (connector⇄gateway auth). Redeems a single-use enrollment token for the
    # per-gateway secret + per-tenant delivery key and writes them to .env.
    # See docs/relay-connector-contract.md (and the connector repo's
    # docs/connector-gateway-auth-design.md). EXPERIMENTAL.
    gateway_enroll = gateway_subparsers.add_parser(
        "enroll",
        help="Зарегистрировать gateway в relay connector и записать auth-данные в .env",
        description=(
            "Обменять одноразовый enrollment token у relay connector, получить "
            "секрет gateway и delivery key, затем записать GATEWAY_RELAY_* в ~/.hermes/.env."
        ),
    )
    gateway_enroll.add_argument(
        "--token",
        default=None,
        help=(
            "Одноразовый enrollment token от connector. Можно задать через GATEWAY_RELAY_ENROLL_TOKEN."
        ),
    )
    gateway_enroll.add_argument(
        "--connector-url",
        dest="connector_url",
        default=None,
        help=(
            "Base/relay URL connector, например wss://connector.example.com/relay."
        ),
    )
    gateway_enroll.add_argument(
        "--gateway-id",
        dest="gateway_id",
        default=None,
        help=(
            "Стабильный ID этого экземпляра gateway. По умолчанию gw-<hostname>."
        ),
    )
    gateway_enroll.set_defaults(func=cmd_gateway_enroll)

    # =========================================================================
    # proxy command — local OpenAI-compatible proxy that attaches the user's
    # OAuth-authenticated provider credentials to outbound requests. Lets
    # external apps (OpenViking, Karakeep, Open WebUI, ...) ride a logged-in
    # subscription without copy-pasting static API keys.
    # =========================================================================
    proxy_parser = subparsers.add_parser(
        "proxy",
        help="Локальный OpenAI-compatible proxy к OAuth-провайдерам",
        description=(
            "Запустить локальный HTTP-сервер, который проксирует OpenAI-compatible "
            "запросы к OAuth-authenticated провайдеру."
        ),
    )
    proxy_subparsers = proxy_parser.add_subparsers(dest="proxy_command")

    proxy_start = proxy_subparsers.add_parser(
        "start", help="Запустить proxy в foreground-режиме"
    )
    proxy_start.add_argument(
        "--provider",
        default="nous",
        help="Upstream provider: nous или xai.",
    )
    proxy_start.add_argument(
        "--host",
        default=None,
        help="Адрес bind. Используйте 0.0.0.0 для доступа из LAN.",
    )
    proxy_start.add_argument(
        "--port",
        type=int,
        default=None,
        help="Порт bind",
    )

    proxy_subparsers.add_parser(
        "status", help="Показать готовность proxy upstreams"
    )
    proxy_subparsers.add_parser(
        "providers", help="Показать доступные proxy upstream providers"
    )
    proxy_parser.set_defaults(func=cmd_proxy)
    gateway_parser.set_defaults(func=cmd_gateway)

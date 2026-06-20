"""``hermes slack`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_slack_parser(subparsers, *, cmd_slack: Callable) -> None:
    """Attach the ``slack`` subcommand to ``subparsers``."""
    # =========================================================================
    # slack command
    # =========================================================================
    slack_parser = subparsers.add_parser(
        "slack",
        help="Помощники интеграции Slack (генерация manifest и т.п.)",
        description="Помощники интеграции Slack для Hermes.",
    )
    slack_sub = slack_parser.add_subparsers(dest="slack_command")
    slack_manifest = slack_sub.add_parser(
        "manifest",
        help="Вывести или записать manifest Slack app со всеми gateway-командами "
        "как native slash-командами (/btw, /stop, /model, ...)",
        description=(
            "Сгенерировать Slack app manifest, который регистрирует каждую "
            "gateway-команду из COMMAND_REGISTRY как полноценную Slack slash-"
            "команду (паритет с Discord и Telegram). Вставьте вывод в "
            "Slack app config -> Features -> App Manifest -> Edit, затем Save. "
            "Переустановите приложение, если Slack попросит."
        ),
    )
    slack_manifest.add_argument(
        "--write",
        nargs="?",
        const=True,
        default=None,
        metavar="PATH",
        help="Записать manifest в файл вместо stdout. Без PATH пишет в "
        "$HERMES_HOME/slack-manifest.json.",
    )
    slack_manifest.add_argument(
        "--name",
        default=None,
        help='Отображаемое имя бота (по умолчанию: "Hermes")',
    )
    slack_manifest.add_argument(
        "--description",
        default=None,
        help="Описание бота в каталоге приложений Slack.",
    )
    slack_manifest.add_argument(
        "--slashes-only",
        action="store_true",
        help="Вывести только массив features.slash_commands (для ручного "
        "слияния с существующим manifest).",
    )
    slack_parser.set_defaults(func=cmd_slack)

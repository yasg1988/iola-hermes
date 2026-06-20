"""``hermes hooks`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_hooks_parser(subparsers, *, cmd_hooks: Callable) -> None:
    """Attach the ``hooks`` subcommand to ``subparsers``."""
    # =========================================================================
    hooks_parser = subparsers.add_parser(
        "hooks",
        help="Просмотр и управление shell-script hooks",
        description=(
            "Просмотр shell-script hooks из ~/.hermes/config.yaml, проверка "
            "на синтетических payload и управление allowlist первого согласия "
            "в ~/.hermes/shell-hooks-allowlist.json."
        ),
    )
    hooks_subparsers = hooks_parser.add_subparsers(dest="hooks_action")

    hooks_subparsers.add_parser(
        "list",
        aliases=["ls"],
        help="Показать hooks с matcher, timeout и статусом согласия",
    )

    _hk_test = hooks_subparsers.add_parser(
        "test",
        help="Запустить все hooks, подходящие под <event>, на синтетическом payload",
    )
    _hk_test.add_argument(
        "event",
        help="Имя события hook (например: pre_tool_call, pre_llm_call, subagent_stop)",
    )
    _hk_test.add_argument(
        "--for-tool",
        dest="for_tool",
        default=None,
        help=(
            "Запускать только hooks, matcher которых совпадает с этим именем "
            "инструмента (для pre_tool_call / post_tool_call)"
        ),
    )
    _hk_test.add_argument(
        "--payload-file",
        dest="payload_file",
        default=None,
        help=(
            "Путь к JSON-файлу, содержимое которого добавляется в "
            "синтетический payload перед выполнением"
        ),
    )

    _hk_revoke = hooks_subparsers.add_parser(
        "revoke",
        aliases=["remove", "rm"],
        help="Удалить записи команды из allowlist (вступит в силу после перезапуска)",
    )
    _hk_revoke.add_argument(
        "command",
        help="Точная строка команды для отзыва (как указано в config.yaml)",
    )

    hooks_subparsers.add_parser(
        "doctor",
        help=(
            "Проверить каждый настроенный hook: exec bit, allowlist, drift mtime, "
            "валидность JSON и время синтетического запуска"
        ),
    )

    hooks_parser.set_defaults(func=cmd_hooks)

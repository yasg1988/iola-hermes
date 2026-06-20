"""
Top-level argparse construction for the hermes CLI.

Lives in its own module so other modules (e.g. ``relaunch.py``) can
introspect the parser to discover which flags exist without running the
``main`` fn.

Only the top-level parser and the ``chat`` subparser live here. Every other
subparser (model, gateway, sessions, …) is built inline in ``main.py``
because its dispatch is tightly coupled to module-level ``cmd_*`` functions.
"""

import argparse

_ARGPARSE_RU = {
    "positional arguments": "позиционные аргументы",
    "options": "опции",
    "optional arguments": "опции",
    "show this help message and exit": "показать эту справку и выйти",
    "usage: ": "использование: ",
}


def _argparse_gettext(message: str) -> str:
    return _ARGPARSE_RU.get(message, message)


argparse._ = _argparse_gettext


# `--profile` / `-p` is consumed by ``main._apply_profile_override`` before
# argparse runs (it sets ``HERMES_HOME`` and strips itself from ``sys.argv``),
# so it isn't on the parser. Listed here so all "carry over on relaunch"
# metadata lives in one file.
PRE_ARGPARSE_INHERITED_FLAGS: list[tuple[str, bool]] = [
    ("--profile", True),
    ("-p", True),
]

_TOP_LEVEL_COMMAND_HELP_RU: dict[str, str] = {
    "chat": "Интерактивный чат с агентом",
    "model": "Выбрать модель и провайдера по умолчанию",
    "fallback": "Управлять fallback-провайдерами",
    "secrets": "Управлять внешними источниками секретов",
    "migrate": "Миграция устаревших моделей и настроек",
    "gateway": "Управление gateway мессенджеров",
    "proxy": "Локальный OpenAI-compatible proxy к OAuth-провайдерам",
    "lsp": "Управление Language Server Protocol",
    "setup": "Интерактивный мастер настройки",
    "postinstall": "Установить non-Python зависимости после pip install",
    "whatsapp": "Настроить интеграцию WhatsApp",
    "whatsapp-cloud": "Настроить WhatsApp Business Cloud API",
    "slack": "Помощники интеграции Slack",
    "send": "Отправить сообщение в настроенную платформу",
    "login": "Войти в провайдера",
    "logout": "Очистить авторизацию провайдера",
    "auth": "Управлять пулом учётных данных провайдеров",
    "status": "Показать статус всех компонентов",
    "cron": "Управлять задачами по расписанию",
    "webhook": "Управлять динамическими webhook-подписками",
    "portal": "Настроить Nous Portal",
    "kanban": "Доска совместной работы нескольких профилей",
    "hooks": "Проверить и настроить shell hooks",
    "doctor": "Проверить конфигурацию и зависимости",
    "security": "Аудит supply-chain безопасности",
    "dump": "Вывести сводку установки для поддержки",
    "debug": "Debug-инструменты: логи и сведения о системе",
    "backup": "Сделать backup домашнего каталога Hermes",
    "checkpoints": "Управлять файловыми checkpoints",
    "import": "Восстановить backup Hermes",
    "config": "Просмотр и редактирование конфигурации",
    "pairing": "Управлять DM pairing-кодами",
    "skills": "Искать, устанавливать и управлять навыками",
    "bundles": "Создавать и управлять наборами навыков",
    "plugins": "Управлять плагинами",
    "curator": "Фоновое обслуживание навыков",
    "memory": "Настроить внешний провайдер памяти",
    "tools": "Настроить инструменты по платформам",
    "computer-use": "Управлять backend Computer Use",
    "mcp": "Управлять MCP-серверами",
    "sessions": "Управлять историей сессий",
    "insights": "Показать аналитику использования",
    "claw": "Инструменты миграции OpenClaw",
    "version": "Показать информацию о версии",
    "update": "Обновить Hermes RU Iola",
    "uninstall": "Удалить Hermes RU Iola",
    "acp": "Запустить ACP-сервер",
    "profile": "Управлять профилями",
    "completion": "Вывести скрипт автодополнения shell",
    "dashboard": "Запустить web-панель управления",
    "desktop": "Собрать и запустить desktop-приложение",
    "gui": "Собрать и запустить desktop-приложение",
    "logs": "Просмотр и фильтрация логов",
    "prompt-size": "Показать размер системного промпта и схем инструментов",
}


def localize_top_level_subcommands(subparsers) -> None:
    """Translate top-level subparser help rows after all modules register."""
    for action in getattr(subparsers, "_choices_actions", []):
        translated = _TOP_LEVEL_COMMAND_HELP_RU.get(getattr(action, "dest", ""))
        if translated:
            action.help = translated

    for name, child in getattr(subparsers, "choices", {}).items():
        translated = _TOP_LEVEL_COMMAND_HELP_RU.get(name)
        if translated and not getattr(child, "description", None):
            child.description = translated


def _inherited_flag(parser, *args, **kwargs):
    """Register a flag that ``hermes_cli.relaunch`` should carry over when
    the CLI re-execs itself (e.g. after ``sessions browse`` picks a session,
    or after the setup wizard launches chat).

    Equivalent to ``parser.add_argument(...)`` plus tagging the resulting
    Action with ``inherit_on_relaunch = True`` so the relaunch table builder
    can find it via introspection.
    """
    action = parser.add_argument(*args, **kwargs)
    action.inherit_on_relaunch = True
    return action


_EPILOGUE = """
Примеры:
    hermes                         Запустить интерактивный чат
    hermes chat -q "Привет"        Один запрос без интерактивного режима
    hermes --tui                   Запустить современный TUI
    hermes --cli                   Принудительно открыть классический REPL
    hermes -c                      Продолжить последнюю сессию
    hermes -c "мой проект"         Продолжить сессию по имени
    hermes --resume <session_id>   Продолжить сессию по ID
    hermes setup                   Запустить мастер настройки
    hermes logout                  Очистить сохранённую авторизацию
    hermes auth add <provider>     Добавить ключ/учётные данные провайдера
    hermes auth list               Показать сохранённые учётные данные
    hermes model                   Выбрать модель по умолчанию
    hermes fallback [list]         Показать цепочку fallback-провайдеров
    hermes config                  Показать конфигурацию
    hermes config edit             Открыть config.yaml в редакторе
    hermes gateway                 Запустить gateway мессенджеров
    hermes -s hermes-agent-dev     Предзагрузить навыки
    hermes -w                      Запустить в изолированном git worktree
    hermes sessions list           Показать прошлые сессии
    hermes sessions browse         Открыть интерактивный выбор сессии
    hermes logs                    Показать agent.log
    hermes logs -f                 Читать лог в реальном времени
    hermes update                  Обновить Hermes RU Iola
    hermes dashboard               Запустить web-панель управления

Подробная справка по команде:
    hermes <command> --help
"""


def build_top_level_parser():
    """Build the top-level parser, the subparsers action, and the ``chat`` subparser.

    Returns ``(parser, subparsers, chat_parser)``. The caller wires
    ``chat_parser.set_defaults(func=cmd_chat)`` and continues registering
    other subparsers via ``subparsers.add_parser(...)``.
    """
    parser = argparse.ArgumentParser(
        prog="hermes",
        description="Hermes RU Iola — русскоязычный ИИ-ассистент с инструментами, провайдерами и мессенджерами",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=_EPILOGUE,
    )

    parser.add_argument(
        "--version", "-V", action="store_true", help="Показать версию и выйти"
    )
    parser.add_argument(
        "-z",
        "--oneshot",
        metavar="PROMPT",
        default=None,
        help=(
            "Одноразовый режим: отправить один запрос и вывести только финальный "
            "ответ в stdout. Без баннера, спиннера, превью инструментов и строки "
            "session_id. Инструменты, память, правила и AGENTS.md загружаются как обычно; "
            "подтверждения автоматически пропускаются. Подходит для скриптов и pipe."
        ),
    )
    # --model / --provider are accepted at the top level so they can pair
    # with -z without needing the `chat` subcommand.  If neither -z nor a
    # subcommand consumes them, they fall through harmlessly as None.
    # Mirrors `hermes chat --model ... --provider ...` semantics.
    _inherited_flag(
        parser,
        "-m",
        "--model",
        default=None,
        help=(
            "Переопределить модель для этого запуска, например anthropic/claude-sonnet-4.6. "
            "Работает с -z/--oneshot и --tui. Также можно задать через HERMES_INFERENCE_MODEL."
        ),
    )
    _inherited_flag(
        parser,
        "--provider",
        default=None,
        help=(
            "Переопределить провайдера для этого запуска, например openrouter или anthropic. "
            "Работает с -z/--oneshot и --tui. Постоянный провайдер хранится в config.yaml "
            "в model.provider; измените его через `hermes setup` или вручную."
        ),
    )
    parser.add_argument(
        "-t",
        "--toolsets",
        default=None,
        help="Список toolsets через запятую для этого запуска. Работает с -z/--oneshot и --tui.",
    )
    parser.add_argument(
        "--resume",
        "-r",
        metavar="SESSION",
        default=None,
        help="Продолжить предыдущую сессию по ID или названию",
    )
    parser.add_argument(
        "--continue",
        "-c",
        dest="continue_last",
        nargs="?",
        const=True,
        default=None,
        metavar="SESSION_NAME",
        help="Продолжить сессию по имени или последнюю сессию, если имя не указано",
    )
    parser.add_argument(
        "--worktree",
        "-w",
        action="store_true",
        default=False,
        help="Запустить в изолированном git worktree для параллельной работы",
    )
    _inherited_flag(
        parser,
        "--accept-hooks",
        action="store_true",
        default=False,
        help=(
            "Автоматически одобрять новые shell hooks из config.yaml без TTY-запроса. "
            "Эквивалент HERMES_ACCEPT_HOOKS=1 или hooks_auto_accept: true. "
            "Полезно для CI и headless-запусков."
        ),
    )
    _inherited_flag(
        parser,
        "--skills",
        "-s",
        action="append",
        default=None,
        help="Предзагрузить один или несколько навыков для сессии",
    )
    _inherited_flag(
        parser,
        "--yolo",
        action="store_true",
        default=False,
        help="Пропускать подтверждения опасных команд (используйте осознанно)",
    )
    _inherited_flag(
        parser,
        "--pass-session-id",
        action="store_true",
        default=False,
        help="Добавить ID сессии в системный промпт агента",
    )
    _inherited_flag(
        parser,
        "--ignore-user-config",
        action="store_true",
        default=False,
        help="Игнорировать ~/.hermes/config.yaml и использовать встроенные значения по умолчанию",
    )
    _inherited_flag(
        parser,
        "--ignore-rules",
        action="store_true",
        default=False,
        help="Не подмешивать AGENTS.md, SOUL.md, .cursorrules, память и предзагруженные навыки",
    )
    _inherited_flag(
        parser,
        "--safe-mode",
        action="store_true",
        default=False,
        help="Режим диагностики: отключить пользовательскую конфигурацию, правила, память, плагины и MCP",
    )
    _inherited_flag(
        parser,
        "--tui",
        action="store_true",
        default=False,
        help="Запустить современный TUI вместо классического REPL",
    )
    _inherited_flag(
        parser,
        "--cli",
        action="store_true",
        default=False,
        help="Принудительно открыть классический prompt_toolkit REPL",
    )
    _inherited_flag(
        parser,
        "--dev",
        dest="tui_dev",
        action="store_true",
        default=False,
        help="Для --tui: запускать TypeScript-исходники через tsx без dist-сборки",
    )

    subparsers = parser.add_subparsers(dest="command", help="Команда для запуска")

    # =========================================================================
    # chat command
    # =========================================================================
    chat_parser = subparsers.add_parser(
        "chat",
        help="Интерактивный чат с агентом",
        description="Запустить интерактивную сессию чата с Hermes RU Iola",
    )
    chat_parser.add_argument(
        "-q", "--query", help="Один запрос без интерактивного режима"
    )
    chat_parser.add_argument(
        "--image", help="Путь к локальному изображению для прикрепления к одному запросу"
    )
    _inherited_flag(
        chat_parser,
        "-m", "--model", help="Модель для использования, например anthropic/claude-sonnet-4",
    )
    chat_parser.add_argument(
        "-t", "--toolsets", help="Список toolsets через запятую"
    )
    _inherited_flag(
        chat_parser,
        "-s",
        "--skills",
        action="append",
        default=argparse.SUPPRESS,
        help="Предзагрузить один или несколько навыков для сессии",
    )
    _inherited_flag(
        chat_parser,
        "--provider",
        # No `choices=` here: user-defined providers from config.yaml `providers:`
        # are also valid values, and runtime resolution (resolve_runtime_provider)
        # handles validation/error reporting consistently with the top-level
        # `--provider` flag.
        default=None,
        help="Провайдер инференса. Встроенный или пользовательский из `providers:` в config.yaml.",
    )
    chat_parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        default=argparse.SUPPRESS,
        help="Подробный вывод",
    )
    chat_parser.add_argument(
        "-Q",
        "--quiet",
        action="store_true",
        help="Тихий режим для скриптов: без баннера, спиннера и превью инструментов.",
    )
    chat_parser.add_argument(
        "--resume",
        "-r",
        metavar="SESSION_ID",
        default=argparse.SUPPRESS,
        help="Продолжить предыдущую сессию по ID",
    )
    chat_parser.add_argument(
        "--continue",
        "-c",
        dest="continue_last",
        nargs="?",
        const=True,
        default=argparse.SUPPRESS,
        metavar="SESSION_NAME",
        help="Продолжить сессию по имени или последнюю сессию, если имя не указано",
    )
    chat_parser.add_argument(
        "--worktree",
        "-w",
        action="store_true",
        default=argparse.SUPPRESS,
        help="Запустить в изолированном git worktree",
    )
    _inherited_flag(
        chat_parser,
        "--accept-hooks",
        action="store_true",
        default=argparse.SUPPRESS,
        help=(
            "Автоматически одобрять новые shell hooks из config.yaml без TTY-запроса."
        ),
    )
    chat_parser.add_argument(
        "--checkpoints",
        action="store_true",
        default=False,
        help="Включить файловые checkpoints перед разрушительными операциями",
    )
    chat_parser.add_argument(
        "--max-turns",
        type=int,
        default=None,
        metavar="N",
        help="Максимум итераций tool-calling на один ход",
    )
    _inherited_flag(
        chat_parser,
        "--yolo",
        action="store_true",
        default=argparse.SUPPRESS,
        help="Пропускать подтверждения опасных команд",
    )
    _inherited_flag(
        chat_parser,
        "--pass-session-id",
        action="store_true",
        default=argparse.SUPPRESS,
        help="Добавить ID сессии в системный промпт агента",
    )
    _inherited_flag(
        chat_parser,
        "--ignore-user-config",
        action="store_true",
        default=argparse.SUPPRESS,
        help="Игнорировать ~/.hermes/config.yaml и использовать встроенные значения.",
    )
    _inherited_flag(
        chat_parser,
        "--ignore-rules",
        action="store_true",
        default=argparse.SUPPRESS,
        help="Не подмешивать AGENTS.md, SOUL.md, .cursorrules, память и навыки.",
    )
    _inherited_flag(
        chat_parser,
        "--safe-mode",
        action="store_true",
        default=argparse.SUPPRESS,
        help="Режим диагностики: отключить конфиг, правила, память, плагины и MCP.",
    )
    chat_parser.add_argument(
        "--source",
        default=None,
        help="Метка источника сессии для фильтрации.",
    )
    _inherited_flag(
        chat_parser,
        "--tui",
        action="store_true",
        default=False,
        help="Запустить современный TUI вместо классического REPL",
    )
    _inherited_flag(
        chat_parser,
        "--cli",
        action="store_true",
        default=False,
        help="Принудительно открыть классический prompt_toolkit REPL",
    )
    _inherited_flag(
        chat_parser,
        "--dev",
        dest="tui_dev",
        action="store_true",
        default=False,
        help="Для --tui: запускать TypeScript-исходники через tsx без dist-сборки",
    )

    return parser, subparsers, chat_parser

"""``hermes cron`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` — same arguments, same
``func=cmd_cron`` dispatch. The handler is injected so this module does not
import ``main`` (cycle avoidance).
"""

from __future__ import annotations

from typing import Callable

from hermes_cli.subcommands._shared import add_accept_hooks_flag


def build_cron_parser(subparsers, *, cmd_cron: Callable) -> None:
    """Attach the ``cron`` subcommand (and its sub-actions) to ``subparsers``."""
    cron_parser = subparsers.add_parser(
        "cron", help="Управление заданиями cron", description="Управление задачами по расписанию"
    )
    cron_subparsers = cron_parser.add_subparsers(dest="cron_command")

    # cron list
    cron_list = cron_subparsers.add_parser("list", help="Показать задания по расписанию")
    cron_list.add_argument("--all", action="store_true", help="Включить отключенные задания")

    # cron create/add
    cron_create = cron_subparsers.add_parser(
        "create", aliases=["add"], help="Создать задание по расписанию"
    )
    cron_create.add_argument(
        "schedule", help="Расписание вида '30m', 'every 2h' или '0 9 * * *'"
    )
    cron_create.add_argument(
        "prompt", nargs="?", help="Необязательный самостоятельный промпт или инструкция задачи"
    )
    cron_create.add_argument("--name", help="Необязательное понятное имя задания")
    cron_create.add_argument(
        "--deliver",
        help="Куда доставлять результат: origin, local, telegram, discord, signal или platform:chat_id",
    )
    cron_create.add_argument("--repeat", type=int, help="Необязательное число повторов")
    cron_create.add_argument(
        "--skill",
        dest="skills",
        action="append",
        help="Подключить навык. Повторите флаг, чтобы добавить несколько навыков.",
    )
    cron_create.add_argument(
        "--script",
        help=(
            "Путь к скрипту внутри ~/.hermes/scripts/. По умолчанию stdout "
            "скрипта добавляется в промпт агента при каждом запуске. "
            "С --no-agent скрипт и есть задание, а его stdout доставляется "
            "как есть. .sh/.bash запускаются через bash, остальное через Python."
        ),
    )
    cron_create.add_argument(
        "--no-agent",
        dest="no_agent",
        action="store_true",
        default=False,
        help=(
            "Полностью пропустить LLM: запускать --script по расписанию и "
            "доставлять stdout напрямую. Пустой stdout = без сообщения. "
            "Классический watchdog-сценарий (память, диск, CI)."
        ),
    )
    cron_create.add_argument(
        "--workdir",
        help="Абсолютный путь рабочей папки задания. Подмешивает AGENTS.md / CLAUDE.md / .cursorrules из этой папки и использует ее как cwd для terminal/file/code_exec. Не указывайте, чтобы сохранить старое поведение.",
    )

    # cron edit
    cron_edit = cron_subparsers.add_parser(
        "edit", help="Изменить существующее задание"
    )
    cron_edit.add_argument("job_id", help="ID задания для изменения")
    cron_edit.add_argument("--schedule", help="Новое расписание")
    cron_edit.add_argument("--prompt", help="Новый промпт/инструкция задачи")
    cron_edit.add_argument("--name", help="Новое имя задания")
    cron_edit.add_argument("--deliver", help="Новая цель доставки")
    cron_edit.add_argument("--repeat", type=int, help="Новое число повторов")
    cron_edit.add_argument(
        "--skill",
        dest="skills",
        action="append",
        help="Заменить навыки задания этим набором. Повторите флаг для нескольких навыков.",
    )
    cron_edit.add_argument(
        "--add-skill",
        dest="add_skills",
        action="append",
        help="Добавить навык без замены текущего списка. Можно повторять.",
    )
    cron_edit.add_argument(
        "--remove-skill",
        dest="remove_skills",
        action="append",
        help="Удалить конкретный подключенный навык. Можно повторять.",
    )
    cron_edit.add_argument(
        "--clear-skills",
        action="store_true",
        help="Удалить все подключенные навыки из задания",
    )
    cron_edit.add_argument(
        "--script",
        help=(
            "Путь к скрипту внутри ~/.hermes/scripts/. Передайте пустую строку, "
            "чтобы очистить. С --no-agent скрипт и есть задание; иначе его stdout "
            "добавляется в промпт агента при каждом запуске."
        ),
    )
    cron_edit.add_argument(
        "--no-agent",
        dest="no_agent",
        action="store_const",
        const=True,
        default=None,
        help=(
            "Включить режим no-agent для этого задания (нужен --script или "
            "уже заданный скрипт)."
        ),
    )
    cron_edit.add_argument(
        "--agent",
        dest="no_agent",
        action="store_const",
        const=False,
        help="Отключить режим no-agent для задания (вернуться к выполнению через LLM).",
    )
    cron_edit.add_argument(
        "--workdir",
        help="Абсолютный путь рабочей папки задания (подмешивает AGENTS.md и задает cwd терминала). Пустая строка очищает значение.",
    )

    # lifecycle actions
    cron_pause = cron_subparsers.add_parser("pause", help="Приостановить задание")
    cron_pause.add_argument("job_id", help="ID задания для паузы")

    cron_resume = cron_subparsers.add_parser("resume", help="Возобновить задание")
    cron_resume.add_argument("job_id", help="ID задания для возобновления")

    cron_run = cron_subparsers.add_parser(
        "run", help="Запустить задание на следующем тике планировщика"
    )
    cron_run.add_argument("job_id", help="ID задания для запуска")
    add_accept_hooks_flag(cron_run)

    cron_remove = cron_subparsers.add_parser(
        "remove", aliases=["rm", "delete"], help="Удалить задание"
    )
    cron_remove.add_argument("job_id", help="ID задания для удаления")

    # cron status
    cron_subparsers.add_parser("status", help="Проверить, запущен ли планировщик cron")

    # cron tick (mostly for debugging)
    cron_tick = cron_subparsers.add_parser("tick", help="Один раз выполнить готовые задания и выйти")
    add_accept_hooks_flag(cron_tick)
    add_accept_hooks_flag(cron_parser)
    cron_parser.set_defaults(func=cmd_cron)

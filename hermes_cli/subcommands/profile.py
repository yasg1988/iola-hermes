"""``hermes profile`` subcommand parser.

Extracted verbatim from ``hermes_cli/main.py:main()`` (god-file Phase 2).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_profile_parser(subparsers, *, cmd_profile: Callable) -> None:
    """Attach the ``profile`` subcommand to ``subparsers``."""
    # =========================================================================
    # profile command
    # =========================================================================
    profile_parser = subparsers.add_parser(
        "profile",
        help="Управление профилями — несколькими изолированными экземплярами Hermes",
    )
    profile_subparsers = profile_parser.add_subparsers(dest="profile_action")

    profile_subparsers.add_parser("list", help="Показать все профили")
    profile_use = profile_subparsers.add_parser(
        "use", help="Задать профиль по умолчанию"
    )
    profile_use.add_argument("profile_name", help="Имя профиля (или 'default')")

    profile_create = profile_subparsers.add_parser(
        "create", help="Создать новый профиль"
    )
    profile_create.add_argument(
        "profile_name", help="Имя профиля (нижний регистр, буквы/цифры)"
    )
    profile_create.add_argument(
        "--clone",
        action="store_true",
        help="Скопировать config.yaml, .env, SOUL.md и навыки из активного профиля",
    )
    profile_create.add_argument(
        "--clone-all",
        action="store_true",
        help="Полная копия активного профиля (все состояние, кроме истории профиля)",
    )
    profile_create.add_argument(
        "--clone-from",
        metavar="SOURCE",
        help="Исходный профиль для клонирования; подразумевает --clone, если не задан --clone-all",
    )
    profile_create.add_argument(
        "--no-alias", action="store_true", help="Пропустить создание wrapper-скрипта"
    )
    profile_create.add_argument(
        "--no-skills",
        action="store_true",
        help="Создать пустой профиль без встроенных навыков (отключает sync навыков через `hermes update`)",
    )
    profile_create.add_argument(
        "--description",
        default=None,
        help="Описание в одно-два предложения, для чего хорош этот профиль. "
             "Используется kanban decomposer для маршрутизации задач по роли, "
             "а не только имени профиля. Можно пропустить и добавить позже через `hermes profile describe`.",
    )

    profile_delete = profile_subparsers.add_parser("delete", help="Удалить профиль")
    profile_delete.add_argument("profile_name", help="Профиль для удаления")
    profile_delete.add_argument(
        "-y", "--yes", action="store_true", help="Пропустить подтверждение"
    )

    profile_describe = profile_subparsers.add_parser(
        "describe",
        help="Прочитать или задать описание профиля (для kanban orchestrator)",
    )
    profile_describe.add_argument(
        "profile_name",
        nargs="?",
        default=None,
        help="Профиль для описания (опустите и используйте --all --auto для прохода по всем)",
    )
    profile_describe.add_argument(
        "--text",
        default=None,
        help="Задать описание этим точным текстом (перезаписывает существующее)",
    )
    profile_describe.add_argument(
        "--auto",
        action="store_true",
        help="Автоматически сгенерировать описание через вспомогательную LLM "
             "(использует auxiliary.profile_describer)",
    )
    profile_describe.add_argument(
        "--overwrite",
        action="store_true",
        help="С --auto заменять и пользовательские описания (по умолчанию только "
             "заполняются отсутствующие или ранее авто-сгенерированные)",
    )
    profile_describe.add_argument(
        "--all",
        dest="all_missing",
        action="store_true",
        help="С --auto выполнить для каждого профиля без описания",
    )

    profile_show = profile_subparsers.add_parser("show", help="Показать детали профиля")
    profile_show.add_argument("profile_name", help="Профиль для просмотра")

    profile_alias = profile_subparsers.add_parser(
        "alias", help="Управление wrapper-скриптами"
    )
    profile_alias.add_argument("profile_name", help="Имя профиля")
    profile_alias.add_argument(
        "--remove", action="store_true", help="Удалить wrapper-скрипт"
    )
    profile_alias.add_argument(
        "--name",
        dest="alias_name",
        metavar="NAME",
        help="Свое имя alias (по умолчанию имя профиля)",
    )

    profile_rename = profile_subparsers.add_parser("rename", help="Переименовать профиль")
    profile_rename.add_argument("old_name", help="Текущее имя профиля")
    profile_rename.add_argument("new_name", help="Новое имя профиля")

    profile_export = profile_subparsers.add_parser(
        "export", help="Экспортировать профиль в архив"
    )
    profile_export.add_argument("profile_name", help="Профиль для экспорта")
    profile_export.add_argument(
        "-o", "--output", default=None, help="Файл вывода (по умолчанию: <name>.tar.gz)"
    )

    profile_import = profile_subparsers.add_parser(
        "import", help="Импортировать профиль из архива"
    )
    profile_import.add_argument("archive", help="Путь к архиву .tar.gz")
    profile_import.add_argument(
        "--name",
        dest="import_name",
        metavar="NAME",
        help="Имя профиля (по умолчанию определяется из архива)",
    )

    # ---------- Distribution subcommands (issue #20456) ----------
    profile_install = profile_subparsers.add_parser(
        "install",
        help="Установить дистрибутив профиля из git URL или локальной папки",
        description=(
            "Установить дистрибутив профиля Hermes. SOURCE может быть git URL "
            "(github.com/user/repo, https://..., git@...) или локальной "
            "папкой с distribution.yaml в корне."
        ),
    )
    profile_install.add_argument(
        "source",
        help="Источник дистрибутива (git URL или локальная папка)",
    )
    profile_install.add_argument(
        "--name", dest="install_name", metavar="NAME",
        help="Переопределить имя профиля (по умолчанию читается из manifest)",
    )
    profile_install.add_argument(
        "--alias", action="store_true",
        help="Создать shell wrapper alias для установленного профиля",
    )
    profile_install.add_argument(
        "--force", action="store_true",
        help="Перезаписать существующий профиль с тем же именем (данные пользователя сохраняются)",
    )
    profile_install.add_argument(
        "-y", "--yes", action="store_true",
        help="Пропустить подтверждение preview manifest",
    )

    profile_update = profile_subparsers.add_parser(
        "update",
        help="Повторно скачать дистрибутив и применить обновления (данные пользователя сохраняются)",
        description=(
            "Скачать дистрибутив из сохраненного источника и перезаписать "
            "файлы дистрибутива (SOUL.md, skills/, cron/, mcp.json). "
            "Данные пользователя (memories, sessions, auth, .env) не трогаются. "
            "config.yaml сохраняется, если не передан --force-config."
        ),
    )
    profile_update.add_argument("profile_name", help="Профиль для обновления")
    profile_update.add_argument(
        "--force-config", action="store_true",
        help="Также перезаписать config.yaml (обычно сохраняется ради пользовательских настроек)",
    )
    profile_update.add_argument(
        "-y", "--yes", action="store_true",
        help="Пропустить подтверждение",
    )

    profile_info = profile_subparsers.add_parser(
        "info",
        help="Показать distribution manifest профиля (версия, требования, источник)",
    )
    profile_info.add_argument("profile_name", help="Профиль для просмотра")

    profile_parser.set_defaults(func=cmd_profile)

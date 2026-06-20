"""``hermes skills`` subcommand parser.

Extracted from ``hermes_cli/main.py:main()`` (god-file Phase 2 follow-up).
Handler injected to avoid importing ``main``.
"""

from __future__ import annotations

from typing import Callable


def build_skills_parser(subparsers, *, cmd_skills: Callable) -> None:
    """Attach the ``skills`` subcommand to ``subparsers``."""
    skills_parser = subparsers.add_parser(
        "skills",
        help="Поиск, установка, настройка и управление навыками",
        description="Поиск, установка, просмотр, аудит, настройка и управление навыками из skills.sh, well-known agent skill endpoints, GitHub, ClawHub и других реестров.",
    )
    skills_subparsers = skills_parser.add_subparsers(dest="skills_action")

    skills_browse = skills_subparsers.add_parser(
        "browse", help="Просмотреть все доступные навыки (постранично)"
    )
    skills_browse.add_argument(
        "--page", type=int, default=1, help="Номер страницы (по умолчанию: 1)"
    )
    skills_browse.add_argument(
        "--size", type=int, default=20, help="Результатов на страницу (по умолчанию: 20)"
    )
    skills_browse.add_argument(
        "--source",
        default="all",
        choices=[
            "all",
            "official",
            "skills-sh",
            "well-known",
            "github",
            "clawhub",
            "lobehub",
            "browse-sh",
        ],
        help="Фильтр по источнику (по умолчанию: all)",
    )

    skills_search = skills_subparsers.add_parser(
        "search", help="Искать в реестрах навыков"
    )
    skills_search.add_argument("query", help="Поисковый запрос")
    skills_search.add_argument(
        "--source",
        default="all",
        choices=[
            "all",
            "official",
            "skills-sh",
            "well-known",
            "github",
            "clawhub",
            "lobehub",
            "browse-sh",
        ],
    )
    skills_search.add_argument("--limit", type=int, default=10, help="Максимум результатов")
    skills_search.add_argument(
        "--json",
        action="store_true",
        help="Вывести JSON вместо таблицы (полные идентификаторы, удобно для скриптов)",
    )

    skills_install = skills_subparsers.add_parser("install", help="Установить навык")
    skills_install.add_argument(
        "identifier",
        help="Идентификатор навыка (например openai/skills/skill-creator) или прямой HTTP(S) URL к SKILL.md",
    )
    skills_install.add_argument(
        "--category", default="", help="Папка категории для установки"
    )
    skills_install.add_argument(
        "--name",
        default="",
        help="Переопределить имя навыка (полезно при установке из URL, где SKILL.md без frontmatter `name:`)",
    )
    skills_install.add_argument(
        "--force", action="store_true", help="Установить несмотря на блокирующий verdict сканирования"
    )
    skills_install.add_argument(
        "--yes",
        "-y",
        action="store_true",
        help="Пропустить подтверждение (нужно в TUI-режиме)",
    )

    skills_inspect = skills_subparsers.add_parser(
        "inspect", help="Просмотреть навык без установки"
    )
    skills_inspect.add_argument("identifier", help="Идентификатор навыка")

    skills_list = skills_subparsers.add_parser("list", help="Показать установленные навыки")
    skills_list.add_argument(
        "--source", default="all", choices=["all", "hub", "builtin", "local"]
    )
    skills_list.add_argument(
        "--enabled-only",
        action="store_true",
        help="Скрыть отключенные навыки. Используйте с -p <profile>, чтобы увидеть, "
        "какие навыки загрузятся для профиля.",
    )

    skills_check = skills_subparsers.add_parser(
        "check", help="Проверить обновления установленных hub-навыков"
    )
    skills_check.add_argument(
        "name", nargs="?", help="Конкретный навык для проверки (по умолчанию: все)"
    )

    skills_update = skills_subparsers.add_parser(
        "update", help="Обновить установленные hub-навыки"
    )
    skills_update.add_argument(
        "name",
        nargs="?",
        help="Конкретный навык для обновления (по умолчанию: все устаревшие)",
    )

    skills_audit = skills_subparsers.add_parser(
        "audit", help="Повторно просканировать установленные hub-навыки"
    )
    skills_audit.add_argument(
        "name", nargs="?", help="Конкретный навык для аудита (по умолчанию: все)"
    )
    skills_audit.add_argument(
        "--deep",
        action="store_true",
        help="Запустить AST-анализ Python-файлов (опциональная диагностика)",
    )

    skills_uninstall = skills_subparsers.add_parser(
        "uninstall", help="Удалить hub-навык"
    )
    skills_uninstall.add_argument("name", help="Имя навыка для удаления")

    skills_reset = skills_subparsers.add_parser(
        "reset",
        help="Сбросить встроенный навык: очистить tracking 'user-modified', чтобы обновления снова работали",
        description=(
            "Очистить запись встроенного навыка из sync manifest (~/.hermes/skills/.bundled_manifest), "
            "чтобы будущие запуски 'hermes update' перестали считать его user-modified. Передайте --restore, "
            "чтобы также заменить текущую копию встроенной версией."
        ),
    )
    skills_reset.add_argument(
        "name", help="Имя навыка для сброса (например google-workspace)"
    )
    skills_reset.add_argument(
        "--restore",
        action="store_true",
        help="Также удалить текущую копию и заново скопировать встроенную версию",
    )
    skills_reset.add_argument(
        "--yes",
        "-y",
        action="store_true",
        help="Пропустить подтверждение при использовании --restore",
    )

    skills_list_modified = skills_subparsers.add_parser(
        "list-modified",
        help="Показать измененные вами встроенные навыки (которые `hermes update` сохраняет)",
        description=(
            "Показать встроенные навыки, локальная копия которых отличается от последней "
            "синхронизированной версии, то есть те, которые `hermes update` помечает как "
            "user-modified и пропускает. Используйте `hermes skills diff <name>` для просмотра "
            "изменений и `hermes skills reset <name>`, чтобы вернуть обновления."
        ),
    )
    skills_list_modified.add_argument(
        "--json",
        action="store_true",
        help="Вывести список в JSON",
    )

    skills_diff = skills_subparsers.add_parser(
        "diff",
        help="Показать отличия вашей копии встроенного навыка от штатной версии",
        description=(
            "Вывести unified diff между вашей локальной копией встроенного навыка и "
            "текущей штатной версией, чтобы проверить изменения перед запуском "
            "`hermes skills reset`."
        ),
    )
    skills_diff.add_argument(
        "name", help="Имя навыка для diff (например google-workspace)"
    )

    skills_opt_out = skills_subparsers.add_parser(
        "opt-out",
        help="Отключить добавление встроенных навыков в этот профиль",
        description=(
            "Записать маркер .no-bundled-skills, чтобы installer, `hermes update` "
            "и прямой sync перестали добавлять встроенные навыки в активный профиль. "
            "По умолчанию уже существующие файлы не трогаются. Передайте --remove, "
            "чтобы также удалить неизмененные встроенные навыки (пользовательские "
            "правки и hub/local навыки никогда не удаляются)."
        ),
    )
    skills_opt_out.add_argument(
        "--remove",
        action="store_true",
        help="Также удалить уже имеющиеся неизмененные встроенные навыки",
    )
    skills_opt_out.add_argument(
        "--yes",
        "-y",
        action="store_true",
        help="Пропустить подтверждение при использовании --remove",
    )

    skills_opt_in = skills_subparsers.add_parser(
        "opt-in",
        help="Снова включить добавление встроенных навыков (отменить opt-out)",
        description=(
            "Удалить маркер .no-bundled-skills, чтобы встроенные навыки снова "
            "добавлялись при следующем `hermes update`. Передайте --sync, чтобы "
            "добавить их сразу."
        ),
    )
    skills_opt_in.add_argument(
        "--sync",
        action="store_true",
        help="Сразу заново добавить встроенные навыки, не дожидаясь update",
    )

    skills_repair_official = skills_subparsers.add_parser(
        "repair-official",
        help="Дозаполнить или восстановить официальные optional-навыки из исходников репозитория",
        description=(
            "Исправить provenance официальных optional-навыков. По умолчанию только "
            "дозаполняет hub metadata для точных совпадений. Передайте --restore, "
            "чтобы заменить отсутствующие или измененные активные копии из optional-skills/, "
            "предварительно переместив существующие копии в backup восстановления. "
            "Используйте имя 'all' для восстановления всех optional-навыков."
        ),
    )
    skills_repair_official.add_argument(
        "name", help="Папка/frontmatter name официального optional-навыка или 'all'"
    )
    skills_repair_official.add_argument(
        "--restore",
        action="store_true",
        help="Восстановить из официального optional-источника с backup существующих совпадающих копий",
    )
    skills_repair_official.add_argument(
        "--yes",
        "-y",
        action="store_true",
        help="Пропустить подтверждение при использовании --restore",
    )

    skills_publish = skills_subparsers.add_parser(
        "publish", help="Опубликовать навык в реестр"
    )
    skills_publish.add_argument("skill_path", help="Путь к папке навыка")
    skills_publish.add_argument(
        "--to", default="github", choices=["github", "clawhub"], help="Целевой реестр"
    )
    skills_publish.add_argument(
        "--repo", default="", help="Целевой GitHub repo (например openai/skills)"
    )

    skills_snapshot = skills_subparsers.add_parser(
        "snapshot", help="Экспорт/импорт конфигураций навыков"
    )
    snapshot_subparsers = skills_snapshot.add_subparsers(dest="snapshot_action")
    snap_export = snapshot_subparsers.add_parser(
        "export", help="Экспортировать установленные навыки в файл"
    )
    snap_export.add_argument("output", help="Путь к выходному JSON-файлу (используйте - для stdout)")
    snap_import = snapshot_subparsers.add_parser(
        "import", help="Импортировать и установить навыки из файла"
    )
    snap_import.add_argument("input", help="Путь к входному JSON-файлу")
    snap_import.add_argument(
        "--force", action="store_true", help="Принудительно установить несмотря на caution verdict"
    )

    skills_tap = skills_subparsers.add_parser("tap", help="Управление источниками навыков")
    tap_subparsers = skills_tap.add_subparsers(dest="tap_action")
    tap_subparsers.add_parser("list", help="Показать настроенные taps")
    tap_add = tap_subparsers.add_parser("add", help="Добавить GitHub repo как источник навыков")
    tap_add.add_argument("repo", help="GitHub repo (например owner/repo)")
    tap_rm = tap_subparsers.add_parser("remove", help="Удалить tap")
    tap_rm.add_argument("name", help="Имя tap для удаления")

    # config sub-action: interactive enable/disable
    skills_subparsers.add_parser(
        "config",
        help="Интерактивная настройка навыков: включение/отключение отдельных навыков",
    )
    skills_parser.set_defaults(func=cmd_skills)

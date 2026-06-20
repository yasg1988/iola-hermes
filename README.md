<p align="center">
  <img src="assets/iola-hermes-banner.png" alt="Hermes RU Iola" width="100%">
</p>

# Hermes RU Iola

**Hermes RU Iola** — русскоязычный публичный форк
[NousResearch/Hermes Agent](https://github.com/NousResearch/hermes-agent).
Проект адаптирует Hermes Agent для русскоязычных пользователей, развивает
установку через npm, desktop-сборки для Windows/Linux, русскую локализацию,
дополнительных провайдеров моделей и интеграции с мессенджерами.

Проект не связан с Nous Research, не поддерживается ими и не является
официальной сборкой Hermes Agent. Оригинальный проект доступен в репозитории
[nousresearch/hermes-agent](https://github.com/NousResearch/hermes-agent).
Лицензия MIT и уведомления об авторских правах upstream сохранены.

## Что уже сделано

- Создан публичный форк `yasg1988/iola-hermes`.
- Подключён бренд **Hermes RU Iola**.
- Добавлен CLI-alias `iola-hermes`.
- Подготовлен npm wrapper-пакет `iola-hermes`.
- Добавлена частичная русская локаль desktop-интерфейса с fallback на английский.
- Добавлен roadmap и шаблон для новых OpenAI-compatible провайдеров.
- Подключён новый баннер проекта.

## Установка через npm

```bash
npm install -g iola-hermes
iola-hermes
```

npm-пакет устанавливает Python backend из этого репозитория:

```text
https://github.com/yasg1988/iola-hermes
```

Команда `iola-hermes` запускает Hermes CLI через Python-модуль
`hermes_cli.main`. Команда `hermes` также остаётся доступной при установке
через Python packaging для совместимости с upstream.

## Установка из исходников

```bash
git clone https://github.com/yasg1988/iola-hermes.git
cd iola-hermes
py -3.13 -m pip install -e . --no-deps
iola-hermes
```

Проект требует Python `>=3.11,<3.14`. На Windows рекомендуется Python 3.13,
если Python 3.14 уже установлен как версия по умолчанию.

## Desktop

Desktop-приложение основано на Electron-сборке upstream Hermes. Целевое имя:
**Hermes RU Iola**.

Запланированные форматы:

- Windows: `nsis`, `msi`
- Linux: `AppImage`, `deb`, `rpm`

Скрипты сборки находятся в `apps/desktop/package.json`.

## Русификация

В проекте уже есть русские каталоги для части интерфейсов:

- `locales/ru.yaml`
- `web/src/i18n/ru.ts`
- `apps/desktop/src/i18n/ru.ts`

Desktop-локаль сейчас частичная: непереведённые строки автоматически
возвращаются к английскому тексту upstream. Это сделано намеренно, чтобы
локализация не блокировала сборку и могла расширяться поэтапно.

## Провайдеры моделей

Hermes RU Iola использует plugin-подход upstream. Для простых
OpenAI-compatible провайдеров не нужно менять ядро проекта: достаточно
добавить provider plugin.

Документы:

- `docs/iola-provider-roadmap.md`
- `docs/templates/model-provider-plugin/`

## Мессенджеры

Интеграции с мессенджерами развиваются через gateway adapters и plugin path.
Сначала сохраняется совместимость с upstream-платформами, затем добавляются
новые адаптеры и русские инструкции настройки.

## Связь с upstream

Remote `upstream` должен указывать на оригинальный репозиторий:

```bash
git remote add upstream https://github.com/nousresearch/hermes-agent.git
```

Remote `origin` должен указывать на этот форк:

```bash
git remote add origin https://github.com/yasg1988/iola-hermes.git
```

## Лицензия

Проект распространяется по лицензии MIT. См. `LICENSE`.

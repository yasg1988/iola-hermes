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
- Добавлен CLI-alias `iola-hermes` и npm-пакет `iola-hermes`.
- Опубликована npm-версия `0.17.1` от пользователя `yasg1988`.
- Русский язык включён по умолчанию для web/desktop-интерфейсов.
- Переведены основные пользовательские поверхности: README, web/desktop
  документация, CLI help, setup/uninstall banners, TUI-брендинг,
  installer/update metadata и ключевые desktop-сообщения.
- Добавлен roadmap и шаблон для новых OpenAI-compatible провайдеров.
- Подключён новый баннер проекта.
- Настроен remote `upstream` для синхронизации с оригинальным Hermes Agent.
- Включена базовая защита ветки `main` от удаления и force push.

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

Desktop-приложение основано на Electron-сборке upstream Hermes. Имя приложения,
идентификаторы релиза и ссылки обновления переведены на **Hermes RU Iola**.

Запланированные форматы:

- Windows: `nsis`, `msi`
- Linux: `AppImage`, `deb`, `rpm`

Скрипты сборки находятся в `apps/desktop/package.json`.

## Русификация

Русский язык является целевым языком форка. Основные каталоги локализации:

- `locales/ru.yaml`
- `web/src/i18n/ru.ts`
- `apps/desktop/src/i18n/ru.ts`

В текущем состоянии переведены основные видимые пользовательские сценарии:

- CLI help и команды запуска/установки/удаления.
- Desktop onboarding, настройки, обновления, gateway-сообщения и основные
  системные уведомления.
- Web-интерфейс и русская локаль по умолчанию.
- TUI-брендинг и основные billing-экраны.
- README и npm-документация.

Что намеренно может оставаться на английском:

- Названия внешних сервисов и провайдеров: OpenAI, Nous Research, Slack,
  Discord, Matrix, Telegram и другие.
- Технические комментарии, тестовые фикстуры и типы.
- Неактивные локали других языков, которые сохранены для совместимости с
  upstream.
- Юридически значимые upstream-упоминания и ссылки на оригинальный проект.

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

## Ближайшие этапы

- Дособрать Windows/Linux desktop-релизы и проверить установщики на чистых
  машинах.
- Добавить новые OpenAI-compatible провайдеры через plugin-шаблон.
- Расширить русские инструкции для Telegram, Matrix, Discord, Slack и других
  gateway-интеграций.
- Пройти отдельный глубокий перевод больших CLI-мастеров `hermes_cli/setup.py`
  и `hermes_cli/skills_hub.py`, где часть строк зависит от внешних upstream
  сервисов и требует аккуратной проверки.
- Поддерживать синхронизацию с upstream без потери русской локализации.

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

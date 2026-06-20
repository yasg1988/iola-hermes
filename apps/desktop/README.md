# Hermes RU Iola Desktop

Нативное desktop-приложение для [Hermes RU Iola](../../README.md): тот же агент, навыки, память и история, что в CLI и gateway, но в отдельном окне без обязательного терминала. Поддерживаются **Windows, Linux и macOS**.

## Возможности

| Раздел | Что дает |
|---|---|
| Полный агент в чате | Потоковые ответы, live-активность инструментов, структурированные summaries и общая история с другими интерфейсами Hermes. |
| Превью рядом с чатом | Web-страницы, файлы и вывод инструментов открываются в правой панели. |
| Файловый браузер | Просмотр рабочей папки без выхода из приложения. |
| Голос | Можно говорить с Hermes и получать голосовой ответ. |
| Настройки и onboarding | Провайдеры, модели, инструменты и учетные данные настраиваются через UI. |
| Обновления | Встроенное обновление подтягивает свежую версию агента и пересобирает приложение на месте. |

## Установка

### Через Hermes CLI

Если CLI уже установлен:

```bash
hermes desktop
```

Команда соберет и запустит GUI поверх текущей установки: та же конфигурация, ключи, сессии и навыки. При первом запуске Hermes поможет выбрать провайдера и модель.

### Готовые installers

Для публичного релиза Hermes RU Iola installers будут публиковаться в GitHub Releases проекта `yasg1988/iola-hermes`.

## Обновление

Приложение проверяет обновления в фоне и предлагает обновиться в один клик. Из CLI можно обновиться вручную:

```bash
hermes update
```

## Требования

Installer должен подготовить все нужное: Python 3.11+, portable Git, ripgrep и runtime-зависимости.

## Разработка

Один раз установите workspace-зависимости из корня репозитория, затем запускайте dev server из этой папки:

```bash
npm install
cd apps/desktop
npm run dev
```

Можно указать конкретный checkout исходников или изолировать тестовую конфигурацию:

```bash
HERMES_DESKTOP_HERMES_ROOT=/path/to/clone npm run dev
HERMES_HOME=/tmp/throwaway npm run dev
npm run dev:fake-boot
```

## Сборка installers

```bash
npm run dist:mac     # DMG + zip
npm run dist:win     # NSIS + MSI
npm run dist:linux   # AppImage + deb + rpm
npm run pack         # unpacked app в release/
```

Installers собираются и загружаются в GitHub Releases вручную. Подпись macOS/Windows включается автоматически, если в окружении есть нужные credentials (`CSC_LINK`, `CSC_KEY_PASSWORD`, `APPLE_*`, `WIN_CSC_*`).

## Как это устроено

Packaged app содержит Electron shell. При первом запуске он устанавливает runtime Hermes в `HERMES_HOME` (`~/.hermes`, на Windows `%LOCALAPPDATA%\hermes`) с тем же layout, что у CLI-установки. Renderer на React общается с backend `hermes dashboard` через стандартные gateway API и переиспользует embedded TUI вместо отдельной реализации чата. Логика установки, поиска backend и self-update находится в `electron/main.cjs`.

## Проверка

Перед PR:

```bash
npm run fix
npm run typecheck
npm run lint
npm run test:desktop:all
```

## Диагностика

Логи старта пишутся в `HERMES_HOME/logs/desktop.log`. Смотрите этот файл первым, если приложение сообщает об ошибке запуска.

**macOS / Linux:**

```bash
rm "$HOME/.hermes/hermes-agent/.hermes-bootstrap-complete"
rm -rf "$HOME/.hermes/hermes-agent/venv"
tccutil reset Microphone com.nousresearch.hermes
```

**Windows PowerShell:**

```powershell
Remove-Item "$env:LOCALAPPDATA\hermes\hermes-agent\.hermes-bootstrap-complete"
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\hermes\hermes-agent\venv"
```

По умолчанию Hermes home на Windows: `%LOCALAPPDATA%\hermes`. Если путь перенесен, задайте `HERMES_HOME`.

## Лицензия

MIT, см. [LICENSE](../../LICENSE).

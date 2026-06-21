# Hermes RU Iola Desktop на Tauri

Легкое desktop-приложение Hermes RU Iola на Rust и системном WebView.

Эта сборка развивается параллельно с Electron-приложением и предназначена для
Windows/Linux пакетов с меньшим потреблением памяти, меньшим размером
установщика и быстрым запуском.

## Команды

```bash
npm run --workspace apps/desktop-tauri dev
npm run --workspace apps/desktop-tauri check
npm run --workspace apps/desktop-tauri build
```

Публичные Windows/Linux пакеты публикуются в GitHub Releases как отдельные
assets `Hermes-RU-Iola-Tauri-*`:

- Windows: `Hermes-RU-Iola-Tauri-<version>-win-x64.exe`, `.msi`;
- Linux: `Hermes-RU-Iola-Tauri-<version>-linux-x86_64.AppImage`, `.deb`,
  `.rpm`.

На текущем этапе Tauri-приложение собирает основной React UI из `apps/desktop`.
Rust-часть поднимает локальный `hermes dashboard`, держит session token, отдает
`getConnection`/`getGatewayWsUrl` и выполняет REST-запросы через `hermes_api`.
TypeScript-слой устанавливает совместимый `window.hermesDesktop`.

## Что уже есть

- отдельное приложение `apps/desktop-tauri`;
- отдельный Rust crate `src-tauri`;
- команды `backend_probe`, `backend_version`, `start_backend`;
- команды `get_connection`, `get_gateway_ws_url`, `hermes_api`;
- команды `open_external`, `read_file_text`, `read_file_data_url`, `read_dir`,
  `sanitize_workspace_cwd`, `git_root`;
- системные диалоги выбора файлов/каталогов через `select_paths`;
- сохранение каталога проектов по умолчанию через системный диалог и ручной
  ввод пути;
- запись текста в буфер обмена через `write_clipboard`;
- встроенный PTY-терминал через `terminal.start/write/resize/dispose` и
  события `terminal.onData/onExit`;
- сохранение изображений из URL и буфера в папку загрузок;
- сохранение изображения из системного clipboard;
- запрос доступа к микрофону через WebView media API;
- проверка и применение обновлений для source/git-запуска через `git fetch` и
  `git pull --ff-only`;
- проверка обновлений packaged-приложения через GitHub Releases, выбор
  подходящего Tauri installer/AppImage, скачивание и передача запуска
  установщику;
- сводка удаления и запуск штатного `hermes uninstall` из Tauri;
- TypeScript bridge `src/hermes-desktop-bridge.ts`;
- основной React renderer из `apps/desktop`;
- настройки подключения gateway: local mode и remote token gateway с
  сохранением, применением, проверкой и probe `/api/status`;
- проверка живого подключения и touch текущего backend для восстановления
  gateway после обрыва связи;
- password gateway: вход по логину/паролю, локальное хранение session cookies
  и получение свежего `ws-ticket` для WebSocket-подключения;
- redirect OAuth gateway: отдельное окно входа `/login`, проверка сессии через
  `/api/auth/me`, локальное хранение session cookies и получение свежего
  `ws-ticket` для WebSocket-подключения;
- события запуска локального backend через `onBootProgress`;
- событие завершения локального backend через `onBackendExit`;
- сброс и повторная попытка запуска backend из boot-failure overlay через
  `resetBootstrap`/`repairBootstrap`;
- сохранение активного профиля, переключение профиля из интерфейса и запуск
  локального backend с выбранным профилем;
- локальный backend pool для profile-scoped `getConnection`, `getGatewayWsUrl`,
  `hermes_api` и `touchBackend` без смешивания активного и фоновых профилей;
- синхронизация native theme и события состояния окна через
  `setNativeTheme`/`onWindowStateChanged`;
- runtime-настройка цвета заголовка и фона через стабильный Tauri API,
  валидация данных, совместимых с Electron, и сохранение значения прозрачности
  для совместимости настроек;
- обработка `hermes://` deep links через `onDeepLink` и очередь до
  `signalDeepLinkReady`;
- получение заголовков страниц для ссылок с ограничением сетевого доступа и
  размера ответа;
- поиск и установка цветовых тем из VS Code Marketplace без выполнения кода
  расширений;
- определение git worktree/ветки для локальных рабочих каталогов;
- нормализация целей предпросмотра: локальные URL, файлы, каталоги с
  `index.html`, MIME-тип, размер и тип предпросмотра;
- отслеживание изменений локальных файлов предпросмотра через
  `watchPreviewFile`/`onPreviewFileChanged`;
- нативные desktop-уведомления через Tauri notification plugin, permission
  flow, клик для фокуса нужной сессии и best-effort обработка действий там,
  где их поддерживает платформа;
- отдельные окна сессий через `openSessionWindow`/`openNewSessionWindow`:
  pop-out для существующей сессии, spectator/watch mode, компактное окно новой
  сессии и фокус уже открытого окна вместо дубликата;
- Windows/Linux release workflow для Tauri-сборок и публикация отдельных
  `Hermes-RU-Iola-Tauri-*` assets в GitHub Releases;
- Rust-тесты Tauri updater в release workflow перед упаковкой Windows/Linux
  установщиков;
- ручной smoke-test Windows NSIS: silent-install, запуск установленного
  приложения и проверка живого процесса.
- ручной smoke-test Linux-пакетов в WSL: проверка метаданных `.deb`, состава
  пакета, извлечения AppImage и файла рабочего стола.

## Что дальше

- прогнать ручной сценарий установки и запуска Linux-пакетов на Linux-хосте.

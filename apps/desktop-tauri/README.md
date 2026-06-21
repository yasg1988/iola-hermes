# Hermes RU Iola Tauri

Экспериментальная легкая desktop-оболочка на Tauri.

Эта сборка не заменяет основное Electron-приложение. Она нужна, чтобы
постепенно перенести desktop-оболочку на Rust + системный WebView и сравнить
память, размер установщика и скорость запуска.

## Команды

```bash
npm run --workspace apps/desktop-tauri dev
npm run --workspace apps/desktop-tauri check
npm run --workspace apps/desktop-tauri build
```

На текущем этапе Tauri-приложение собирает основной React UI из `apps/desktop`.
Rust-часть поднимает локальный `hermes dashboard`, держит session token, отдает
`getConnection`/`getGatewayWsUrl` и проксирует REST-запросы через `hermes_api`.
TypeScript-слой устанавливает совместимый `window.hermesDesktop`.

## Что уже есть

- отдельное приложение `apps/desktop-tauri`;
- отдельный Rust crate `src-tauri`;
- команды `backend_probe`, `backend_version`, `start_backend`;
- команды `get_connection`, `get_gateway_ws_url`, `hermes_api`;
- команды `open_external`, `read_file_text`, `read_file_data_url`, `read_dir`,
  `sanitize_workspace_cwd`, `git_root`;
- системные диалоги выбора файлов/каталогов через `select_paths`;
- запись текста в буфер обмена через `write_clipboard`;
- встроенный PTY-терминал через `terminal.start/write/resize/dispose` и
  события `terminal.onData/onExit`;
- сохранение изображений из URL и буфера в папку загрузок;
- сохранение изображения из системного clipboard;
- запрос доступа к микрофону через WebView media API;
- проверка и применение обновлений для source/git-запуска через `git fetch` и
  `git pull --ff-only`;
- сводка удаления и запуск штатного `hermes uninstall` из Tauri;
- TypeScript bridge `src/hermes-desktop-bridge.ts`;
- основной React renderer из `apps/desktop`;
- настройки подключения gateway: local mode и remote token gateway с
  сохранением, применением, проверкой и probe `/api/status`;
- password gateway: вход по логину/паролю, локальное хранение session cookies
  и получение свежего `ws-ticket` для WebSocket-подключения;
- redirect OAuth gateway: отдельное окно входа `/login`, проверка сессии через
  `/api/auth/me`, локальное хранение session cookies и получение свежего
  `ws-ticket` для WebSocket-подключения;
- события запуска локального backend через `onBootProgress`;
- событие завершения локального backend через `onBackendExit`;
- синхронизация native theme и события состояния окна через
  `setNativeTheme`/`onWindowStateChanged`;
- обработка `hermes://` deep links через `onDeepLink` и очередь до
  `signalDeepLinkReady`;
- отслеживание изменений локальных файлов предпросмотра через
  `watchPreviewFile`/`onPreviewFileChanged`;
- Windows/Linux release workflow для Tauri-сборок.

## Что дальше

- заменить оставшиеся заглушки на настоящие Tauri-команды для packaged
  auto-update, multi-window и notification/focus events;
- доработать runtime translucency/titlebar tint, если выбранная платформа и
  Tauri API позволят применить эффект без нестабильных системных вызовов;
- прогнать ручной сценарий запуска установленного приложения.

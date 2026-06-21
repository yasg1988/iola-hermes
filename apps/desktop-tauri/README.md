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
- Windows/Linux release workflow для Tauri-сборок.

## Что дальше

- заменить оставшиеся заглушки на настоящие Tauri-команды для packaged
  auto-update и расширенных desktop-событий;
- перенести OAuth login flow для remote gateway;
- перенести boot/progress events;
- прогнать ручной сценарий запуска установленного приложения.

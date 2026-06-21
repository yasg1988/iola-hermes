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

На текущем этапе это рабочая легкая панель и первый слой совместимости с
боевым desktop UI: Rust-часть поднимает локальный `hermes dashboard`, держит
session token, отдает `getConnection`/`getGatewayWsUrl` и проксирует REST
запросы через `hermes_api`. TypeScript-слой устанавливает
`window.hermesDesktop`, чтобы следующий этап мог подключить основной React UI
из `apps/desktop`.

## Что уже есть

- отдельное приложение `apps/desktop-tauri`;
- отдельный Rust crate `src-tauri`;
- команды `backend_probe`, `backend_version`, `start_backend`;
- команды `get_connection`, `get_gateway_ws_url`, `hermes_api`;
- TypeScript bridge `src/hermes-desktop-bridge.ts`;
- отдельный минимальный Vite UI для проверки Rust/backend-интеграции.

## Что дальше

- добавить JS bridge, совместимый с `window.hermesDesktop`;
- перенести boot/progress events;
- перенести gateway WebSocket URL/auth helpers;
- добавить PTY/терминал;
- настроить Windows/Linux release workflow.

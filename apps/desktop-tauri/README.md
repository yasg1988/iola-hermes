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

На первом этапе это рабочая легкая панель: она проверяет Python backend,
показывает версию и запускает dashboard. Полный React UI из `apps/desktop`
будет подключен после переноса совместимого `window.hermesDesktop` bridge из
Electron preload.

## Что уже есть

- отдельное приложение `apps/desktop-tauri`;
- отдельный Rust crate `src-tauri`;
- команды `backend_probe`, `backend_version`, `start_backend`;
- отдельный минимальный Vite UI для проверки Rust/backend-интеграции.

## Что дальше

- добавить JS bridge, совместимый с `window.hermesDesktop`;
- перенести boot/progress events;
- перенести gateway WebSocket URL/auth helpers;
- добавить PTY/терминал;
- настроить Windows/Linux release workflow.

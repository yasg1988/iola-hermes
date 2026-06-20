# Hermes RU Iola — Web UI

Браузерная панель для управления конфигурацией Hermes RU Iola, API-ключами и активными сессиями.

## Стек

- **Vite** + **React 19** + **TypeScript**
- **Tailwind CSS v4** с собственной темной темой
- Компоненты в стиле **shadcn/ui** без зависимости от CLI-генератора

## Разработка

```bash
# Запустить backend API
cd ../
python -m hermes_cli.main dashboard --no-open

# В другом терминале запустить Vite dev server с HMR и API proxy
cd web/
npm install
npm run dev
```

Откройте **Vite URL**, который появится в терминале, обычно `http://localhost:5173`. Это live-reload интерфейс.

`hermes dashboard` на порту 9119 отдает **собранный** bundle из `hermes_cli/web_dist/`, а не Vite dev server. Изменения в `web/src/` появятся там только после `npm run build` и перезапуска dashboard.

Vite dev server проксирует запросы `/api` на `http://127.0.0.1:9119` (FastAPI backend).

## Сборка

```bash
npm run build
```

Сборка попадает в `../hermes_cli/web_dist/`; FastAPI отдает ее как статическое SPA. Собранные assets включаются в Python-пакет через `pyproject.toml` package-data.

## Структура

```text
src/
├── components/ui/   # Переиспользуемые UI-примитивы
├── lib/
│   ├── api.ts       # API-клиент с типизированными fetch wrappers
│   └── utils.ts     # cn() helper для объединения Tailwind-классов
├── pages/
│   ├── StatusPage   # Статус агента, активные и недавние сессии
│   ├── ConfigPage   # Динамический редактор config по schema backend
│   └── EnvPage      # Управление API-ключами
├── App.tsx          # Основной layout и навигация
├── main.tsx         # React entry point
└── index.css        # Tailwind imports и переменные темы
```

## Правила типографики и контраста

Перед изменением UI-стилей проверьте эти правила: они сохраняют читаемость dashboard во всех встроенных темах.

- Минимальный размер основного текста: `text-xs` (12px / 0.75rem).
- Не используйте opacity ниже `0.7` для текста.
- Не накладывайте opacity токены друг на друга, например `text-muted-foreground/60`.
- Для обычного текста предпочитайте семантические токены `text-text-primary`, `text-text-secondary`, `text-text-tertiary`, `text-text-disabled`, `text-text-on-accent`.
- Верхний регистр для брендовых элементов включается точечно через `text-display`, а не глобально через `uppercase`.
- Не ставьте `themedBody` или `themedFont` на `<main>`, `App` и другие layout wrappers.
- Для новых UI-элементов предпочитайте семантические цвета (`text-text-*`, `bg-card`, `border-border`, `text-destructive`, `text-success`, `text-warning`) вместо сырых layer refs.

## Шрифты

| Уровень | Классы | Где использовать |
|---|---|---|
| Брендовый chrome | `font-mondwest text-display` или `themedChrome` | Sidebar, заголовки карточек, фильтры |
| Тематический body | `font-mondwest normal-case` или `themedBody` | Содержимое карточек, строки сессий, таблицы |
| Page chrome | `font-expanded` | Заголовок страницы |
| Wordmark | `Typography` + размер/tracking | Sidebar/mobile `Hermes RU Iola` |
| Технический текст | `font-mono-ui`, `font-mono`, `font-courier` | Model slugs, env keys, YAML, URL |

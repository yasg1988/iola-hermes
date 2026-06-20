# План добавления провайдеров Iola

Hermes RU Iola сохраняет plugin-архитектуру upstream Hermes. Если провайдер
совместим с OpenAI Chat Completions API, добавляйте его как provider plugin,
а не через правки ядра.

## Основное правило

Встроенный провайдер нужен только тогда, когда требуется отдельная UX-логика:

- специальные инструкции настройки;
- curated список моделей;
- нестандартные поля запроса;
- OAuth или обновление токенов;
- собственная загрузка списка моделей.

Для обычного OpenAI-compatible endpoint достаточно каталога:

```text
plugins/model-providers/<provider>/
```

Личный provider plugin можно установить без изменения репозитория:

```text
$HERMES_HOME/plugins/model-providers/<provider>/
```

## Шаблон

Базовый шаблон лежит здесь:

```text
docs/templates/model-provider-plugin/
```

Скопируйте его в `plugins/model-providers/<provider>/`, затем замените:

- `your-provider`
- `YOUR_PROVIDER_API_KEY`
- `https://api.your-provider.example/v1`
- модели по умолчанию;
- ссылку регистрации;
- описание провайдера.

## Приоритеты

1. OpenAI-compatible агрегаторы и hosted inference API.
2. Провайдеры с сильной поддержкой русского языка.
3. Native-провайдеры со своим API — после стабилизации npm wrapper,
   desktop-сборок и русской локали.

## Проверка

Для каждого нового провайдера выполните smoke test:

```bash
python -m hermes_cli.main chat -q "Ответь одним предложением по-русски." --provider <provider> --model <model>
```

Затем запустите целевые тесты:

```bash
python -m pytest tests/hermes_cli/test_runtime_provider_resolution.py tests/agent/test_auxiliary_named_custom_providers.py -q
```

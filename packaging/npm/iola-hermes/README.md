# iola-hermes

![Hermes RU Iola](https://raw.githubusercontent.com/yasg1988/iola-hermes/main/assets/iola-hermes-banner.png)

`iola-hermes` — npm wrapper для **Hermes RU Iola**, русскоязычного форка
оригинального проекта NousResearch Hermes Agent.

## Установка

```bash
npm install -g iola-hermes
iola-hermes
```

Пакет устанавливает Python backend из PyPI той же версии, что и npm-пакет:

```text
iola-hermes
```

После установки команда `iola-hermes` запускает Hermes CLI через
`python -m hermes_cli.main`.

## Требования

- Node.js 18+
- Python `>=3.11,<3.15`
- pip

Если backend уже установлен вручную, можно пропустить Python-установку:

```bash
IOLA_HERMES_SKIP_PYTHON_INSTALL=1 npm install -g iola-hermes
```

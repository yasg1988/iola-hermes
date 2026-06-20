import { defineLocale } from './define-locale'

export const ru = defineLocale({
  common: {
    apply: 'Применить',
    back: 'Назад',
    save: 'Сохранить',
    saving: 'Сохранение...',
    cancel: 'Отмена',
    change: 'Изменить',
    choose: 'Выбрать',
    clear: 'Очистить',
    close: 'Закрыть',
    collapse: 'Свернуть',
    confirm: 'Подтвердить',
    connect: 'Подключить',
    connecting: 'Подключение...',
    continue: 'Продолжить',
    copied: 'Скопировано',
    copy: 'Копировать',
    copyFailed: 'Не удалось скопировать',
    delete: 'Удалить',
    docs: 'Документация',
    done: 'Готово',
    error: 'Ошибка',
    failed: 'Ошибка',
    free: 'Бесплатно',
    loading: 'Загрузка...',
    notSet: 'Не задано',
    refresh: 'Обновить',
    remove: 'Удалить',
    replace: 'Заменить',
    retry: 'Повторить',
    run: 'Запустить',
    send: 'Отправить',
    set: 'Задать',
    skip: 'Пропустить',
    update: 'Обновить',
    on: 'Вкл',
    off: 'Выкл'
  },

  boot: {
    ready: 'Hermes RU Iola готов',
    desktopBootFailedWithMessage: message => `Не удалось запустить desktop: ${message}`,
    steps: {
      connectingGateway: 'Подключение к desktop gateway',
      loadingSettings: 'Загрузка настроек Hermes',
      loadingSessions: 'Загрузка последних сессий',
      startingDesktopConnection: 'Запуск desktop-подключения',
      startingHermesDesktop: 'Запуск Hermes RU Iola...'
    },
    errors: {
      backgroundExited: 'Фоновый процесс Hermes завершился.',
      backgroundExitedDuringStartup: 'Фоновый процесс Hermes завершился во время запуска.',
      backendStopped: 'Backend остановлен',
      desktopBootFailed: 'Не удалось запустить desktop',
      gatewaySignInRequired: 'Требуется вход в gateway',
      ipcBridgeUnavailable: 'Desktop IPC bridge недоступен.'
    },
    failure: {
      title: 'Hermes RU Iola не запустился',
      description: 'Фоновый gateway не поднялся. Попробуйте восстановление ниже. Чаты и настройки не удаляются.',
      retry: 'Повторить',
      repairInstall: 'Восстановить установку',
      useLocalGateway: 'Использовать локальный gateway',
      openLogs: 'Открыть логи'
    }
  },

  language: {
    label: 'Язык',
    description: 'Выберите язык интерфейса.',
    saving: 'Сохранение языка...',
    saveError: 'Не удалось сохранить язык.',
    switchTo: 'Переключить на',
    searchPlaceholder: 'Поиск языка...',
    noResults: 'Языки не найдены'
  },

  settings: {
    closeSettings: 'Закрыть настройки',
    exportConfig: 'Экспорт конфигурации',
    importConfig: 'Импорт конфигурации',
    resetToDefaults: 'Сбросить по умолчанию',
    resetConfirm: 'Сбросить настройки к значениям по умолчанию?',
    exportFailed: 'Не удалось экспортировать конфигурацию',
    resetFailed: 'Не удалось сбросить настройки',
    nav: {
      providers: 'Провайдеры',
      providerAccounts: 'Аккаунты провайдеров',
      providerApiKeys: 'API-ключи провайдеров',
      gateway: 'Gateway',
      apiKeys: 'API-ключи',
      keysTools: 'Ключи и инструменты',
      keysSettings: 'Ключи и настройки',
      mcp: 'MCP',
      archivedChats: 'Архив чатов',
      about: 'О программе',
      notifications: 'Уведомления'
    },
    appearance: {
      title: 'Внешний вид',
      intro: 'Настройте тему и отображение интерфейса.',
      colorMode: 'Цветовой режим',
      colorModeDesc: 'Светлая, тёмная или системная тема.',
      product: 'Hermes RU Iola',
      productDesc: 'Русская сборка Hermes Agent от Iola.'
    },
    about: {
      heading: 'Hermes RU Iola',
      version: value => `Версия ${value}`,
      versionUnavailable: 'Версия недоступна',
      updates: 'Обновления',
      checkNow: 'Проверить',
      checking: 'Проверка...',
      seeWhatsNew: 'Что нового',
      releaseNotes: 'Заметки о релизе',
      onLatest: 'Установлена последняя версия',
      installing: 'Установка...',
      cantUpdate: 'Не удалось обновить',
      cantReach: 'Не удалось подключиться к серверу обновлений'
    },
    providers: {
      connectAccount: 'Подключить аккаунт',
      haveApiKey: 'У меня есть API-ключ',
      intro: 'Настройте провайдеров моделей и ключи доступа.',
      connected: 'Подключено',
      collapse: 'Свернуть',
      connectAnother: 'Подключить ещё',
      otherProviders: 'Другие провайдеры',
      disconnect: 'Отключить',
      noProviderKeys: 'Ключи провайдеров не настроены',
      searchKeys: 'Поиск ключей...',
      noKeysMatch: 'Ключи не найдены',
      loading: 'Загрузка провайдеров...'
    }
  },

  commandCenter: {
    paletteTitle: 'Центр команд',
    searchPlaceholder: 'Поиск команд, сессий и настроек...',
    commandCenter: 'Центр команд',
    appearance: 'Внешний вид',
    settings: 'Настройки',
    refresh: 'Обновить',
    refreshing: 'Обновление...',
    noResults: 'Ничего не найдено',
    restartGateway: 'Перезапустить gateway',
    updateHermes: 'Обновить Hermes RU Iola'
  },

  messaging: {
    search: 'Поиск мессенджеров...',
    loading: 'Загрузка мессенджеров...',
    loadFailed: 'Не удалось загрузить мессенджеры',
    unknown: 'Неизвестно',
    credentialsSet: 'Учётные данные заданы',
    needsSetup: 'Требуется настройка',
    gatewayStopped: 'Gateway остановлен',
    getCredentials: 'Получить ключи',
    openSetupGuide: 'Открыть инструкцию',
    required: 'Обязательно',
    recommended: 'Рекомендуется',
    noTokenNeeded: 'Токен не требуется',
    enabled: 'Включено',
    disabled: 'Отключено',
    unsavedChanges: 'Есть несохранённые изменения',
    saving: 'Сохранение...',
    saveChanges: 'Сохранить изменения',
    saved: 'Сохранено',
    replaceValue: 'Заменить значение',
    openDocs: 'Открыть документацию',
    restartToApply: 'Перезапустите gateway, чтобы применить изменения',
    restartToReconnect: 'Перезапустите gateway для переподключения'
  },

  desktop: {
    desktopCommands: 'Команды Hermes RU Iola',
    providerCredentialRequired: 'Нужны учётные данные провайдера',
    emptySlashCommand: 'Введите команду',
    yoloTitle: 'YOLO-режим',
    sttDisabled: 'Распознавание речи отключено',
    sessionBusy: 'Сессия занята',
    modelSwitchFailed: 'Не удалось переключить модель',
    openImage: 'Открыть изображение',
    downloadImage: 'Скачать изображение',
    clipboard: 'Буфер обмена'
  },

  errors: {
    genericFailure: 'Что-то пошло не так',
    boundaryTitle: 'Ошибка интерфейса',
    boundaryDesc: 'Перезагрузите окно или откройте логи.',
    reloadWindow: 'Перезагрузить окно',
    openLogs: 'Открыть логи'
  }
})

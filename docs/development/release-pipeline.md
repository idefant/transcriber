# Пайплайн релиза

Этот документ описывает, как релизы Transcriber собираются, публикуются и доставляются пользователям через автоматические обновления.

## Обзор

Релизы полностью автоматизированы через GitHub Actions. Триггером служит push git-тега, соответствующего `v*` (например, `v1.2.3` или `v1.2.3-beta.1`). Кроме простановки тега, никакого ручного вмешательства не требуется.

Доставка обновлений использует два канала — **stable** и **unstable** — публикуемых в виде JSON-манифестов на GitHub Pages. Tauri updater во время выполнения запрашивает соответствующий манифест.

## Версионирование

Единственный источник истины для версии — это git-тег. Перед сборкой CI запускает `node scripts/set-version.mjs <version>`, который записывает одну и ту же версию в три файла:

- `package.json` → `version`
- `src-tauri/tauri.conf.json` → `version`
- `src-tauri/Cargo.toml` → `[package] version`

Эти файлы не следует редактировать вручную для релизов.

### Stable и pre-release

Тег считается pre-release, если его версия содержит `-` (например, `v1.2.0-beta.1`, `v2.0.0-alpha.3`). Для таких тегов CI устанавливает `prerelease: true`. GitHub помечает релиз как «Latest» только для тегов, не являющихся pre-release.

## Workflow релиза (`.github/workflows/release.yml`)

Шаги:

1. `actions/checkout` с `fetch-depth: 0` (нужно для просмотра полной истории для CHANGELOG).
2. Node 24 + Rust stable + `swatinem/rust-cache` для артефактов Cargo.
3. Версия извлекается из тега (`v1.2.3` → `1.2.3`); флаг pre-release определяется по наличию `-` в версии.
4. `node scripts/set-version.mjs` синхронизирует версию во всех трёх манифестах.
5. `node scripts/extract-changelog.mjs` извлекает раздел с заметками о релизе из `CHANGELOG.md`.
6. `npm ci` устанавливает зависимости фронтенда.
7. `tauri-apps/tauri-action@v0` собирает установщик NSIS, подписывает артефакт обновления, создаёт GitHub Release и прикрепляет `latest.json` (манифест Tauri updater).
8. `gh release download` скачивает собранный `latest.json`.
9. Манифест безусловно копируется в `unstable.json`, а в `stable.json` — только для тегов, не являющихся pre-release, после чего коммитится в ветку `gh-pages`.
10. Workflow загружает эти JSON-файлы как артефакт GitHub Pages и разворачивает их с помощью `actions/deploy-pages`.

Необходимые секреты: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

## Sentry

Sentry необязателен для сборки и работы приложения. Если DSN или переменные CI отсутствуют, Sentry не инициализируется, source maps и PDB не загружаются, а релиз продолжает собираться и публиковаться как обычно.

Для фронтенда добавьте GitHub Actions variable `VITE_SENTRY_DSN`, для Rust — secret `SENTRY_DSN_RUST`. В оба проекта передаётся одно имя релиза `transcriber@<версия>`; для стабильного тега используется окружение `production`, для pre-release — `canary`.

Чтобы CI загрузил диагностические файлы, добавьте secret `SENTRY_AUTH_TOKEN` со scope `org:ci`, а также Actions variables `SENTRY_ORG`, `SENTRY_PROJECT_REACT` и `SENTRY_PROJECT_RUST`. Vite загружает source maps и удаляет их из итогового артефакта. После сборки CI загружает Rust PDB вместе с исходным контекстом в проект `SENTRY_PROJECT_RUST`. GitHub Release остаётся черновиком, пока эти загрузки не завершатся, затем публикуется.

## Каналы обновлений

Два JSON-файла на GitHub Pages служат манифестами обновлений:

| Файл            | Обновляется при                             | Используется, когда                                        |
| --------------- | ------------------------------------------- | ---------------------------------------------------------- |
| `stable.json`   | Только для тегов, не являющихся pre-release | По умолчанию; `isOfferUnstableVersionsEnabled` равно false |
| `unstable.json` | Для каждого тега                            | `isOfferUnstableVersionsEnabled` равно true                |

Оба файла находятся в ветке `gh-pages` репозитория и также разворачиваются на GitHub Pages workflow'ом релиза. Для репозитория должен быть включён GitHub Pages с источником `GitHub Actions`, а не `Deploy from a branch`.

## Ключи подписи

Tauri updater требует пару ключей minisign для проверки подлинности обновления.

Сгенерируйте пару ключей один раз:

```bash
npm run tauri signer generate -- -w transcriber-updater.key
```

- `transcriber-updater.key` — приватный ключ. **Никогда не коммитьте этот файл.** Храните его в безопасном месте.
- `transcriber-updater.key.pub` — публичный ключ. Коммитится в репозиторий и копируется в `tauri.conf.json` под `plugins.updater.pubkey`.

Добавьте приватный ключ и его пароль в секреты репозитория GitHub:

- `TAURI_SIGNING_PRIVATE_KEY` — содержимое `transcriber-updater.key`.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — пароль, выбранный при генерации ключа.

**Потеря приватного ключа означает, что существующие установки никогда не смогут обновиться автоматически.** Сохраните резервную копию в надёжном месте.

## Доставка обновлений в приложении

Rust-часть находится в `src-tauri/src/updater.rs`. Предоставляются две команды Tauri:

- `check_for_update(offer_unstable: bool)` — запрашивает соответствующий эндпоинт, сохраняет найденный `Update` в управляемом состоянии `PendingUpdate`, возвращает `UpdateInfo { version, notes }` или `null`.
- `download_and_install_update()` — берёт сохранённый `Update`, скачивает его, генерирует события `updater://progress` с payload `{ downloaded, total }`, затем вызывает `app.restart()`.

Фронтенд связывается с ними через `src/shared/updaterApi.ts`. Общее состояние фронтенда для обнаружения обновлений, кешированной ожидающей версии и прогресса установки находится в `src/stores/updaterStore.ts`, а видимость/активная секция модального окна настроек — в `src/stores/uiStore.ts`.

При запуске `UpdateChecker` в `App.tsx` выполняет одну тихую проверку после загрузки настроек, но только если `isUpdateNotificationsEnabled` равно true. Если обновление найдено, в правом нижнем углу появляется уведомление с действием `Download`, встроенным в Ant Design 10-секундным индикатором прогресса (`showProgress: true`) и `pauseOnHover: true`. Клик по `Download` не запускает updater — он открывает существующую вкладку настроек `About`.

Полный UI обновления (кнопка ручной проверки, кешированная ожидающая версия, кнопка установки, прогресс загрузки, переключатель уведомлений об обновлениях, переключатель нестабильного канала) находится в `src/app/AppSettingsModal/AboutSettingsTab`. Переход на вкладку `About` всегда запускает свежую проверку обновлений, но кешированный результат из `updaterStore` показывается сразу, поэтому действие установки уже видно, если запуск обнаружил версию ранее.

Когда `isOfferUnstableVersionsEnabled` равно true, приложение по-прежнему проверяет `unstable.json` точно так же, как раньше. Этот манифест всегда указывает на самый новый опубликованный релиз в целом, поэтому пользователям на нестабильном канале может быть предложен стабильный релиз, если именно он является последним доступным.

## Формат CHANGELOG

`CHANGELOG.md` использует формат [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Каждый раздел релиза начинается с `## [X.Y.Z] - YYYY-MM-DD`. Скрипт `scripts/extract-changelog.mjs` извлекает текст между соответствующим заголовком `## [X.Y.Z]` и следующим заголовком `## `; результат используется как тело GitHub Release.

## Брендинг Canary для pre-release

Pre-release сборки (теги, содержащие `-`, например `v0.1.0-alpha.1`) собираются в отдельном варианте canary, который визуально отличается от stable-сборок, оставаясь при этом тем же приложением (те же `productName` и `identifier`, поэтому путь установщика и данные пользователя общие).

Изменения, специфичные для canary, применяемые во время сборки:

| Аспект                 | Значение                     |
| ---------------------- | ---------------------------- |
| Иконки бандла          | `src-tauri/icons-canary/`    |
| Заголовок окна         | `Transcriber Canary`         |
| Канал фронтенда        | `VITE_APP_CHANNEL=canary`    |
| Бейдж на вкладке About | тег «Canary» рядом с версией |

### Переопределение конфигурации

`src-tauri/tauri.canary.conf.json` — это частичный конфиг Tauri, который переопределяет только заголовок окна и `bundle.icon`. Он применяется через `--config src-tauri/tauri.canary.conf.json`.

Важно: поскольку переопределение также заново определяет `app.windows`, общие структурные поля окна нужно явно повторить там, а не полагаться на то, что они безопасно унаследуются из `tauri.conf.json`. Держите их синхронизированными с базовым конфигом при изменении поведения оболочки:

- `decorations`
- `shadow`
- `width` / `height`
- `minWidth` / `minHeight`
- `visible`

### Автоматизация CI

В `.github/workflows/release.yml` шаг `Build and publish Tauri release` устанавливает:

- `env.VITE_APP_CHANNEL` — `canary` для pre-release тегов, иначе `stable`.
- `with.args` — `--config src-tauri/tauri.canary.conf.json` для pre-release тегов, иначе пусто.

Pre-release релизы, как и прежде, продолжают попадать в канал обновлений `unstable`.

### Локальная сборка canary

```bash
npm run build:tauri:canary
```

Это использует `cross-env` для установки `VITE_APP_CHANNEL=canary` и передаёт переопределение конфига canary в `tauri build`.

## Примечание про SmartScreen

Подпись minisign проверяет целостность обновления внутри Tauri updater. Это **не** сертификат подписи кода Authenticode. Без сертификата Authenticode Windows SmartScreen может показать предупреждение при первой установке пользователями (но не при тихих обновлениях). Подпись кода — это отдельный платный шаг, не входящий в этот пайплайн.

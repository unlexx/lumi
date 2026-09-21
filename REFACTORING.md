# Refactoring Plan

## Goal

Разбить монолитный App.vue на компоненты и composables.

## Principles

1. **Dumb components** — компоненты эмитят события, родитель обрабатывает.
2. **Types in `src/types.ts`** — все интерфейсы в одном месте.
3. **Composables** — логика отдельно от UI.
4. **Scoped styles** — стили в компонентах, не в App.vue.

## Current state (done)

- ✅ `types.ts` — все интерфейсы
- ✅ `useLibrary`, `usePosters`, `usePlayer`, `useWatched`, `useContinueWatching`, `useKeyboard`, `useUndefined`
- ✅ `MediaPoster.vue`, `MovieCard.vue`, `ShowCard.vue`

## In progress

- ⏳ `ContextMenu.vue` — вынести меню «⋮» из MovieCard / ShowCard
- ⏳ `MovieModal.vue`, `ShowModal.vue`, `MatchModal.vue` — вынести из App.vue
- ⏳ `LibraryView.vue` — секция библиотеки
- ⏳ `App.vue` — только toolbar + переключатель views

## Design decisions

### ContextMenu — слот, не массив

Меню сильно разное у фильмов и сериалов. Слот гибче массива items.

### Emits vs direct calls

Компоненты эмитят события (`@play`, `@match`). Родитель обрабатывает через composables.
Пример: `MovieCard` эмитит `play(path, startPosition)`, `App.vue` вызывает `play` из `usePlayer`.

### progressPercent / showProgress — где живут

В карточках (`MovieCard`, `ShowCard`), не в App.vue. Логика карточки — в карточке.

### uid и manual match

Для фильмов — `uid` в `matchTarget`, обновление `media_items`.
Для сериалов — сопоставление из меню «⋮» не обновляет `media_items` (только через «Неопределённое»).

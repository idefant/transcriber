import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { OverlayShowPayload, OverlayVariant } from './types';

import './shadow.scss';

// Тень карточки живёт в отдельном окне: окно карточки равно карточке и не может
// нарисовать тень внутри себя, а прозрачное поле вокруг тени не должно
// перехватывать клики. Это окно не интерактивно и лишь повторяет видимость
// карточки, поэтому обходится без React.
const variantClasses: Record<OverlayVariant, string> = {
  bottom: 'shadowBottom',
  center: 'shadowCenter',
};

const shadow = document.querySelector('#shadow');

const show = (variant: OverlayVariant): void => {
  shadow?.setAttribute('class', `shadow ${variantClasses[variant]} shadowVisible`);
};

const hide = (): void => {
  shadow?.classList.remove('shadowVisible');
};

void listen<OverlayShowPayload>('show-overlay', (event) => {
  show(event.payload.variant);
});

void listen('hide-overlay', hide);

// Окно, созданное для второго монитора, может смонтироваться после отправки
// события `show-overlay`, поэтому после подписки оно восстанавливает состояние.
const payload = await invoke<OverlayShowPayload | null>('get_overlay_state');

if (payload) {
  show(payload.variant);
}

import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import '@fontsource-variable/inter/standard.css';
import '@fontsource-variable/jetbrains-mono/wght.css';
import '#/styles/global.scss';

import App from '#/App';

document.addEventListener('contextmenu', (event) => {
  event.preventDefault();
});

const rootElement = document.querySelector('#root');

if (!(rootElement instanceof HTMLElement)) {
  throw new TypeError('Root element was not found');
}

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);

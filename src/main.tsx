import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import '#/styles/global.scss';

import App from '#/App';

const rootElement = document.querySelector('#root');

if (!(rootElement instanceof HTMLElement)) {
  throw new TypeError('Root element was not found');
}

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);

import React from 'react';
import { createRoot } from 'react-dom/client';

import RecordingOverlay from './RecordingOverlay';

import './styles.scss';

document.addEventListener('contextmenu', (event) => {
  event.preventDefault();
});

const root = document.querySelector('#root');

if (root !== null) {
  createRoot(root).render(
    <React.StrictMode>
      <RecordingOverlay />
    </React.StrictMode>,
  );
}

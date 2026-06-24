/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_APP_CHANNEL?: 'stable' | 'canary';
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

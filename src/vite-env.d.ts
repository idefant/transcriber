/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_APP_CHANNEL?: 'stable' | 'canary';
  readonly VITE_SENTRY_DSN?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

declare const __APP_VERSION__: string;

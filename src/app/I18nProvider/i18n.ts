import i18next, { type i18n as I18n } from 'i18next';
import { initReactI18next } from 'react-i18next';

import { defaultLanguage, defaultNamespace, resources } from './resources';

declare global {
  var transcriberI18n: I18n | undefined;
}

/** Единый экземпляр локализации, сохраняемый между обновлениями модулей Vite. */
export const i18n = globalThis.transcriberI18n ?? i18next.createInstance();

if (globalThis.transcriberI18n === undefined) {
  globalThis.transcriberI18n = i18n;

  void i18n.use(initReactI18next).init({
    defaultNS: defaultNamespace,
    fallbackLng: defaultLanguage,
    interpolation: {
      escapeValue: false,
    },
    lng: defaultLanguage,
    resources,
  });
}

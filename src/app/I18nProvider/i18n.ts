import i18next from 'i18next';
import { initReactI18next } from 'react-i18next';

import { defaultLanguage, defaultNamespace, resources } from './resources';

export const i18n = i18next.createInstance();

void i18n.use(initReactI18next).init({
  defaultNS: defaultNamespace,
  fallbackLng: defaultLanguage,
  interpolation: {
    escapeValue: false,
  },
  lng: defaultLanguage,
  resources,
});

import { type FC, type ReactNode, useEffect } from 'react';
import i18next from 'i18next';
import { I18nextProvider, initReactI18next } from 'react-i18next';

import { defaultLanguage, defaultNamespace, resources } from './resources';

import type { EffectiveUiLanguage } from '#/models/Settings';
import { useSettingsStore } from '#/stores';

interface I18nProviderProps {
  children: ReactNode;
}

const i18n = i18next.createInstance();

void i18n.use(initReactI18next).init({
  defaultNS: defaultNamespace,
  fallbackLng: defaultLanguage,
  interpolation: {
    escapeValue: false,
  },
  lng: defaultLanguage,
  resources,
});

const changeLanguage = (language: EffectiveUiLanguage) => {
  if (i18n.language !== language) {
    void i18n.changeLanguage(language);
  }
};

const I18nProvider: FC<I18nProviderProps> = ({ children }) => {
  const settings = useSettingsStore((s) => s.settings);

  useEffect(() => {
    changeLanguage(settings.effectiveUiLanguage);
  }, [settings.effectiveUiLanguage]);

  changeLanguage(settings.effectiveUiLanguage);

  return <I18nextProvider i18n={i18n}>{children}</I18nextProvider>;
};

export default I18nProvider;

import { type FC, type ReactNode, useEffect } from 'react';
import { I18nextProvider } from 'react-i18next';

import { i18n } from './i18n';

import type { EffectiveUiLanguage } from '#/models/Settings';
import { useSettingsStore } from '#/stores';

interface I18nProviderProps {
  children: ReactNode;
}

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

  return <I18nextProvider i18n={i18n}>{children}</I18nextProvider>;
};

export default I18nProvider;

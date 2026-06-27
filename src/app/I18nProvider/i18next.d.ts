import 'i18next';

import type { I18nDefaultNamespace, I18nResources } from './resources';

declare module 'i18next' {
  interface CustomTypeOptions {
    defaultNS: I18nDefaultNamespace;
    resources: I18nResources;
    strictKeyChecks: true;
  }
}

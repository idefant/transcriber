import type { ProviderKind } from './Provider';

export type ModelTask = 'stt' | 'postProcess';

export interface CuratedModelProviderEntry {
  apiId: string;
  isRecommended: boolean;
  provider: ProviderKind;
}

export interface CuratedModelInfo {
  key: string;
  label: string;
  providerEntries: CuratedModelProviderEntry[];
  task: ModelTask;
  providerKinds: ProviderKind[];
}

import type { ProviderKind } from './Provider';

export type ModelTask = 'stt' | 'postProcess';

export interface CuratedModelInfo {
  key: string;
  label: string;
  task: ModelTask;
  providerKinds: ProviderKind[];
}

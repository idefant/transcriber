import { invoke } from '@tauri-apps/api/core';

import type { CuratedModelInfo } from '#/models/Catalog';

export const getModelCatalog = () => invoke<CuratedModelInfo[]>('get_model_catalog');

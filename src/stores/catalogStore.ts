import { create } from 'zustand';

import * as catalogApi from '#/shared/catalogApi';

import type { CuratedModelInfo } from '#/models/Catalog';

interface CatalogState {
  catalog: CuratedModelInfo[];
  isLoading: boolean;
  load: () => Promise<void>;
}

export const useCatalogStore = create<CatalogState>((set) => ({
  catalog: [],
  isLoading: true,

  load: async () => {
    set({ isLoading: true });
    try {
      const catalog = await catalogApi.getModelCatalog();
      set({ catalog });
    } catch {
      // Каталог статический; при ошибке оставляем пустой массив.
    } finally {
      set({ isLoading: false });
    }
  },
}));

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
      // Catalog is static; keep empty array on error.
    } finally {
      set({ isLoading: false });
    }
  },
}));

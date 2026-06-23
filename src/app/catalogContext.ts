import { createContext, useContext } from 'react';

import type { CuratedModelInfo } from '#/models/Catalog';

interface CatalogContextValue {
  catalog: CuratedModelInfo[];
  isLoading: boolean;
}

export const CatalogContext = createContext<CatalogContextValue | undefined>(undefined);

export const useCatalog = () => {
  const value = useContext(CatalogContext);

  if (value === undefined) {
    throw new Error('useCatalog must be used inside CatalogProvider');
  }

  return value;
};

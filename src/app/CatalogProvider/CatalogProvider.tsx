import { type FC, type ReactNode, useEffect, useMemo, useState } from 'react';

import { CatalogContext } from '#/app/catalogContext';
import * as catalogApi from '#/shared/catalogApi';

import type { CuratedModelInfo } from '#/models/Catalog';

interface CatalogProviderProps {
  children: ReactNode;
}

const CatalogProvider: FC<CatalogProviderProps> = ({ children }) => {
  const [catalog, setCatalog] = useState<CuratedModelInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const loadCatalog = async () => {
      try {
        const data = await catalogApi.getModelCatalog();

        setCatalog(data);
      } catch {
        // Catalog is static; keep empty array on error.
      } finally {
        setIsLoading(false);
      }
    };

    queueMicrotask(() => {
      void loadCatalog().catch(() => {
        // loadCatalog handles all errors internally.
      });
    });
  }, []);

  const contextValue = useMemo(
    () => ({
      catalog,
      isLoading,
    }),
    [catalog, isLoading],
  );

  return <CatalogContext.Provider value={contextValue}>{children}</CatalogContext.Provider>;
};

export default CatalogProvider;

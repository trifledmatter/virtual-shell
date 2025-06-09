import React, { useEffect, useRef, createContext, useContext } from "react";

import { configure, InMemory } from '@zenfs/core';
import { IndexedDB } from '@zenfs/dom';
import { Zip } from '@zenfs/archives';

// zenfs is assumed to be available as an ESM import
// If zenfs is not available as an ESM, you may need to load it differently
// import zenfs from 'zenfs';

// For this example, we will assume zenfs is available globally as window.zenfs
// and provides async methods: writeFile, mkdir, unlink, symlink, etc.

interface VfsEventProviderProps {
  children: React.ReactNode;
}

interface VfsContextType {
  isReady: boolean;
  readFile: (path: string) => Promise<Uint8Array>;
  readdir: (path: string) => Promise<string[]>;
  stat: (path: string) => Promise<{ isFile: boolean; isDirectory: boolean }>;
  getAllFiles: () => Promise<Array<{ path: string; content: Uint8Array }>>;
}

const VfsContext = createContext<VfsContextType | null>(null);

export const useVfs = () => {
  const context = useContext(VfsContext);
  if (!context) {
    throw new Error('useVfs must be used within VfsEventProvider');
  }
  return context;
};

const VFS_EVENT_MAP = {
  "vfs-create-file": async (detail: any) => {
    const { fs } = await import('@zenfs/core');
    await fs.promises.writeFile(detail.path, new Uint8Array(detail.content));
  },
  "vfs-write-file": async (detail: any) => {
    const { fs } = await import('@zenfs/core');
    await fs.promises.writeFile(detail.path, new Uint8Array(detail.content));
  },
  "vfs-delete": async (detail: any) => {
    const { fs } = await import('@zenfs/core');
    await fs.promises.unlink(detail.path);
  },
  "vfs-create-dir": async (detail: any) => {
    const { fs } = await import('@zenfs/core');
    await fs.promises.mkdir(detail.path, { recursive: true });
  },
  "vfs-create-symlink": async (detail: any) => {
    const { fs } = await import('@zenfs/core');
    await fs.promises.symlink(detail.target, detail.path);
  },
};

export const VfsEventProvider: React.FC<VfsEventProviderProps> = ({ children }) => {
  const handlerRef = useRef(VFS_EVENT_MAP);
  const [zenfsReady, setZenfsReady] = React.useState(false);

  // Initialize ZenFS
  useEffect(() => {
    (async () => {
      try {
        await configure({
          mounts: {
            '/': IndexedDB, // persistent storage at root
            '/tmp': InMemory, // in-memory at /tmp
          },
          addDevices: true,
        });
        setZenfsReady(true);
        console.log('[zenfs] ZenFS configured and ready');
      } catch (err) {
        console.error('[zenfs] Failed to configure ZenFS:', err);
      }
    })();
  }, []);

  // VFS operations
  const readFile = async (path: string): Promise<Uint8Array> => {
    if (!zenfsReady) throw new Error('ZenFS not ready');
    const { fs } = await import('@zenfs/core');
    return await fs.promises.readFile(path);
  };

  const readdir = async (path: string): Promise<string[]> => {
    if (!zenfsReady) throw new Error('ZenFS not ready');
    const { fs } = await import('@zenfs/core');
    return await fs.promises.readdir(path);
  };

  const stat = async (path: string) => {
    if (!zenfsReady) throw new Error('ZenFS not ready');
    const { fs } = await import('@zenfs/core');
    const stats = await fs.promises.stat(path);
    return {
      isFile: stats.isFile(),
      isDirectory: stats.isDirectory()
    };
  };

  const getAllFiles = async (): Promise<Array<{ path: string; content: Uint8Array }>> => {
    if (!zenfsReady) throw new Error('ZenFS not ready');
    const files: Array<{ path: string; content: Uint8Array }> = [];
    
    const scanDirectory = async (dirPath: string) => {
      try {
        const entries = await readdir(dirPath);
        for (const entry of entries) {
          const fullPath = dirPath === '/' ? `/${entry}` : `${dirPath}/${entry}`;
          try {
            const stats = await stat(fullPath);
            if (stats.isFile) {
              const content = await readFile(fullPath);
              files.push({ path: fullPath, content });
            } else if (stats.isDirectory) {
              await scanDirectory(fullPath);
            }
          } catch (error) {
            console.warn(`Failed to process ${fullPath}:`, error);
          }
        }
      } catch (error) {
        console.warn(`Failed to scan directory ${dirPath}:`, error);
      }
    };

    await scanDirectory('/');
    return files;
  };

  const vfsContextValue: VfsContextType = {
    isReady: zenfsReady,
    readFile,
    readdir,
    stat,
    getAllFiles
  };

  useEffect(() => {
    const handleEvent = (event: Event) => {
      const customEvent = event as CustomEvent;
      const type = event.type;
      const handler = handlerRef.current[type as keyof typeof VFS_EVENT_MAP];
      if (handler && customEvent.detail) {
        handler(customEvent.detail).catch((err: any) => {
          // eslint-disable-next-line no-console
          console.error(`[zenfs] Error handling VFS event '${type}':`, err);
        });
      }
    };

    // Register listeners for all VFS event types
    Object.keys(VFS_EVENT_MAP).forEach((eventType) => {
      window.addEventListener(eventType, handleEvent as EventListener);
    });
    return () => {
      Object.keys(VFS_EVENT_MAP).forEach((eventType) => {
        window.removeEventListener(eventType, handleEvent as EventListener);
      });
    };
  }, []);

  return (
    <VfsContext.Provider value={vfsContextValue}>
      {children}
    </VfsContext.Provider>
  );
};

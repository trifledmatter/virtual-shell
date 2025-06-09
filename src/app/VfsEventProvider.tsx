import React, { useEffect, useRef, createContext, useContext } from "react";

import { configure, InMemory } from '@zenfs/core';
import { IndexedDB } from '@zenfs/dom';
import { Zip } from '@zenfs/archives';

// zenfs better be available as esm or we're screwed
// if it's not, you'll need to figure out how to load it yourself

// assuming zenfs is sitting on window.zenfs like a good little global
// and has all the async methods we need

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

// deal with map objects from rust wasm or regular js objects
const extractVfsData = (detail: any) => {
  if (detail instanceof Map) {
    // rust wasm likes to send maps, because of course it does
    return {
      path: detail.get('path'),
      content: detail.get('content'),
      target: detail.get('target')
    };
  } else {
    // normal js objects for normal people
    return {
      path: detail.path,
      content: detail.content,
      target: detail.target
    };
  }
};

// make sure parent dirs exist because apparently we need to hold everyone's hand
const ensureParentDirs = async (filePath: string) => {
  const { fs } = await import('@zenfs/core');
  const pathParts = filePath.split('/').filter(part => part.length > 0);
  if (pathParts.length <= 1) return; // nothing to do here

  // strip the filename, keep the directories
  const dirParts = pathParts.slice(0, -1);
  let currentPath = '';
  
  for (const part of dirParts) {
    currentPath += '/' + part;
    try {
      await fs.promises.stat(currentPath);
      // dir exists, whatever
    } catch {
      // dir doesn't exist, time to make it
      console.log('[vfs] creating parent dir:', currentPath);
      await fs.promises.mkdir(currentPath, { recursive: true });
    }
  }
};

const VFS_EVENT_MAP = {
  "vfs-create-file": async (detail: any) => {
    const { path, content } = extractVfsData(detail);
    console.log('[vfs] creating file:', path, `(${content?.length || 0} bytes)`);
    const { fs } = await import('@zenfs/core');
    
    // make sure parent dirs exist first
    await ensureParentDirs(path);
    
    await fs.promises.writeFile(path, new Uint8Array(content || []));
    console.log('[vfs] file created:', path);
  },
  "vfs-write-file": async (detail: any) => {
    const { path, content } = extractVfsData(detail);
    console.log('[vfs] writing file:', path, `(${content?.length || 0} bytes)`);
    const { fs } = await import('@zenfs/core');
    
    // parent dirs better exist or we'll make them
    await ensureParentDirs(path);
    
    await fs.promises.writeFile(path, new Uint8Array(content || []));
    console.log('[vfs] file written:', path);
  },
  "vfs-delete": async (detail: any) => {
    const { path } = extractVfsData(detail);
    console.log('[vfs] deleting file:', path);
    const { fs } = await import('@zenfs/core');
    await fs.promises.unlink(path);
    console.log('[vfs] file deleted:', path);
  },
  "vfs-create-dir": async (detail: any) => {
    const { path } = extractVfsData(detail);
    console.log('[vfs] creating directory:', path);
    const { fs } = await import('@zenfs/core');
    await fs.promises.mkdir(path, { recursive: true });
    console.log('[vfs] directory created:', path);
  },
  "vfs-create-symlink": async (detail: any) => {
    const { path, target } = extractVfsData(detail);
    console.log('[vfs] creating symlink:', path, '→', target);
    const { fs } = await import('@zenfs/core');
    await fs.promises.symlink(target, path);
    console.log('[vfs] symlink created:', path);
  },
  "vfs-create-zip": async (detail: any) => {
    const { path, content } = extractVfsData(detail);
    console.log('[vfs] creating zip archive:', path, `(${content?.length || 0} bytes)`);
    const { fs } = await import('@zenfs/core');
    
    // make sure parent dirs exist first
    await ensureParentDirs(path);
    
    // store the zip file directly
    await fs.promises.writeFile(path, new Uint8Array(content || []));
    console.log('[vfs] zip archive created:', path);
    
    // also try to mount the zip as a readable filesystem for extraction
    try {
      if (content && content.length > 0) {
        console.log('[vfs] attempting to mount zip as filesystem...');
        const { configure } = await import('@zenfs/core');
        const { Zip } = await import('@zenfs/archives');
        
        // create a mount point for the zip content
        const mountPoint = `/mnt/${path.replace(/\.zip$/, '').replace(/[/\\]/g, '_')}`;
        console.log('[vfs] mounting zip at:', mountPoint);
        
        await configure({
          mounts: {
            [mountPoint]: {
              backend: Zip,
              data: new Uint8Array(content)
            }
          },
          addDevices: false
        });
        
        console.log('[vfs] zip filesystem mounted at:', mountPoint);
      }
    } catch (zipError) {
      console.warn('[vfs] failed to mount zip as filesystem (this is ok):', zipError);
    }
  },
};

export const VfsEventProvider: React.FC<VfsEventProviderProps> = ({ children }) => {
  const handlerRef = useRef(VFS_EVENT_MAP);
  const [zenfsReady, setZenfsReady] = React.useState(false);

  // global callback for wasm to hit directly instead of dom event nonsense
  const handleVfsOperation = React.useCallback(async (operation: string, data: any) => {
    console.log('[vfs] direct callback received:', operation, data);
    
    const handler = handlerRef.current[operation as keyof typeof VFS_EVENT_MAP];
    if (handler && data) {
      try {
        await handler(data);
        console.log('[vfs] direct callback handled:', operation);
      } catch (error) {
        console.error('[vfs] direct callback failed:', operation, error);
      }
    } else {
      console.warn('[vfs] no handler for direct callback:', operation);
    }
  }, []);

  // set up zenfs and expose the global callback
  useEffect(() => {
    // stick the callback on window so wasm can find it
    (window as any).__vfsCallback = handleVfsOperation;
    console.log('[vfs] global callback exposed as window.__vfsCallback');

    (async () => {
      try {
        console.log('[vfs] initializing zenfs...');
        await configure({
          mounts: {
            '/': IndexedDB, // persistent storage because we're not animals
            '/tmp': InMemory, // temp stuff in memory obviously
          },
          addDevices: true,
        });
        setZenfsReady(true);
        console.log('[vfs] zenfs configured - indexeddb persistence enabled');
      } catch (err) {
        console.error('[vfs] zenfs config failed:', err);
      }
    })();

    return () => {
      // clean up the global mess we made
      delete (window as any).__vfsCallback;
    };
  }, [handleVfsOperation]);

  // vfs operations that actually do stuff
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
    console.log('[vfs] scanning all files in zenfs...');
    const files: Array<{ path: string; content: Uint8Array }> = [];
    
    const scanDirectory = async (dirPath: string) => {
      try {
        console.log('[vfs] scanning directory:', dirPath);
        const entries = await readdir(dirPath);
        console.log('[vfs] found entries:', entries);
        
        for (const entry of entries) {
          const fullPath = dirPath === '/' ? `/${entry}` : `${dirPath}/${entry}`;
          try {
            const stats = await stat(fullPath);
            if (stats.isFile) {
              console.log('[vfs] reading file:', fullPath);
              const content = await readFile(fullPath);
              console.log('[vfs] file read:', fullPath, `(${content.length} bytes)`);
              files.push({ path: fullPath, content });
            } else if (stats.isDirectory) {
              console.log('[vfs] entering subdirectory:', fullPath);
              await scanDirectory(fullPath);
            }
          } catch (error) {
            console.warn(`[vfs] failed to process ${fullPath}:`, error);
          }
        }
      } catch (error) {
        console.warn(`[vfs] failed to scan directory ${dirPath}:`, error);
      }
    };

    await scanDirectory('/');
    console.log(`[vfs] total files found: ${files.length}`);
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
      
      console.log(`[vfs] received event: ${type}`, customEvent.detail);
      
      if (handler && customEvent.detail) {
        handler(customEvent.detail).catch((err: any) => {
          console.error(`[vfs] error handling event '${type}':`, err);
        });
      } else if (!handler) {
        console.warn(`[vfs] no handler found for event type: ${type}`);
      } else if (!customEvent.detail) {
        console.warn(`[vfs] event ${type} received but no detail provided`);
      }
    };

    // register listeners for all vfs events on window and document because redundancy
    console.log('[vfs] registering event listeners for:', Object.keys(VFS_EVENT_MAP));
    console.log('[vfs] registering on window and document for better event capture');
    
    Object.keys(VFS_EVENT_MAP).forEach((eventType) => {
      window.addEventListener(eventType, handleEvent as EventListener);
      document.addEventListener(eventType, handleEvent as EventListener);
    });

    // test the event system because trust nothing
    const testEventSystem = () => {
      console.log('[vfs] testing event system...');
      const testEvent = new CustomEvent('vfs-write-file', {
        detail: { path: '/test-event.txt', content: [116, 101, 115, 116] }
      });
      window.dispatchEvent(testEvent);
    };

    // debug listener to catch all custom events because debugging is life
    const debugAllEvents = (event: Event) => {
      if (event.type.startsWith('vfs-')) {
        console.log('[vfs] global listener caught vfs event:', event.type, event);
      }
    };

    // listen everywhere with capture=true to catch everything
    window.addEventListener('vfs-write-file', debugAllEvents, true);
    window.addEventListener('vfs-create-file', debugAllEvents, true);
    document.addEventListener('vfs-write-file', debugAllEvents, true);
    document.addEventListener('vfs-create-file', debugAllEvents, true);

    // test after a delay because timing is everything
    setTimeout(testEventSystem, 1000);

    return () => {
      console.log('[vfs] unregistering vfs event listeners');
      Object.keys(VFS_EVENT_MAP).forEach((eventType) => {
        window.removeEventListener(eventType, handleEvent as EventListener);
        document.removeEventListener(eventType, handleEvent as EventListener);
      });
    };
  }, []);

  return (
    <VfsContext.Provider value={vfsContextValue}>
      {children}
    </VfsContext.Provider>
  );
};

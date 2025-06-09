use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::*;
use serde::{Serialize, Deserialize};
use crate::vfs::{VirtualFileSystem, VfsNode};
use flate2::{Compression, read::DeflateDecoder, write::DeflateEncoder};
use std::io::{Read, Write};
use base64::{Engine as _, engine::general_purpose};

const DB_NAME: &str = "TrifledOS_VFS";
const DB_VERSION: u32 = 1;
const STORE_NAME: &str = "filesystem";
const COMPRESSION_THRESHOLD: usize = 1024; // only compress files > 1kb, not worth it otherwise

#[derive(Serialize, Deserialize, Clone)]
pub struct StoredFile {
    pub path: String,
    pub content: String, // base64 encoded, maybe compressed
    pub compressed: bool,
    pub original_size: usize,
    pub modified: String, // iso timestamp
    pub permissions: [u8; 3], // [user, group, other]
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StoredDirectory {
    pub path: String,
    pub modified: String,
    pub permissions: [u8; 3],
    pub children: HashMap<String, StoredNode>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StoredSymlink {
    pub path: String,
    pub target: String,
    pub modified: String,
    pub permissions: [u8; 3],
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum StoredNode {
    File(StoredFile),
    Directory(StoredDirectory),
    Symlink(StoredSymlink),
}

#[derive(Serialize, Deserialize)]
pub struct StoredVFS {
    pub root: StoredNode,
    pub version: u32,
}

pub struct PersistentStorage {
    db: Option<IdbDatabase>,
}

impl PersistentStorage {
    pub fn new() -> Self {
        Self { 
            db: None 
        }
    }

    /// init indexeddb connection
    pub async fn init(&mut self) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No global window")?;
        let idb_factory = window.indexed_db()?.ok_or("IndexedDB not available")?;
        
        let open_request = idb_factory.open_with_u32(DB_NAME, DB_VERSION)?;
        
        // setup upgrade handler
        let upgrade_closure = Closure::wrap(Box::new(move |event: Event| {
            let target = event.target().unwrap();
            let request: IdbRequest = target.dyn_into().unwrap();
            let db: IdbDatabase = request.result().unwrap().dyn_into().unwrap();
            
            // try to create object store - will fail silently if exists already
            let _ = db.create_object_store(STORE_NAME);
        }) as Box<dyn FnMut(_)>);
        
        open_request.set_onupgradeneeded(Some(upgrade_closure.as_ref().unchecked_ref()));
        upgrade_closure.forget(); // keep closure alive
        
        // wait for database to open using a promise wrapper
        let promise = js_sys::Promise::new(&mut |resolve, _| {
            let success_closure = Closure::wrap(Box::new(move |_: Event| {
                resolve.call0(&JsValue::NULL).unwrap();
            }) as Box<dyn FnMut(_)>);
            
            open_request.set_onsuccess(Some(success_closure.as_ref().unchecked_ref()));
            success_closure.forget();
        });
        
        JsFuture::from(promise).await?;
        
        // get the database from the request
        self.db = Some(open_request.result()?.dyn_into()?);
        
        Ok(())
    }

    /// compress data if above threshold using deflate
    fn compress_data(data: &[u8]) -> Result<(Vec<u8>, bool), Box<dyn std::error::Error>> {
        if data.len() < COMPRESSION_THRESHOLD {
            return Ok((data.to_vec(), false));
        }

        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        let compressed = encoder.finish()?;
        
        // only use compression if it actually saves space
        if compressed.len() < data.len() {
            Ok((compressed, true))
        } else {
            Ok((data.to_vec(), false))
        }
    }

    /// decompress data if compressed using deflate
    fn decompress_data(data: &[u8], compressed: bool) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if !compressed {
            return Ok(data.to_vec());
        }

        let mut decoder = DeflateDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }

    /// convert vfs node to stored format recursively
    fn node_to_stored(&self, node: &VfsNode, path: &str) -> StoredNode {
        match node {
            VfsNode::File { content, permissions, mtime, .. } => {
                let (processed_content, compressed) = Self::compress_data(content)
                    .unwrap_or_else(|_| (content.clone(), false));
                
                StoredNode::File(StoredFile {
                    path: path.to_string(),
                    content: general_purpose::STANDARD.encode(&processed_content),
                    compressed,
                    original_size: content.len(),
                    modified: mtime.to_rfc3339(),
                    permissions: [permissions.user, permissions.group, permissions.other],
                })
            }
            VfsNode::Directory { children, permissions, mtime, .. } => {
                // recursively convert all children
                let mut stored_children = HashMap::new();
                for (name, child) in children {
                    let child_path = if path == "/" {
                        format!("/{}", name)
                    } else {
                        format!("{}/{}", path, name)
                    };
                    stored_children.insert(name.clone(), self.node_to_stored(child, &child_path));
                }
                
                StoredNode::Directory(StoredDirectory {
                    path: path.to_string(),
                    modified: mtime.to_rfc3339(),
                    permissions: [permissions.user, permissions.group, permissions.other],
                    children: stored_children,
                })
            }
            VfsNode::Symlink { target, permissions, mtime, .. } => {
                StoredNode::Symlink(StoredSymlink {
                    path: path.to_string(),
                    target: target.clone(),
                    modified: mtime.to_rfc3339(),
                    permissions: [permissions.user, permissions.group, permissions.other],
                })
            }
        }
    }

    /// convert stored format back to vfs node
    fn stored_to_node(&self, stored: &StoredNode) -> Result<VfsNode, Box<dyn std::error::Error>> {
        match stored {
            StoredNode::File(file) => {
                let decoded_content = general_purpose::STANDARD.decode(&file.content)?;
                let content = Self::decompress_data(&decoded_content, file.compressed)?;
                let mtime = chrono::DateTime::parse_from_rfc3339(&file.modified)?
                    .with_timezone(&chrono::Local);
                
                Ok(VfsNode::File {
                    name: std::path::Path::new(&file.path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    content,
                    permissions: crate::vfs::Permissions::new(
                        file.permissions[0],
                        file.permissions[1], 
                        file.permissions[2]
                    ),
                    mtime,
                })
            }
            StoredNode::Directory(dir) => {
                let mtime = chrono::DateTime::parse_from_rfc3339(&dir.modified)?
                    .with_timezone(&chrono::Local);
                
                // recursively convert all children
                let mut vfs_children = HashMap::new();
                for (name, stored_child) in &dir.children {
                    let child_node = self.stored_to_node(stored_child)?;
                    vfs_children.insert(name.clone(), child_node);
                }
                
                Ok(VfsNode::Directory {
                    name: std::path::Path::new(&dir.path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    children: vfs_children,
                    permissions: crate::vfs::Permissions::new(
                        dir.permissions[0],
                        dir.permissions[1],
                        dir.permissions[2]
                    ),
                    mtime,
                })
            }
            StoredNode::Symlink(link) => {
                let mtime = chrono::DateTime::parse_from_rfc3339(&link.modified)?
                    .with_timezone(&chrono::Local);
                
                Ok(VfsNode::Symlink {
                    name: std::path::Path::new(&link.path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    target: link.target.clone(),
                    permissions: crate::vfs::Permissions::new(
                        link.permissions[0],
                        link.permissions[1],
                        link.permissions[2]
                    ),
                    mtime,
                })
            }
        }
    }

    /// save a single node to indexeddb
    pub async fn save_node(&self, path: &str, node: &VfsNode) -> Result<(), JsValue> {
        let db = self.db.as_ref().ok_or("Database not initialized")?;
        
        // create readwrite transaction
        let transaction = db.transaction_with_str_and_mode(STORE_NAME, IdbTransactionMode::Readwrite)?;
        let store = transaction.object_store(STORE_NAME)?;
        
        let stored_node = self.node_to_stored(node, path);
        let serialized = serde_json::to_string(&stored_node)
            .map_err(|e| JsValue::from_str(&format!("serialization error: {}", e)))?;
        
        // put data and wait for completion
        let request = store.put_with_key(&JsValue::from_str(&serialized), &JsValue::from_str(path))?;
        
        // wait for the put operation to complete
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let success_closure = Closure::wrap(Box::new({
                let request = request.clone();
                move |_: Event| {
                    resolve.call0(&JsValue::NULL).unwrap();
                }
            }) as Box<dyn FnMut(_)>);
            
            let error_closure = Closure::wrap(Box::new({
                let request = request.clone();
                move |_: Event| {
                    let error = match request.error() {
                        Ok(Some(dom_err)) => JsValue::from(dom_err),
                        Ok(None) => JsValue::from_str("unknown save error"),
                        Err(js_err) => js_err,
                    };
                    web_sys::console::log_1(&format!("vfs save failed: {:?}", error).into());
                    reject.call1(&JsValue::NULL, &error).unwrap();
                }
            }) as Box<dyn FnMut(_)>);
            
            request.set_onsuccess(Some(success_closure.as_ref().unchecked_ref()));
            request.set_onerror(Some(error_closure.as_ref().unchecked_ref()));
            success_closure.forget();
            error_closure.forget();
        });
        
        JsFuture::from(promise).await?;
        Ok(())
    }

    /// save entire vfs to indexeddb
    pub async fn save_vfs(&self, vfs: &VirtualFileSystem) -> Result<(), JsValue> {
        web_sys::console::log_1(&"starting vfs save...".into());
        
        let db = self.db.as_ref().ok_or("Database not initialized")?;
        
        // create readwrite transaction
        let transaction = db.transaction_with_str_and_mode(STORE_NAME, IdbTransactionMode::Readwrite)?;
        let store = transaction.object_store(STORE_NAME)?;
        
        // serialize the entire vfs
        let stored_vfs = StoredVFS {
            root: self.node_to_stored(&vfs.root, "/"),
            version: 1,
        };
        
        let serialized = serde_json::to_string(&stored_vfs)
            .map_err(|e| JsValue::from_str(&format!("serialization error: {}", e)))?;
        
        web_sys::console::log_1(&format!("serialized vfs size: {} bytes", serialized.len()).into());
        
        // put data and wait for completion
        let request = store.put_with_key(&JsValue::from_str(&serialized), &JsValue::from_str("__VFS_ROOT__"))?;
        
        // wait for the put operation to complete
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let success_closure = Closure::wrap(Box::new({
                let request = request.clone();
                move |_: Event| {
                    web_sys::console::log_1(&"vfs save completed successfully".into());
                    resolve.call0(&JsValue::NULL).unwrap();
                }
            }) as Box<dyn FnMut(_)>);
            
            let error_closure = Closure::wrap(Box::new({
                let request = request.clone();
                move |_: Event| {
                    let error = match request.error() {
                        Ok(Some(dom_err)) => JsValue::from(dom_err),
                        Ok(None) => JsValue::from_str("unknown save error"),
                        Err(js_err) => js_err,
                    };
                    web_sys::console::log_1(&format!("vfs save failed: {:?}", error).into());
                    reject.call1(&JsValue::NULL, &error).unwrap();
                }
            }) as Box<dyn FnMut(_)>);
            
            request.set_onsuccess(Some(success_closure.as_ref().unchecked_ref()));
            request.set_onerror(Some(error_closure.as_ref().unchecked_ref()));
            success_closure.forget();
            error_closure.forget();
        });
        
        JsFuture::from(promise).await?;
        web_sys::console::log_1(&"vfs save operation finished".into());
        Ok(())
    }

    /// load a single node from indexeddb
    pub async fn load_node(&self, path: &str) -> Result<Option<VfsNode>, JsValue> {
        if path == "/" {
            match self.load_vfs().await {
                Ok(vfs) => Ok(Some(vfs.root)),
                Err(_) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    /// load entire vfs from indexeddb
    pub async fn load_vfs(&self) -> Result<VirtualFileSystem, JsValue> {
        web_sys::console::log_1(&"starting vfs load...".into());
        
        let db = self.db.as_ref().ok_or("Database not initialized")?;
        let transaction = db.transaction_with_str(STORE_NAME)?;
        let store = transaction.object_store(STORE_NAME)?;
        
        let request = store.get(&JsValue::from_str("__VFS_ROOT__"))?;
        
        // wait for get operation to complete
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let success_closure = Closure::wrap(Box::new({
                let request = request.clone();
                move |_: Event| {
                    let result = request.result().unwrap();
                    web_sys::console::log_1(&format!("vfs load got result: {:?}", result).into());
                    resolve.call1(&JsValue::NULL, &result).unwrap();
                }
            }) as Box<dyn FnMut(_)>);
            
            let error_closure = Closure::wrap(Box::new({
                let request = request.clone();
                move |_: Event| {
                    let error = match request.error() {
                        Ok(Some(dom_err)) => JsValue::from(dom_err),
                        Ok(None) => JsValue::from_str("unknown load error"),
                        Err(js_err) => js_err,
                    };
                    web_sys::console::log_1(&format!("vfs load failed: {:?}", error).into());
                    reject.call1(&JsValue::NULL, &error).unwrap();
                }
            }) as Box<dyn FnMut(_)>);
            
            request.set_onsuccess(Some(success_closure.as_ref().unchecked_ref()));
            request.set_onerror(Some(error_closure.as_ref().unchecked_ref()));
            success_closure.forget();
            error_closure.forget();
        });
        
        let result = JsFuture::from(promise).await?;
        
        if result.is_undefined() || result.is_null() {
            web_sys::console::log_1(&"no saved vfs data found, returning fresh vfs".into());
            // no saved data, return fresh vfs
            return Ok(VirtualFileSystem::new());
        }
        
        let serialized = result.as_string()
            .ok_or_else(|| JsValue::from_str("invalid data format"))?;
        
        web_sys::console::log_1(&format!("deserializing vfs data: {} bytes", serialized.len()).into());
        
        let stored_vfs: StoredVFS = serde_json::from_str(&serialized)
            .map_err(|e| JsValue::from_str(&format!("deserialization error: {}", e)))?;

        let root = self.stored_to_node(&stored_vfs.root)
            .map_err(|e| JsValue::from_str(&format!("node conversion error: {}", e)))?;

        let mut vfs = VirtualFileSystem::new();
        vfs.root = root;
        
        web_sys::console::log_1(&"vfs load completed successfully".into());
        Ok(vfs)
    }

    /// delete node from indexeddb
    pub async fn delete_node(&self, _path: &str) -> Result<(), JsValue> {
        let db = self.db.as_ref().ok_or("Database not initialized")?;
        
        // create readwrite transaction  
        let transaction = db.transaction_with_str_and_mode(STORE_NAME, IdbTransactionMode::Readwrite)?;
        let store = transaction.object_store(STORE_NAME)?;
        
        let request = store.delete(&JsValue::from_str("__VFS_ROOT__"))?;
        
        // wait for delete to complete
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let success_closure = Closure::wrap(Box::new({
                let request = request.clone();
                move |_: Event| {
                    resolve.call0(&JsValue::NULL).unwrap();
                }
            }) as Box<dyn FnMut(_)>);
            
            let error_closure = Closure::wrap(Box::new({
                let request = request.clone();
                move |_: Event| {
                    let error = match request.error() {
                        Ok(Some(dom_err)) => JsValue::from(dom_err),
                        Ok(None) => JsValue::from_str("unknown delete error"),
                        Err(js_err) => js_err,
                    };
                    reject.call1(&JsValue::NULL, &error).unwrap();
                }
            }) as Box<dyn FnMut(_)>);
            
            request.set_onsuccess(Some(success_closure.as_ref().unchecked_ref()));
            request.set_onerror(Some(error_closure.as_ref().unchecked_ref()));
            success_closure.forget();
            error_closure.forget();
        });
        
        JsFuture::from(promise).await?;
        Ok(())
    }

    /// get storage statistics
    pub async fn get_storage_stats(&self) -> Result<JsValue, JsValue> {
        let db = self.db.as_ref().ok_or("Database not initialized")?;
        let _transaction = db.transaction_with_str(STORE_NAME)?;
        
        // for now, return simplified stats without waiting for async operations
        let stats = serde_json::json!({
            "node_count": 1,
            "total_original_size": 0,
            "total_stored_size": 0,
            "compression_ratio": 1.0,
            "storage_type": "IndexedDB",
            "database_name": DB_NAME,
            "store_name": STORE_NAME,
            "database_version": DB_VERSION,
            "message": "full stats calculation requires async operations"
        });
        
        serde_wasm_bindgen::to_value(&stats)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// calculate original size of stored vfs recursively
    fn calculate_original_size(&self, node: &StoredNode) -> usize {
        match node {
            StoredNode::File(file) => file.original_size,
            StoredNode::Directory(dir) => dir.children.values().map(|child| self.calculate_original_size(child)).sum(),
            StoredNode::Symlink(_) => 0,
        }
    }
} 
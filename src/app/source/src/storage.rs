use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use serde::{Serialize, Deserialize};
use crate::vfs::{VirtualFileSystem, VfsNode};
use flate2::{Compression, read::DeflateDecoder, write::DeflateEncoder};
use std::io::{Read, Write};
use base64::{Engine as _, engine::general_purpose};
use rexie::*;

const DB_NAME: &str = "filesystem";
const DB_VERSION: u32 = 1;
const STORE_NAME: &str = "vfs_store";
const VFS_KEY: &str = "vfs";
const COMPRESSION_THRESHOLD: usize = 1024; // only compress files > 1kb

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
    db: Option<Rexie>,
}

impl PersistentStorage {
    pub fn new() -> Self {
        Self { 
            db: None 
        }
    }

    /// init rexie database following the user's example pattern
    pub async fn init(&mut self) -> std::result::Result<(), JsValue> {
        web_sys::console::log_1(&"initializing storage database...".into());
        
        // build database with proper async handling
        let rexie = Rexie::builder(DB_NAME)
            .version(DB_VERSION)
            .add_object_store(ObjectStore::new(STORE_NAME))
            .build()
            .await
            .map_err(|e| JsValue::from_str(&format!("database creation failed: {:?}", e)))?;
        
        self.db = Some(rexie);
        web_sys::console::log_1(&"storage database initialized successfully".into());
        Ok(())
    }

    /// compress data if above threshold using deflate
    fn compress_data(data: &[u8]) -> std::result::Result<(Vec<u8>, bool), Box<dyn std::error::Error>> {
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
    fn decompress_data(data: &[u8], compressed: bool) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
        if !compressed {
            return Ok(data.to_vec());
        }

        let mut decoder = DeflateDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }

    /// convert vfs node to stored format recursively
    pub fn node_to_stored(&self, node: &VfsNode, path: &str) -> StoredNode {
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
    fn stored_to_node(&self, stored: &StoredNode) -> std::result::Result<VfsNode, Box<dyn std::error::Error>> {
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

    /// save entire vfs using proper transaction handling
    pub async fn save_vfs(&self, vfs: &VirtualFileSystem) -> std::result::Result<(), JsValue> {
        web_sys::console::log_1(&"starting vfs save to indexeddb...".into());
        
        let db = self.db.as_ref().ok_or("database not initialized")?;
        
        // serialize the entire vfs
        let stored_vfs = StoredVFS {
            root: self.node_to_stored(&vfs.root, "/"),
            version: 1,
        };
        
        let json = serde_json::to_string(&stored_vfs)
            .map_err(|e| JsValue::from_str(&format!("serialization failed: {}", e)))?;
        
        web_sys::console::log_1(&format!("serialized vfs: {} bytes", json.len()).into());
        
        // create readwrite transaction
        let tx = db.transaction(&[STORE_NAME], TransactionMode::ReadWrite)
            .map_err(|e| JsValue::from_str(&format!("transaction creation failed: {:?}", e)))?;
        
        let store = tx.store(STORE_NAME)
            .map_err(|e| JsValue::from_str(&format!("store access failed: {:?}", e)))?;
        
        // put the data with our key
        store.put(&JsValue::from_str(&json), Some(&JsValue::from_str(VFS_KEY)))
            .await
            .map_err(|e| JsValue::from_str(&format!("data storage failed: {:?}", e)))?;
        
        web_sys::console::log_1(&"vfs successfully saved to indexeddb".into());
        Ok(())
    }

    /// load entire vfs with proper transaction handling  
    pub async fn load_vfs(&self) -> std::result::Result<VirtualFileSystem, JsValue> {
        web_sys::console::log_1(&"starting vfs load from indexeddb...".into());
        
        let db = self.db.as_ref().ok_or("database not initialized")?;
        
        // create readonly transaction
        let tx = db.transaction(&[STORE_NAME], TransactionMode::ReadOnly)
            .map_err(|e| JsValue::from_str(&format!("read transaction failed: {:?}", e)))?;
        
        let store = tx.store(STORE_NAME)
            .map_err(|e| JsValue::from_str(&format!("store access failed: {:?}", e)))?;
        
        // get the data
        let result = store.get(&JsValue::from_str(VFS_KEY))
            .await
            .map_err(|e| JsValue::from_str(&format!("data retrieval failed: {:?}", e)))?;
        
        if result.is_undefined() || result.is_null() {
            web_sys::console::log_1(&"no saved vfs found, creating fresh filesystem".into());
            return Ok(VirtualFileSystem::new());
        }
        
        let json = result.as_string()
            .ok_or_else(|| JsValue::from_str("stored data is not a string"))?;
        
        web_sys::console::log_1(&format!("deserializing vfs: {} bytes", json.len()).into());
        
        let stored_vfs: StoredVFS = serde_json::from_str(&json)
            .map_err(|e| JsValue::from_str(&format!("deserialization failed: {}", e)))?;

        let root = self.stored_to_node(&stored_vfs.root)
            .map_err(|e| JsValue::from_str(&format!("node reconstruction failed: {}", e)))?;

        let mut vfs = VirtualFileSystem::new();
        vfs.root = root;
        
        web_sys::console::log_1(&"vfs successfully loaded from indexeddb".into());
        Ok(vfs)
    }

    /// save a single node (keeping for compatibility)
    pub async fn save_node(&self, _path: &str, _node: &VfsNode) -> std::result::Result<(), JsValue> {
        Err(JsValue::from_str("use save_vfs instead - saves entire filesystem atomically"))
    }

    /// load a single node (keeping for compatibility)
    pub async fn load_node(&self, path: &str) -> std::result::Result<Option<VfsNode>, JsValue> {
        if path == "/" {
            match self.load_vfs().await {
                Ok(vfs) => Ok(Some(vfs.root)),
                Err(_) => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    /// delete all stored data
    pub async fn delete_node(&self, _path: &str) -> std::result::Result<(), JsValue> {
        let db = self.db.as_ref().ok_or("database not initialized")?;
        
        let tx = db.transaction(&[STORE_NAME], TransactionMode::ReadWrite)
            .map_err(|e| JsValue::from_str(&format!("delete transaction failed: {:?}", e)))?;
        
        let store = tx.store(STORE_NAME)
            .map_err(|e| JsValue::from_str(&format!("store access failed: {:?}", e)))?;
        
        store.delete(&JsValue::from_str(VFS_KEY))
            .await
            .map_err(|e| JsValue::from_str(&format!("deletion failed: {:?}", e)))?;
        
        web_sys::console::log_1(&"storage cleared successfully".into());
        Ok(())
    }

    /// get storage statistics
    pub async fn get_storage_stats(&self) -> std::result::Result<JsValue, JsValue> {
        let db = self.db.as_ref().ok_or("database not initialized")?;
        
        let tx = db.transaction(&[STORE_NAME], TransactionMode::ReadOnly)
            .map_err(|e| JsValue::from_str(&format!("stats transaction failed: {:?}", e)))?;
        
        let store = tx.store(STORE_NAME)
            .map_err(|e| JsValue::from_str(&format!("store access failed: {:?}", e)))?;
        
        let result = store.get(&JsValue::from_str(VFS_KEY))
            .await
            .map_err(|e| JsValue::from_str(&format!("stats retrieval failed: {:?}", e)))?;
        
        let stored_size = if result.is_undefined() || result.is_null() {
            0
        } else {
            result.as_string().map(|s| s.len()).unwrap_or(0)
        };
        
        let stats = serde_json::json!({
            "storage_type": "IndexedDB",
            "database_name": DB_NAME,
            "store_name": STORE_NAME,
            "stored_size_bytes": stored_size,
            "stored_size_kb": stored_size as f64 / 1024.0,
            "compression_threshold": COMPRESSION_THRESHOLD,
            "has_data": stored_size > 0,
            "vfs_key": VFS_KEY
        });
        
        serde_wasm_bindgen::to_value(&stats)
            .map_err(|e| JsValue::from_str(&format!("serialization failed: {}", e)))
    }
} 
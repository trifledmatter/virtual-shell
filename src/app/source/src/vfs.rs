use std::collections::HashMap;
use chrono::{DateTime, Local};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Permissions {
    pub user: u8,  // bits: rwx
    pub group: u8, // bits: rwx
    pub other: u8, // bits: rwx
}

impl Permissions {
    pub fn new(user: u8, group: u8, other: u8) -> Self {
        Self { user, group, other }
    }
    pub fn default_file() -> Self {
        Self::new(0b110, 0b100, 0b100) // rw-r--r--
    }
    pub fn default_dir() -> Self {
        Self::new(0b111, 0b101, 0b101) // rwxr-xr-x
    }
}

#[derive(Debug, Clone)]
pub enum VfsNode {
    File {
        name: String,
        content: Vec<u8>,
        permissions: Permissions,
        mtime: DateTime<Local>,
    },
    Directory {
        name: String,
        children: HashMap<String, VfsNode>,
        permissions: Permissions,
        mtime: DateTime<Local>,
    },
    Symlink {
        name: String,
        target: String,
        permissions: Permissions,
        mtime: DateTime<Local>,
    },
}

#[derive(Debug, Clone)]
pub struct VirtualFileSystem {
    pub root: VfsNode,
}

impl VirtualFileSystem {
    pub fn new() -> Self {
        Self {
            root: VfsNode::Directory {
                name: "/".to_string(),
                children: HashMap::new(),
                permissions: Permissions::default_dir(),
                mtime: Local::now(),
            },
        }
    }

    // get mutable node ref - pretty straightforward
    pub fn resolve_path_mut<'a>(&'a mut self, path: &str) -> Option<&'a mut VfsNode> {
        let mut components = path.trim_matches('/').split('/').filter(|c| !c.is_empty());
        let mut node = &mut self.root;
        for comp in components {
            match node {
                VfsNode::Directory { children, .. } => {
                    node = children.get_mut(comp)?;
                }
                _ => return None,
            }
        }
        Some(node)
    }

    // immutable version - same deal
    pub fn resolve_path<'a>(&'a self, path: &str) -> Option<&'a VfsNode> {
        let mut components = path.trim_matches('/').split('/').filter(|c| !c.is_empty());
        let mut node = &self.root;
        for comp in components {
            match node {
                VfsNode::Directory { children, .. } => {
                    node = children.get(comp)?;
                }
                _ => return None,
            }
        }
        Some(node)
    }

    /// follows symlinks unless physical=true
    pub fn resolve_path_with_symlinks<'a>(&'a self, path: &str, physical: bool) -> Option<&'a VfsNode> {
        let mut components: Vec<&str> = path.trim_matches('/').split('/').filter(|c| !c.is_empty()).collect();
        let mut node = &self.root;
        let mut seen = 0;
        while let Some(comp) = components.first() {
            match node {
                VfsNode::Directory { children, .. } => {
                    if let Some(next) = children.get(*comp) {
                        match next {
                            VfsNode::Symlink { target, .. } if !physical => {
                                // swap in symlink target for current component
                                let mut target_comps: Vec<&str> = target.trim_matches('/').split('/').filter(|c| !c.is_empty()).collect();
                                components = [target_comps, components[1..].to_vec()].concat();
                                seen += 1;
                                if seen > 16 { return None; } // bail if too many redirects
                                continue;
                            }
                            _ => {
                                node = next;
                                components.remove(0);
                            }
                        }
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        Some(node)
    }

    // make a new file - content passed as bytes
    pub fn create_file(&mut self, path: &str, content: Vec<u8>) -> Result<(), String> {
        let (parent_path, file_name) = Self::split_path(path)?;
        let parent = self.resolve_path_mut(parent_path)
            .and_then(|node| match node {
                VfsNode::Directory { children, .. } => Some(children),
                _ => None,
            })
            .ok_or("Parent directory not found")?;
        if parent.contains_key(file_name) {
            return Err("File already exists".to_string());
        }
        parent.insert(
            file_name.to_string(),
            VfsNode::File {
                name: file_name.to_string(),
                content: content.clone(),
                permissions: Permissions::default_file(),
                mtime: Local::now(),
            },
        );
        Ok(())
    }

    // get file contents as byte slice
    pub fn read_file(&self, path: &str) -> Result<&[u8], String> {
        match self.resolve_path(path) {
            Some(VfsNode::File { content, .. }) => Ok(content),
            _ => Err("File not found".to_string()),
        }
    }

    // nuke existing file contents and replace
    pub fn write_file(&mut self, path: &str, content: Vec<u8>) -> Result<(), String> {
        match self.resolve_path_mut(path) {
            Some(VfsNode::File { content: file_content, mtime, .. }) => {
                *file_content = content.clone();
                *mtime = Local::now();
                Ok(())
            }
            _ => Err("File not found".to_string()),
        }
    }

    // rm -rf basically
    pub fn delete(&mut self, path: &str) -> Result<(), String> {
        let (parent_path, name) = Self::split_path(path)?;
        let parent = self.resolve_path_mut(parent_path)
            .and_then(|node| match node {
                VfsNode::Directory { children, .. } => Some(children),
                _ => None,
            })
            .ok_or("Parent directory not found")?;
        let result = parent.remove(name).map(|_| ()).ok_or("Node not found".to_string());
        result
    }

    // mkdir - errors if exists already
    pub fn create_dir(&mut self, path: &str) -> Result<(), String> {
        let (parent_path, dir_name) = Self::split_path(path)?;
        let parent = self.resolve_path_mut(parent_path)
            .and_then(|node| match node {
                VfsNode::Directory { children, .. } => Some(children),
                _ => None,
            })
            .ok_or("Parent directory not found")?;
        if parent.contains_key(dir_name) {
            return Err("Directory already exists".to_string());
        }
        parent.insert(
            dir_name.to_string(),
            VfsNode::Directory {
                name: dir_name.to_string(),
                children: HashMap::new(),
                permissions: Permissions::default_dir(),
                mtime: Local::now(),
            },
        );
        Ok(())
    }

    /// ln -s target path
    pub fn create_symlink(&mut self, path: &str, target: &str) -> Result<(), String> {
        let (parent_path, link_name) = Self::split_path(path)?;
        let parent = self.resolve_path_mut(parent_path)
            .and_then(|node| match node {
                VfsNode::Directory { children, .. } => Some(children),
                _ => None,
            })
            .ok_or("Parent directory not found")?;
        if parent.contains_key(link_name) {
            return Err("File exists".to_string());
        }
        parent.insert(link_name.to_string(), VfsNode::Symlink {
            name: link_name.to_string(),
            target: target.to_string(),
            permissions: Permissions::default_file(),
            mtime: Local::now(),
        });
        Ok(())
    }

    // ls - returns just names
    pub fn list_dir(&self, path: &str) -> Result<Vec<String>, String> {
        match self.resolve_path(path) {
            Some(VfsNode::Directory { children, .. }) => Ok(children.keys().cloned().collect()),
            _ => Err("Directory not found".to_string()),
        }
    }

    // util to get parent dir and filename from path
    pub fn split_path(path: &str) -> Result<(&str, &str), String> {
        let path = path.trim_matches('/');
        match path.rfind('/') {
            Some(idx) => Ok((&path[..idx], &path[idx+1..])),
            None => Ok(("/", path)),
        }
    }
}

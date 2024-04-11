extern crate alloc;
use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, vec::Vec};

use crate::locks::SpinLock as Mutex;

use super::{VNodeOps, VNodeOpsError, Vfs, VfsOps};

type File = Vec<u8>;
type Dir = BTreeMap<String, Arc<Mutex<Entity>>>;

enum EntityType {
    File(File),
    Dir(Dir),
}

struct Entity {
    r#type: EntityType,
    parent: Option<Arc<Mutex<Entity>>>,
    vfs_mounted: Option<Arc<Mutex<Vfs>>>,
}

impl VNodeOps for Arc<Mutex<Entity>> {
    fn open(&mut self, flags: u32) -> Result<super::Fd, super::VNodeOpsError> {
        todo!()
    }

    fn close(&mut self, flags: u32) -> Result<(), super::VNodeOpsError> {
        todo!()
    }

    fn read(&self, pos: usize, buf: &mut [u8]) -> Result<usize, super::VNodeOpsError> {
        let cur = self.lock();

        if cur.vfs_mounted.is_some() {
            unreachable!();
        }

        let EntityType::File(file) = &cur.r#type else {
            return Err(crate::vfs::VNodeOpsError::NotARegularFile);
        };

        let src = &file[pos..];
        let dst = buf;
        dst.copy_from_slice(src);

        Ok(core::cmp::min(src.len(), dst.len()))
    }

    fn write(&mut self, pos: usize, buf: &[u8]) -> Result<usize, super::VNodeOpsError> {
        let mut cur = self.lock();

        if cur.vfs_mounted.is_some() {
            unreachable!();
        }

        let EntityType::File(file) = &mut cur.r#type else {
            return Err(crate::vfs::VNodeOpsError::NotARegularFile);
        };

        let dst = &mut file[pos..];
        let src = buf;
        dst.copy_from_slice(src);

        Ok(core::cmp::min(src.len(), dst.len()))
    }

    fn create(&mut self, name: &String) -> Result<(), super::VNodeOpsError> {
        let mut cur = self.lock();

        if cur.vfs_mounted.is_some() {
            unreachable!();
        }

        let EntityType::Dir(dir) = &mut cur.r#type else {
            return Err(VNodeOpsError::NotADirectory);
        };

        let parent = Arc::clone(self);
        let new_file = Entity {
            r#type: EntityType::File(Vec::new()),
            parent: Some(parent),
            vfs_mounted: None,
        };
        dir.insert(name.clone(), Arc::new(Mutex::new(new_file)));

        Ok(())
    }

    fn remove(&mut self, name: &String) -> Result<(), super::VNodeOpsError> {
        let mut cur = self.lock();

        if cur.vfs_mounted.is_some() {
            unreachable!();
        }

        let EntityType::Dir(dir) = &mut cur.r#type else {
            return Err(VNodeOpsError::NotADirectory);
        };

        if dir.remove(name).is_none() {
            return Err(VNodeOpsError::NotFound);
        }

        Ok(())
    }

    fn rename(&mut self, old_name: &String, new_name: &String) -> Result<(), super::VNodeOpsError> {
        let mut cur = self.lock();

        if cur.vfs_mounted.is_some() {
            unreachable!();
        }

        let EntityType::Dir(dir) = &mut cur.r#type else {
            return Err(VNodeOpsError::NotADirectory);
        };

        if let Some(entity) = dir.remove(old_name) {
            dir.insert(new_name.clone(), entity);
            Ok(())
        } else {
            Err(VNodeOpsError::NotFound)
        }
    }
    
    fn mkdir(&mut self) -> Result<(), VNodeOpsError> {
        todo!()
    }
    
    fn rmdir(&mut self) -> Result<(), VNodeOpsError> {
        todo!()
    }
}

struct MemFS {
    root: Arc<Mutex<Entity>>,
}

impl VfsOps for MemFS {
    fn mount(&mut self) -> Result<(), super::VfsOpsError> {
        todo!()
    }

    fn unmount(&mut self) -> Result<(), super::VfsOpsError> {
        todo!()
    }
}
mod memfs;

extern crate alloc;
use crate::locks::SpinLock as Mutex;
use alloc::{boxed::Box, string::String, sync::Arc};
use bitflags::bitflags;

static ROOTVFS: Arc<Mutex<Vfs>> = Arc::new(Mutex::new(todo!()));

// Interface between File system independent part and the Filesystem dependent part
// Filesystem independent part calls the methods defined in the interface to perform some high level file manipulations
// Filesystem dependent part implements the methods defined in the interface
// other modules and syscall handling will use the filesystem independent code to get the job done
// every concrete filesystem implementation will satisfy the interface

// Represents a filesystem
struct Vfs {
    // link to the next Vfs in the chain
    next: Arc<Mutex<Vfs>>,
    // points to the Vnode where this VFS is mounted
    // NULL for the rool VFS
    vnode_covered: Arc<Mutex<Vnode>>,
    flags: u32,
    bsize: u32,
    // pointer to concrete implementation of the filesystem represented by the Vfs
    inner: Box<dyn VfsOps>,
}

enum VType {
    Non,
    Reg,
    Dir,
    Blk,
    Chr,
    Lnk,
    Sock,
    Bad,
}

// Represents an active file
struct Vnode {
    flags: VnodeFlags,
    // number of
    ref_count: u16,
    // points to the VFS that is mounted here
    vfs_mounted_here: Arc<Mutex<Vfs>>,
    // points to the VFS to which the Vnode belongs
    vfsp: Arc<Mutex<Vfs>>,
    vtype: VType,
    // if true, the underlying file is deleted once the ref goes to 0
    marked_for_del: bool,
    inner: Box<dyn VNodeOps>,
}

bitflags! {
    pub struct VnodeFlags: u8 {
        // is the root of a filesystem
        const ROOT = 1 << 0;
    }
}
impl Vnode {
    fn incr_ref(&mut self) {
        self.ref_count += 1;
    }

    fn decr_ref(&mut self) {
        self.ref_count -= 1;
    }
}

enum VfsOpsError {}

trait VfsOps: Send + Sync {
    fn mount(&mut self) -> Result<(), VfsOpsError>;
    fn unmount(&mut self) -> Result<(), VfsOpsError>;
    // fn root(&mut self) -> Result<Arc<Mutex<Vnode>>, VfsOpsError>;
}

bitflags! {
    pub struct OpenModeFlags: u8 {
        const READ    = 1 << 0;
        const WRITE   = 1 << 1;
        const APPEND  = 1 << 2;
        const RD_WR   = 1 << 3;
        const RD_AP   = 1 << 4;
    }
}

// Represents open file
struct File {
    // points to vnode that represents the file
    vnode: Arc<Mutex<Vnode>>,
    // cursor position in the file
    offset: usize,
    // flags passed to `open` call that returned this File
    // used in subsequent operations like read, write etc.
    flags: OpenModeFlags,
    // represents the number of Fd pointing to this
    // once it goes to zero, can be destroyed
    ref_count: usize,
}

type Fd = usize;

enum VNodeOpsError {
    NotARegularFile,
    NotADirectory,
    NotFound,
}

trait VNodeOps: Send + Sync {
    fn open(&mut self, flags: u32) -> Result<Fd, VNodeOpsError>;
    fn close(&mut self, flags: u32) -> Result<(), VNodeOpsError>;
    fn read(&self, pos: usize, buf: &mut [u8]) -> Result<usize, VNodeOpsError>;
    fn write(&mut self, pos: usize, buf: &[u8]) -> Result<usize, VNodeOpsError>;
    fn create(&mut self, name: &String) -> Result<(), VNodeOpsError>;
    fn remove(&mut self, name: &String) -> Result<(), VNodeOpsError>;
    fn mkdir(&mut self) -> Result<(), VNodeOpsError>;
    fn rmdir(&mut self) -> Result<(), VNodeOpsError>;
    fn rename(&mut self, old_name: &String, new_name: &String) -> Result<(), VNodeOpsError>;
}

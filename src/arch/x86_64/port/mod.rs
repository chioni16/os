mod asm;
mod inout;

use inout::InOut;

pub(super) struct Port(u16);

impl Port {
    pub(super) fn new(port: u16) -> Self {
        Self(port)
    }

    // borrows mutably as we want there to be only one handle
    // reading from the given port at any given time
    // not sure if this is even required
    // but let it be there for the time being
    pub(super) unsafe fn read<T: InOut>(&mut self) -> T {
        T::port_in(self.0)
    }

    // borrows mutably as we want there to be only one handle
    // writing to the given port at any given time
    pub(super) unsafe fn write<T: InOut>(&mut self, data: T) {
        T::port_out(self.0, data)
    }
}

// a small delay
// useful during PIC remapping
pub(super) unsafe fn io_wait() {
    u8::port_out(0x80, 0);
}

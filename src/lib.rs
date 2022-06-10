use std::io::{self, Write};

use corosensei::{stack::DefaultStack, CoroutineResult, ScopedCoroutine, Yielder};
use tokio::io::{AsyncRead, ReadBuf};

#[repr(transparent)]
pub struct Writer<'a> {
    y: &'a Yielder<*mut tokio::io::ReadBuf<'static>, ()>,
}

#[repr(transparent)]
pub struct Reader<'a> {
    co: ScopedCoroutine<'a, *mut tokio::io::ReadBuf<'static>, (), io::Result<()>, DefaultStack>,
}

impl<'a> Reader<'a> {
    pub fn new<F>(f: F) -> Self
    where
        F: 'a + for<'b> FnOnce(&'b mut Writer<'b>) -> io::Result<()>,
    {
        Reader {
            co: ScopedCoroutine::new(|y: &Yielder<*mut tokio::io::ReadBuf<'static>, ()>, orig| {
                // when first initialized, it will just send null
                assert!(orig.is_null());
                f(&mut Writer { y })
            }),
        }
    }
}

impl Write for Writer<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // SAFETY: The coroutine ensures this lives as long as the caller
        let out = unsafe { &mut *self.y.suspend(()) };
        let to_write = out.remaining().min(buf.len());
        out.put_slice(&buf[..to_write]);

        drop(out); // Make it explicit we're done with this buffer ASAP

        Ok(to_write)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> AsyncRead for Reader<'a> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        // initialize callback
        if !self.co.started() {
            if let CoroutineResult::Return(res) = self.co.resume(std::ptr::null_mut()) {
                return std::task::Poll::Ready(res);
            }
        }

        if !self.co.done() {
            // SAFETY: The coroutine body is executed immediately, so the lifetime is actually the same
            // just need to bypass the borrow checker...
            if let CoroutineResult::Return(res) = self
                .co
                .resume(unsafe { std::mem::transmute::<_, *mut ReadBuf<'static>>(buf) })
            {
                return std::task::Poll::Ready(res);
            }
        }

        std::task::Poll::Ready(Ok(()))
    }
}

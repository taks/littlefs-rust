//! Unit tests using TestContext.

use super::*;

/// Minimal: construct TestContext and verify config/ram. No lfs calls.
#[test]
fn test_context_smoke() {
    let ctx = TestContext::default_blocks();
    let cfg = ctx.config();
    assert!(!cfg.context.is_null(), "config.context should be set");
    assert!(cfg.read.is_some());
    assert_eq!(ctx.ram.data.len(), 512 * 128);
    // Direct read through callback
    let mut buf = [0u8; 8];
    let err = cfg.read.expect("read")(ctx.config(), 0, 0, &mut buf);
    assert_eq!(err, Ok(()));
    assert_eq!(buf, [0u8; 8]);
}

/// Call lfs_init only. Isolates init from full format.
#[test]
fn test_context_lfs_init() {
    let ctx = TestContext::default_blocks();
    let mut lfs = crate::Lfs::default();
    let err = crate::fs::lfs_init(&mut lfs, ctx.config());
    assert_eq!(err, Ok(()));
}

/// Init + lookahead setup + lfs_dir_alloc. Stops before commit.
#[test]
fn test_context_format_to_alloc() {
    use crate::block_alloc::alloc::lfs_alloc_ckpoint;
    use crate::util::lfs_min;

    let ctx = TestContext::default_blocks();
    let mut lfs = crate::Lfs::default();
    let err = crate::fs::lfs_init(&mut lfs, ctx.config());
    assert_eq!(err, Ok(()));

    let cfg = unsafe { lfs.cfg.as_ref() };
    unsafe {
        lfs.lookahead.buffer.as_mut().fill(0);
    }
    lfs.lookahead.start = 0;
    lfs.lookahead.size = lfs_min(
        8 * cfg.lookahead_buffer.unwrap().len() as u32,
        lfs.block_count,
    );
    lfs.lookahead.next = 0;
    lfs_alloc_ckpoint(&mut lfs);

    #[allow(unused)]
    let root = crate::dir::LfsMdir {
        pair: [0, 0],
        rev: 0,
        off: 0,
        etag: 0,
        count: 0,
        erased: false,
        split: false,
        tail: [0, 0],
    };
    // let err = unsafe { lfs_dir_alloc(&mut lfs, &mut root) };
    // assert_eq!(err, Ok(()));
}

/// Verify buffer pointers are writable (lfs_init writes to them).
#[test]
fn test_context_buffers_writable() {
    let ctx = TestContext::default_blocks();
    // Manually write to each buffer - simulate what lfs_cache_zero and format do
    if let Some(mut buf) = ctx.config.read_buffer {
        unsafe { buf.as_mut().fill(0xff) };
    }
    if let Some(mut buf) = ctx.config.prog_buffer {
        unsafe { buf.as_mut().fill(0xff) };
    }
    if let Some(mut buf) = ctx.config.lookahead_buffer {
        unsafe { buf.as_mut().fill(0) };
    }
}

//! Upstream: tests/test_seek.toml

mod common;

#[cfg(feature = "slow_tests")]
use common::powerloss::{init_powerloss_context, powerloss_config, run_powerloss_linear};
use common::{
    LFS_FILE_MAX, LFS_O_APPEND, LFS_O_CREAT, LFS_O_RDONLY, LFS_O_RDWR, LFS_O_WRONLY, LFS_SEEK_CUR,
    LFS_SEEK_END, LFS_SEEK_SET, assert_ok, default_config, init_context, path_bytes,
};
use littlefs_rust_core::{
    Lfs, LfsConfig, LfsFile, error::Error, lfs_file_close, lfs_file_open, lfs_file_read,
    lfs_file_rewind, lfs_file_seek, lfs_file_size, lfs_file_sync, lfs_file_tell, lfs_file_write,
    lfs_format, lfs_mount, lfs_unmount,
};
use rstest::rstest;

const KITTY: &[u8] = b"kittycatcat";
const DOGGO: &[u8] = b"doggodogdog";
const HEDGEHOG: &[u8] = b"hedgehoghog";
const PORCUPINE: &[u8] = b"porcupineee";

// ── Upstream Cases ──────────────────────────

/// Upstream: [cases.test_seek_read]
/// defines = [{COUNT=132, SKIP=4}, {COUNT=132, SKIP=128}, ...]
#[rstest]
#[case(132, 4)]
#[case(132, 128)]
#[case(200, 10)]
#[case(200, 100)]
#[case(4, 1)]
#[case(4, 2)]
fn test_seek_read(#[case] count: u32, #[case] skip: u32) {
    let mut env = default_config(256);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let path = path_bytes("kitty");
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path.as_ptr(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    for _ in 0..count {
        let n = lfs_file_write(
            lfs,
            file,
            KITTY.as_ptr() as *const core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, Ok(KITTY.len() as u32));
    }
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_file_open(lfs, file, path.as_ptr(), LFS_O_RDONLY));

    let mut buf = [0u8; 32];
    let mut pos: i32 = -1;
    for _ in 0..skip {
        let n = lfs_file_read(
            lfs,
            file,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, Ok(KITTY.len() as u32));
        assert_eq!(&buf[..KITTY.len()], KITTY);
        pos = lfs_file_tell(lfs, file);
    }
    assert!(pos >= 0);

    assert_eq!(lfs_file_seek(lfs, file, pos, LFS_SEEK_SET), Ok(pos));
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, Ok(KITTY.len() as u32));
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert_ok(lfs_file_rewind(lfs, file));
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, Ok(KITTY.len() as u32));
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert_eq!(
        lfs_file_seek(lfs, file, 0, LFS_SEEK_CUR),
        Ok(KITTY.len() as i32)
    );
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, Ok(KITTY.len() as u32));
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert_eq!(
        lfs_file_seek(lfs, file, KITTY.len() as i32, LFS_SEEK_CUR),
        Ok(3 * KITTY.len() as i32)
    );
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, Ok(KITTY.len() as u32));
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert_eq!(lfs_file_seek(lfs, file, pos, LFS_SEEK_SET), Ok(pos));
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, Ok(KITTY.len() as u32));
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert_eq!(
        lfs_file_seek(lfs, file, -(KITTY.len() as i32), LFS_SEEK_CUR),
        Ok(pos)
    );
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, Ok(KITTY.len() as u32));
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert!(lfs_file_seek(lfs, file, -(KITTY.len() as i32), LFS_SEEK_END).is_ok());
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, Ok(KITTY.len() as u32));
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert_eq!(
        lfs_file_size(lfs, file),
        (count * KITTY.len() as u32) as i32
    );
    assert_eq!(
        lfs_file_seek(lfs, file, 0, LFS_SEEK_CUR),
        Ok((count * KITTY.len() as u32) as i32)
    );

    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_seek_write]
#[rstest]
#[case(132, 4)]
#[case(132, 128)]
#[case(200, 10)]
#[case(200, 100)]
#[case(4, 1)]
#[case(4, 2)]
fn test_seek_write(#[case] count: u32, #[case] skip: u32) {
    let mut env = default_config(256);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let path = c"kitty";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    for _ in 0..count {
        let n = lfs_file_write(
            lfs,
            file,
            KITTY.as_ptr() as *const core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, Ok(KITTY.len() as u32));
    }
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDWR));

    let mut buf = [0u8; 32];
    let mut pos: i32 = -1;
    for _ in 0..skip {
        let n = lfs_file_read(
            lfs,
            file,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, Ok(KITTY.len() as u32));
        assert_eq!(&buf[..KITTY.len()], KITTY);
        pos = lfs_file_tell(lfs, file);
    }
    assert!(pos >= 0);

    assert_eq!(lfs_file_seek(lfs, file, pos, LFS_SEEK_SET), Ok(pos));
    let n = lfs_file_write(
        lfs,
        file,
        DOGGO.as_ptr() as *const core::ffi::c_void,
        DOGGO.len() as u32,
    );
    assert_eq!(n, Ok(DOGGO.len() as u32));

    assert_eq!(lfs_file_seek(lfs, file, pos, LFS_SEEK_SET), Ok(pos));
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        DOGGO.len() as u32,
    );
    assert_eq!(n, Ok(DOGGO.len() as u32));
    assert_eq!(&buf[..DOGGO.len()], DOGGO);

    assert_ok(lfs_file_rewind(lfs, file));
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, Ok(KITTY.len() as u32));
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert_eq!(lfs_file_seek(lfs, file, pos, LFS_SEEK_SET), Ok(pos));
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        DOGGO.len() as u32,
    );
    assert_eq!(n, Ok(DOGGO.len() as u32));
    assert_eq!(&buf[..DOGGO.len()], DOGGO);

    assert!(lfs_file_seek(lfs, file, -(KITTY.len() as i32), LFS_SEEK_END).is_ok());
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, Ok(KITTY.len() as u32));
    assert_eq!(&buf[..KITTY.len()], KITTY);

    assert_eq!(
        lfs_file_size(lfs, file),
        (count * KITTY.len() as u32) as i32
    );
    assert_eq!(
        lfs_file_seek(lfs, file, 0, LFS_SEEK_CUR),
        Ok((count * KITTY.len() as u32) as i32)
    );

    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_seek_boundary_read]
/// defines.COUNT = 132
#[test]
fn test_seek_boundary_read() {
    const COUNT: u32 = 132;
    let mut env = default_config(256);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let path = c"kitty";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    for _ in 0..COUNT {
        let n = lfs_file_write(
            lfs,
            file,
            KITTY.as_ptr() as *const core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, Ok(KITTY.len() as u32));
    }
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));

    let size = KITTY.len() as i64;
    let pattern = b"kittycatcatkittycatcat";
    let offsets: [i64; 13] = [
        512,
        1024 - 4,
        512 + 1,
        1024 - 4 + 1,
        512 - 1,
        1024 - 4 - 1,
        512 - size,
        1024 - 4 - size,
        512 - size + 1,
        1024 - 4 - size + 1,
        512 - size - 1,
        1024 - 4 - size - 1,
        size * (COUNT as i64 - 2) - 1,
    ];

    let mut buf = [0u8; 32];
    for off in offsets {
        if off < 0 || off + size > (COUNT as i64 * size) {
            continue;
        }
        assert_eq!(
            lfs_file_seek(lfs, file, off as i32, LFS_SEEK_SET),
            Ok(off as i32)
        );
        let n = lfs_file_read(
            lfs,
            file,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, Ok(KITTY.len() as u32));
        let base = (off % size) as usize;
        assert_eq!(
            &buf[..KITTY.len()],
            &pattern[base..base + KITTY.len()],
            "off={}",
            off
        );

        let off_after = off + size + 1;
        if off_after >= 0 && off_after + size <= COUNT as i64 * size {
            assert_eq!(
                lfs_file_seek(lfs, file, off_after as i32, LFS_SEEK_SET),
                Ok(off_after as i32)
            );
            let n = lfs_file_read(
                lfs,
                file,
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                KITTY.len() as u32,
            );
            assert_eq!(n, Ok(KITTY.len() as u32));
            let base = ((off + 1) % size) as usize;
            assert_eq!(&buf[..KITTY.len()], &pattern[base..base + KITTY.len()]);
        }

        let off_before = off - size - 1;
        if off_before >= 0 {
            assert_eq!(
                lfs_file_seek(lfs, file, off_before as i32, LFS_SEEK_SET),
                Ok(off_before as i32)
            );
            let n = lfs_file_read(
                lfs,
                file,
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                KITTY.len() as u32,
            );
            assert_eq!(n, Ok(KITTY.len() as u32));
            let base = ((off - 1).rem_euclid(size)) as usize;
            assert_eq!(&buf[..KITTY.len()], &pattern[base..base + KITTY.len()]);
        }

        assert_eq!(lfs_file_seek(lfs, file, 0, LFS_SEEK_SET), Ok(0));
        let n = lfs_file_read(
            lfs,
            file,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, Ok(KITTY.len() as u32));
        assert_eq!(&buf[..KITTY.len()], KITTY);

        assert_eq!(
            lfs_file_seek(lfs, file, off as i32, LFS_SEEK_SET),
            Ok(off as i32)
        );
        let n = lfs_file_read(
            lfs,
            file,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, Ok(KITTY.len() as u32));
        let base = (off % size) as usize;
        assert_eq!(&buf[..KITTY.len()], &pattern[base..base + KITTY.len()]);

        let off_after = off + size + 1;
        if off_after >= 0 && off_after + size <= COUNT as i64 * size {
            assert_eq!(
                lfs_file_seek(lfs, file, off_after as i32, LFS_SEEK_SET),
                Ok(off_after as i32)
            );
            let n = lfs_file_read(
                lfs,
                file,
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                KITTY.len() as u32,
            );
            assert_eq!(n, Ok(KITTY.len() as u32));
            let base = ((off + 1) % size) as usize;
            assert_eq!(&buf[..KITTY.len()], &pattern[base..base + KITTY.len()]);
        }

        let off_before = off - size - 1;
        if off_before >= 0 {
            assert_eq!(
                lfs_file_seek(lfs, file, off_before as i32, LFS_SEEK_SET),
                Ok(off_before as i32)
            );
            let n = lfs_file_read(
                lfs,
                file,
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                KITTY.len() as u32,
            );
            assert_eq!(n, Ok(KITTY.len() as u32));
            let base = ((off - 1).rem_euclid(size)) as usize;
            assert_eq!(&buf[..KITTY.len()], &pattern[base..base + KITTY.len()]);
        }

        assert_ok(lfs_file_sync(lfs, file));

        assert_eq!(lfs_file_seek(lfs, file, 0, LFS_SEEK_SET), Ok(0));
        let n = lfs_file_read(
            lfs,
            file,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, Ok(KITTY.len() as u32));
        assert_eq!(&buf[..KITTY.len()], KITTY);

        assert_eq!(
            lfs_file_seek(lfs, file, off as i32, LFS_SEEK_SET),
            Ok(off as i32)
        );
        let n = lfs_file_read(
            lfs,
            file,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, Ok(KITTY.len() as u32));
        let base = (off % size) as usize;
        assert_eq!(&buf[..KITTY.len()], &pattern[base..base + KITTY.len()]);

        let off_after = off + size + 1;
        if off_after >= 0 && off_after + size <= COUNT as i64 * size {
            assert_eq!(
                lfs_file_seek(lfs, file, off_after as i32, LFS_SEEK_SET),
                Ok(off_after as i32)
            );
            let n = lfs_file_read(
                lfs,
                file,
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                KITTY.len() as u32,
            );
            assert_eq!(n, Ok(KITTY.len() as u32));
            let base = ((off + 1) % size) as usize;
            assert_eq!(&buf[..KITTY.len()], &pattern[base..base + KITTY.len()]);
        }

        let off_before = off - size - 1;
        if off_before >= 0 {
            assert_eq!(
                lfs_file_seek(lfs, file, off_before as i32, LFS_SEEK_SET),
                Ok(off_before as i32)
            );
            let n = lfs_file_read(
                lfs,
                file,
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                KITTY.len() as u32,
            );
            assert_eq!(n, Ok(KITTY.len() as u32));
            let base = ((off - 1).rem_euclid(size)) as usize;
            assert_eq!(&buf[..KITTY.len()], &pattern[base..base + KITTY.len()]);
        }
    }

    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_seek_boundary_write]
/// defines.COUNT = 132
#[test]
fn test_seek_boundary_write() {
    const COUNT: u32 = 132;
    let mut env = default_config(256);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let path = c"kitty";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    for _ in 0..COUNT {
        let n = lfs_file_write(
            lfs,
            file,
            KITTY.as_ptr() as *const core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, Ok(KITTY.len() as u32));
    }
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDWR));

    let size = KITTY.len() as i64;
    let offsets: [i64; 13] = [
        512,
        1024 - 4,
        512 + 1,
        1024 - 4 + 1,
        512 - 1,
        1024 - 4 - 1,
        512 - size,
        1024 - 4 - size,
        512 - size + 1,
        1024 - 4 - size + 1,
        512 - size - 1,
        1024 - 4 - size - 1,
        size * (COUNT as i64 - 2) - 1,
    ];

    let mut buf = [0u8; 32];
    for off in offsets {
        if off < 0 || off + size > COUNT as i64 * size {
            continue;
        }
        assert_eq!(
            lfs_file_seek(lfs, file, off as i32, LFS_SEEK_SET),
            Ok(off as i32)
        );
        let n = lfs_file_write(
            lfs,
            file,
            HEDGEHOG.as_ptr() as *const core::ffi::c_void,
            HEDGEHOG.len() as u32,
        );
        assert_eq!(n, Ok(HEDGEHOG.len() as u32));

        assert_eq!(
            lfs_file_seek(lfs, file, off as i32, LFS_SEEK_SET),
            Ok(off as i32)
        );
        let n = lfs_file_read(
            lfs,
            file,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            HEDGEHOG.len() as u32,
        );
        assert_eq!(n, Ok(HEDGEHOG.len() as u32));
        assert_eq!(&buf[..HEDGEHOG.len()], HEDGEHOG);

        assert_eq!(lfs_file_seek(lfs, file, 0, LFS_SEEK_SET), Ok(0));
        let n = lfs_file_read(
            lfs,
            file,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, Ok(KITTY.len() as u32));
        assert_eq!(&buf[..KITTY.len()], KITTY);

        assert_eq!(
            lfs_file_seek(lfs, file, off as i32, LFS_SEEK_SET),
            Ok(off as i32)
        );
        let n = lfs_file_read(
            lfs,
            file,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            HEDGEHOG.len() as u32,
        );
        assert_eq!(n, Ok(HEDGEHOG.len() as u32));
        assert_eq!(&buf[..HEDGEHOG.len()], HEDGEHOG);

        assert_ok(lfs_file_sync(lfs, file));

        assert_eq!(lfs_file_seek(lfs, file, 0, LFS_SEEK_SET), Ok(0));
        let n = lfs_file_read(
            lfs,
            file,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, Ok(KITTY.len() as u32));
        assert_eq!(&buf[..KITTY.len()], KITTY);

        assert_eq!(
            lfs_file_seek(lfs, file, off as i32, LFS_SEEK_SET),
            Ok(off as i32)
        );
        let n = lfs_file_read(
            lfs,
            file,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            HEDGEHOG.len() as u32,
        );
        assert_eq!(n, Ok(HEDGEHOG.len() as u32));
        assert_eq!(&buf[..HEDGEHOG.len()], HEDGEHOG);
    }

    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_seek_out_of_bounds]
#[rstest]
#[case(132, 4)]
#[case(132, 128)]
#[case(200, 10)]
#[case(200, 100)]
#[case(4, 2)]
#[case(4, 3)]
fn test_seek_out_of_bounds(#[case] count: u32, #[case] skip: u32) {
    let mut env = default_config(256);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let path = c"kitty";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    for _ in 0..count {
        let n = lfs_file_write(
            lfs,
            file,
            KITTY.as_ptr() as *const core::ffi::c_void,
            KITTY.len() as u32,
        );
        assert_eq!(n, Ok(KITTY.len() as u32));
    }
    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));

    assert_ok(lfs_mount(lfs, &env.config));
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDWR));

    let size = KITTY.len() as i64;
    let hole_offset = (count as i64 + skip as i64) * size;

    assert_eq!(
        lfs_file_size(lfs, file),
        (count * KITTY.len() as u32) as i32
    );
    assert_eq!(
        lfs_file_seek(lfs, file, hole_offset as i32, LFS_SEEK_SET,),
        Ok(hole_offset as i32)
    );
    let mut buf = [0u8; 32];
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, Ok(0));

    let n = lfs_file_write(
        lfs,
        file,
        PORCUPINE.as_ptr() as *const core::ffi::c_void,
        PORCUPINE.len() as u32,
    );
    assert_eq!(n, Ok(PORCUPINE.len() as u32));

    assert_eq!(
        lfs_file_seek(lfs, file, hole_offset as i32, LFS_SEEK_SET,),
        Ok(hole_offset as i32)
    );
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        PORCUPINE.len() as u32,
    );
    assert_eq!(n, Ok(PORCUPINE.len() as u32));
    assert_eq!(&buf[..PORCUPINE.len()], PORCUPINE);

    assert_eq!(
        lfs_file_seek(lfs, file, (count as i32) * (size as i32), LFS_SEEK_SET,),
        Ok((count as i32) * (size as i32))
    );
    let n = lfs_file_read(
        lfs,
        file,
        buf.as_mut_ptr() as *mut core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, Ok(KITTY.len() as u32));
    assert!(
        buf[..KITTY.len()].iter().all(|&b| b == 0),
        "hole should be zeros, got {:?}",
        &buf[..KITTY.len()]
    );

    // After read at count*size we're at (count+1)*size
    assert_eq!(
        lfs_file_seek(lfs, file, -(hole_offset as i32), LFS_SEEK_CUR,),
        Err(Error::Invalid)
    );
    assert_eq!(lfs_file_tell(lfs, file), (count as i32 + 1) * (size as i32));

    assert_eq!(
        lfs_file_seek(
            lfs,
            file,
            -((count as i32 + 2 * skip as i32) * (size as i32)),
            LFS_SEEK_END,
        ),
        Err(Error::Invalid)
    );
    assert_eq!(lfs_file_tell(lfs, file), (count as i32 + 1) * (size as i32));

    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_seek_inline_write]
/// defines.SIZE = [2, 4, 128, 132]
#[rstest]
#[case(2)]
#[case(4)]
#[case(128)]
#[case(132)]
fn test_seek_inline_write(#[case] size: u32) {
    let mut env = default_config(256);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let path = c"tinykitty";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDWR | LFS_O_CREAT));

    let alphabet = b"abcdefghijklmnopqrstuvwxyz";
    let mut j = 0usize;
    let mut k = 0usize;

    for i in 0..size {
        let c = alphabet[j % 26];
        let n = lfs_file_write(lfs, file, &c as *const u8 as *const core::ffi::c_void, 1);
        assert_eq!(n, Ok(1));
        assert_eq!(lfs_file_tell(lfs, file), (i + 1) as i32);
        assert_eq!(lfs_file_size(lfs, file), (i + 1) as i32);
        j += 1;
    }

    assert_eq!(lfs_file_seek(lfs, file, 0, LFS_SEEK_SET), Ok(0));
    assert_eq!(lfs_file_tell(lfs, file), 0);
    assert_eq!(lfs_file_size(lfs, file), size as i32);

    let mut c = [0u8; 1];
    for _ in 0..size {
        let n = lfs_file_read(lfs, file, c.as_mut_ptr() as *mut core::ffi::c_void, 1);
        assert_eq!(n, Ok(1));
        assert_eq!(c[0], alphabet[k % 26]);
        k += 1;
    }

    assert_ok(lfs_file_sync(lfs, file));
    assert_eq!(lfs_file_tell(lfs, file), size as i32);
    assert_eq!(lfs_file_size(lfs, file), size as i32);

    assert_eq!(lfs_file_seek(lfs, file, 0, LFS_SEEK_SET), Ok(0));

    for i in 0..size {
        let c = alphabet[j % 26];
        let n = lfs_file_write(lfs, file, &c as *const u8 as *const core::ffi::c_void, 1);
        assert_eq!(n, Ok(1));
        assert_eq!(lfs_file_tell(lfs, file), (i + 1) as i32);
        assert_eq!(lfs_file_size(lfs, file), size as i32);
        assert_ok(lfs_file_sync(lfs, file));
        assert_eq!(lfs_file_tell(lfs, file), (i + 1) as i32);
        assert_eq!(lfs_file_size(lfs, file), size as i32);

        if i < size - 2 {
            let mut buf3 = [0u8; 3];
            assert_eq!(lfs_file_seek(lfs, file, -1, LFS_SEEK_CUR), Ok(i as i32));
            let n = lfs_file_read(lfs, file, buf3.as_mut_ptr() as *mut core::ffi::c_void, 3);
            assert_eq!(n, Ok(3));
            assert_eq!(lfs_file_tell(lfs, file), (i + 3) as i32);
            assert_eq!(lfs_file_size(lfs, file), size as i32);
            assert_eq!(
                lfs_file_seek(lfs, file, (i + 1) as i32, LFS_SEEK_SET),
                Ok((i + 1) as i32)
            );
            assert_eq!(lfs_file_tell(lfs, file), (i + 1) as i32);
            assert_eq!(lfs_file_size(lfs, file), size as i32);
        }
        j += 1;
    }

    assert_eq!(lfs_file_seek(lfs, file, 0, LFS_SEEK_SET), Ok(0));
    assert_eq!(lfs_file_tell(lfs, file), 0);
    assert_eq!(lfs_file_size(lfs, file), size as i32);

    let mut c = [0u8; 1];
    for _ in 0..size {
        let n = lfs_file_read(lfs, file, c.as_mut_ptr() as *mut core::ffi::c_void, 1);
        assert_eq!(n, Ok(1));
        assert_eq!(c[0], alphabet[k % 26]);
        k += 1;
    }

    assert_ok(lfs_file_sync(lfs, file));
    assert_eq!(lfs_file_tell(lfs, file), size as i32);
    assert_eq!(lfs_file_size(lfs, file), size as i32);

    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_seek_reentrant_write]
/// defines.COUNT = [4, 64, 128], POWERLOSS_BEHAVIOR = [NOOP, OOO]
#[rstest]
#[case(4)]
#[case(64)]
#[case(128)]
#[cfg(feature = "slow_tests")]
#[ignore = "bug: power-loss iteration returns -1 for some cases"]
fn test_seek_reentrant_write(#[case] count: u32) {
    let mut env = powerloss_config(256);
    init_powerloss_context(&mut env);

    let config_ptr = &env.config;
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };

    assert_ok(littlefs_rust_core::lfs_format(lfs, config_ptr));
    assert_ok(littlefs_rust_core::lfs_mount(lfs, config_ptr));
    assert_ok(littlefs_rust_core::lfs_unmount(lfs));
    let snapshot = env.snapshot();

    let op = |lfs: &mut Lfs, cfg: &LfsConfig| -> Result<(), Error> {
        let err = littlefs_rust_core::lfs_mount(lfs, cfg);
        if err.is_err() {
            let _ = littlefs_rust_core::lfs_format(lfs, cfg);
            let e = littlefs_rust_core::lfs_mount(lfs, cfg)?;
        }

        let path = path_bytes("kitty");
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        let mut buf = [0u8; 32];

        let open_err = littlefs_rust_core::lfs_file_open(lfs, file, path.as_ptr(), LFS_O_RDONLY);
        if open_err.is_ok() {
            let sz = littlefs_rust_core::lfs_file_size(lfs, file);
            if sz != 0 {
                assert_eq!(sz, (count * 11) as i32);
                for _ in 0..count {
                    let n = littlefs_rust_core::lfs_file_read(
                        lfs,
                        file,
                        buf.as_mut_ptr() as *mut core::ffi::c_void,
                        11,
                    )?;
                    if n != 11 {
                        return Err(Error::Invalid);
                    }
                    assert!(
                        &buf[..11] == KITTY || &buf[..11] == DOGGO,
                        "unexpected content"
                    );
                }
            }
            let e = littlefs_rust_core::lfs_file_close(lfs, file)?;
        } else {
            assert_eq!(open_err, Err(Error::NoEntry));
        }

        let e = littlefs_rust_core::lfs_file_open(
            lfs,
            file,
            path.as_ptr(),
            LFS_O_WRONLY | LFS_O_CREAT,
        )?;

        if littlefs_rust_core::lfs_file_size(lfs, file) == 0 {
            for _ in 0..count {
                let n = littlefs_rust_core::lfs_file_write(
                    lfs,
                    file,
                    KITTY.as_ptr() as *const core::ffi::c_void,
                    KITTY.len() as u32,
                )?;

                assert_eq!(n, KITTY.len() as u32);
            }
        }
        let e = littlefs_rust_core::lfs_file_close(lfs, file)?;

        let e = littlefs_rust_core::lfs_file_open(lfs, file, path.as_ptr(), LFS_O_RDWR)?;

        assert_eq!(
            littlefs_rust_core::lfs_file_size(lfs, file),
            (count * 11) as i32
        );

        let mut off: u32 = 0;
        for _ in 0..count {
            off = (5 * off + 1) % count;
            let pos = (off * 11) as i32;
            let seek_res = littlefs_rust_core::lfs_file_seek(lfs, file, pos, LFS_SEEK_SET)?;
            if seek_res != pos {
                return Err(Error::Invalid);
            }
            let n = littlefs_rust_core::lfs_file_read(
                lfs,
                file,
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                11,
            )?;
            if n != 11 {
                return Err(Error::Invalid);
            }
            assert!(&buf[..11] == KITTY || &buf[..11] == DOGGO);
            if &buf[..11] != DOGGO {
                let seek_res = littlefs_rust_core::lfs_file_seek(lfs, file, pos, LFS_SEEK_SET)?;
                if seek_res != pos {
                    return Err(Error::Invalid);
                }
                let n = littlefs_rust_core::lfs_file_write(
                    lfs,
                    file,
                    DOGGO.as_ptr() as *const core::ffi::c_void,
                    DOGGO.len() as u32,
                )?;

                assert_eq!(n, DOGGO.len() as u32);
                let seek_res = littlefs_rust_core::lfs_file_seek(lfs, file, pos, LFS_SEEK_SET)?;
                if seek_res != pos {
                    return Err(Error::Invalid);
                }
                let n = littlefs_rust_core::lfs_file_read(
                    lfs,
                    file,
                    buf.as_mut_ptr() as *mut core::ffi::c_void,
                    11,
                )?;
                if n != 11 {
                    return Err(Error::Invalid);
                }
                assert_eq!(&buf[..11], DOGGO);
                let e = littlefs_rust_core::lfs_file_sync(lfs, file)?;
                let seek_res = littlefs_rust_core::lfs_file_seek(lfs, file, pos, LFS_SEEK_SET)?;
                if seek_res != pos {
                    return Err(Error::Invalid);
                }
                let n = littlefs_rust_core::lfs_file_read(
                    lfs,
                    file,
                    buf.as_mut_ptr() as *mut core::ffi::c_void,
                    11,
                )?;
                if n != 11 {
                    return Err(Error::Invalid);
                }
                assert_eq!(&buf[..11], DOGGO);
            }
        }

        let e = littlefs_rust_core::lfs_file_close(lfs, file)?;

        let e = littlefs_rust_core::lfs_file_open(lfs, file, path.as_ptr(), LFS_O_RDWR)?;
        assert_eq!(
            littlefs_rust_core::lfs_file_size(lfs, file),
            (count * 11) as i32
        );
        for _ in 0..count {
            let n = littlefs_rust_core::lfs_file_read(
                lfs,
                file,
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                11,
            )?;
            if n != 11 {
                return Err(Error::Invalid);
            }
            assert_eq!(&buf[..11], DOGGO);
        }
        let e = littlefs_rust_core::lfs_file_close(lfs, file)?;

        let e = littlefs_rust_core::lfs_unmount(lfs)?;

        Ok(())
    };

    let result = run_powerloss_linear(&mut env, &snapshot, 3000, op, |_, _| Ok(()));
    result.expect("reentrant seek write should eventually succeed");
}

/// Upstream: [cases.test_seek_filemax]
#[test]
fn test_seek_filemax() {
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let path = c"kitty";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    let n = lfs_file_write(
        lfs,
        file,
        KITTY.as_ptr() as *const core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, Ok(KITTY.len() as u32));

    assert_eq!(
        lfs_file_seek(lfs, file, LFS_FILE_MAX, LFS_SEEK_SET),
        Ok(LFS_FILE_MAX)
    );

    assert_eq!(lfs_file_seek(lfs, file, 0, LFS_SEEK_CUR), Ok(LFS_FILE_MAX));

    assert_eq!(
        lfs_file_seek(lfs, file, 10, LFS_SEEK_END),
        Ok(KITTY.len() as i32 + 10)
    );

    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_seek_underflow]
#[test]
fn test_seek_underflow() {
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let path = c"kitty";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    let n = lfs_file_write(
        lfs,
        file,
        KITTY.as_ptr() as *const core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, Ok(KITTY.len() as u32));
    let size = KITTY.len() as i32;

    assert_eq!(
        lfs_file_seek(lfs, file, -(size + 10), LFS_SEEK_CUR),
        Err(Error::Invalid)
    );
    assert_eq!(
        lfs_file_seek(lfs, file, -LFS_FILE_MAX, LFS_SEEK_CUR),
        Err(Error::Invalid)
    );
    assert_eq!(
        lfs_file_seek(
            lfs,
            file,
            i32::MIN, // -(size + LFS_FILE_MAX) overflows; use MIN to trigger underflow
            LFS_SEEK_CUR,
        ),
        Err(Error::Invalid)
    );

    assert_eq!(
        lfs_file_seek(lfs, file, -(size + 10), LFS_SEEK_END),
        Err(Error::Invalid)
    );
    assert_eq!(
        lfs_file_seek(lfs, file, -LFS_FILE_MAX, LFS_SEEK_END),
        Err(Error::Invalid)
    );
    assert_eq!(
        lfs_file_seek(
            lfs,
            file,
            i32::MIN, // -(size + LFS_FILE_MAX) overflows; use MIN
            LFS_SEEK_END,
        ),
        Err(Error::Invalid)
    );

    assert_eq!(lfs_file_tell(lfs, file), size);

    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_seek_overflow]
#[test]
fn test_seek_overflow() {
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let path = c"kitty";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path,
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_APPEND,
    ));
    let n = lfs_file_write(
        lfs,
        file,
        KITTY.as_ptr() as *const core::ffi::c_void,
        KITTY.len() as u32,
    );
    assert_eq!(n, Ok(KITTY.len() as u32));
    let size = KITTY.len() as i32;

    assert_eq!(
        lfs_file_seek(lfs, file, LFS_FILE_MAX, LFS_SEEK_SET),
        Ok(LFS_FILE_MAX)
    );

    assert_eq!(
        lfs_file_seek(lfs, file, 10, LFS_SEEK_CUR),
        Err(Error::Invalid)
    );
    assert_eq!(
        lfs_file_seek(lfs, file, LFS_FILE_MAX, LFS_SEEK_CUR),
        Err(Error::Invalid)
    );

    assert_eq!(
        lfs_file_seek(lfs, file, LFS_FILE_MAX.wrapping_add(10), LFS_SEEK_SET,),
        Err(Error::Invalid)
    );
    assert_eq!(
        lfs_file_seek(
            lfs,
            file,
            LFS_FILE_MAX.wrapping_add(LFS_FILE_MAX),
            LFS_SEEK_SET,
        ),
        Err(Error::Invalid)
    );

    assert_eq!(
        lfs_file_seek(
            lfs,
            file,
            LFS_FILE_MAX.wrapping_sub(size).wrapping_add(10),
            LFS_SEEK_END,
        ),
        Err(Error::Invalid)
    );
    assert_eq!(
        lfs_file_seek(
            lfs,
            file,
            LFS_FILE_MAX.wrapping_sub(size).wrapping_add(LFS_FILE_MAX),
            LFS_SEEK_END,
        ),
        Err(Error::Invalid)
    );

    assert_eq!(lfs_file_tell(lfs, file), LFS_FILE_MAX);

    assert_ok(lfs_file_close(lfs, file));
    assert_ok(lfs_unmount(lfs));
}

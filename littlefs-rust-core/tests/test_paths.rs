//! Path resolution integration tests.
//!
//! Upstream: tests/test_paths.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_paths.toml

mod common;

use std::ffi::CStr;

use common::{assert_err, assert_ok, default_config, init_context, init_logger, path_bytes};
#[allow(unused_imports)]
use littlefs_rust_core::lfs_type::lfs_type::{LFS_TYPE_DIR, LFS_TYPE_REG};
use littlefs_rust_core::{
    Lfs, LfsDir, LfsInfo, error::Error, lfs_dir_close, lfs_dir_open, lfs_format, lfs_mkdir,
    lfs_mount, lfs_remove, lfs_rename, lfs_stat, lfs_unmount,
};
use littlefs_rust_core::{LfsFile, lfs_file_close, lfs_file_open};
use rstest::rstest;

use common::{LFS_O_CREAT, LFS_O_EXCL, LFS_O_RDONLY, LFS_O_WRONLY};

fn info_name_str(info: &LfsInfo) -> &str {
    let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
    core::str::from_utf8(&info.name[..nul]).unwrap_or("")
}

/// Null-terminated path from raw bytes (for non-UTF8 names like 0x7f, 0xff).
fn path_bytes_raw(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

const PATHS: &[&str] = &[
    "drip",
    "coldbrew",
    "turkish",
    "tubruk",
    "vietnamese",
    "thai",
];

// --- test_paths_simple_dirs ---
#[test]
fn test_paths_simple_dirs() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let coffee = path_bytes("coffee");
    assert_ok(lfs_mkdir(lfs, coffee.as_c_str()));

    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        assert_ok(lfs_mkdir(lfs, path.as_c_str()));
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path.as_c_str(), info));
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
        assert_eq!(info.type_, LFS_TYPE_DIR as u8);
    }
    assert_ok(lfs_unmount(lfs));
}

// --- test_paths_simple_files ---
#[test]
fn test_paths_simple_files() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let coffee = path_bytes("coffee");
    assert_ok(lfs_mkdir(lfs, coffee.as_c_str()));

    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            path.as_c_str(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs, file));
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path.as_c_str(), info));
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
        assert_eq!(info.type_, LFS_TYPE_REG as u8);
    }
    assert_ok(lfs_unmount(lfs));
}

// --- test_paths_absolute_files ---
#[test]
fn test_paths_absolute_files() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let coffee = path_bytes("coffee");
    assert_ok(lfs_mkdir(lfs, coffee.as_c_str()));

    for name in PATHS {
        let path = path_bytes(&format!("/coffee/{name}"));
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            path.as_c_str(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs, file));
    }
    for name in PATHS {
        let path = path_bytes(&format!("/coffee/{name}"));
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path.as_c_str(), info));
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
        assert_eq!(info.type_, LFS_TYPE_REG as u8);
    }
    assert_ok(lfs_unmount(lfs));
}

// --- test_paths_absolute_dirs ---
#[test]
fn test_paths_absolute_dirs() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let coffee = path_bytes("coffee");
    assert_ok(lfs_mkdir(lfs, coffee.as_c_str()));

    for name in PATHS {
        let path = path_bytes(&format!("/coffee/{name}"));
        assert_ok(lfs_mkdir(lfs, path.as_c_str()));
    }
    for name in PATHS {
        let path = path_bytes(&format!("/coffee/{name}"));
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path.as_c_str(), info));
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
        assert_eq!(info.type_, LFS_TYPE_DIR as u8);
    }
    assert_ok(lfs_unmount(lfs));
}

// --- test_paths_noent ---
#[test]
fn test_paths_noent() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let coffee = path_bytes("coffee");
    assert_ok(lfs_mkdir(lfs, coffee.as_c_str()));
    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        assert_ok(lfs_mkdir(lfs, path.as_c_str()));
    }

    for bad in &[
        "_rip",
        "c_ldbrew",
        "tu_kish",
        "tub_uk",
        "_vietnamese",
        "thai_",
    ] {
        let path = path_bytes(&format!("coffee/{bad}"));
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        let err = lfs_stat(lfs, path.as_c_str(), info);
        assert_err(Error::NoEntry, err);

        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        let err = lfs_file_open(lfs, file, path.as_c_str(), LFS_O_RDONLY);
        assert_err(Error::NoEntry, err);
    }
    assert_ok(lfs_unmount(lfs));
}

// --- test_paths_root ---
#[test]
fn test_paths_root() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);

    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let root_path = path_bytes("/");
    let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
    assert_ok(lfs_dir_open(lfs, dir, root_path.as_c_str()));
    assert_ok(lfs_dir_close(lfs, dir));

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, root_path.as_c_str(), info));
    assert_eq!(info.type_, LFS_TYPE_DIR as u8);

    assert_ok(lfs_unmount(lfs));
}

// --- Deferred edge-case tests (per roadmap 07a) ---

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_redundant_slashes(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_c_str()));
    let create_paths = &[
        "/coffee/drip",
        "//coffee//coldbrew",
        "///coffee///turkish",
        "////coffee////tubruk",
        "/////coffee/////vietnamese",
        "//////coffee//////thai",
    ];
    for path_str in create_paths {
        let path = path_bytes(path_str);
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_c_str()));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path.as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }

    let stat_paths = &[
        "//////coffee//////drip",
        "/////coffee/////coldbrew",
        "////coffee////turkish",
        "///coffee///tubruk",
        "//coffee//vietnamese",
        "/coffee/thai",
    ];
    let expect_names = [
        "drip",
        "coldbrew",
        "turkish",
        "tubruk",
        "vietnamese",
        "thai",
    ];
    for (path_str, expect) in stat_paths.iter().zip(expect_names) {
        let path = path_bytes(path_str);
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path.as_c_str(), info));
        assert_eq!(info_name_str(&info), expect);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }

    assert_ok(lfs_mkdir(lfs, path_bytes("espresso").as_c_str()));
    let renames = &[
        ("//////coffee//////drip", "/espresso/espresso"),
        ("/////coffee/////coldbrew", "//espresso//americano"),
        ("////coffee////turkish", "///espresso///macchiato"),
        ("///coffee///tubruk", "////espresso////latte"),
        ("//coffee//vietnamese", "/////espresso/////cappuccino"),
        ("/coffee/thai", "//////espresso//////mocha"),
    ];
    for (old, new) in renames {
        assert_ok(lfs_rename(
            lfs,
            path_bytes(old).as_c_str(),
            path_bytes(new).as_c_str(),
        ));
    }
    let mut info_dummy = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    for path_str in stat_paths {
        let path = path_bytes(path_str);
        assert_err(
            Error::NoEntry,
            lfs_stat(lfs, path.as_c_str(), info_dummy.as_mut_ptr()),
        );
    }
    for path_str in &[
        "/espresso/espresso",
        "//espresso//americano",
        "///espresso///macchiato",
        "////espresso////latte",
        "/////espresso/////cappuccino",
        "/espresso/mocha",
    ] {
        assert_ok(lfs_remove(lfs, path_bytes(path_str).as_c_str()));
    }
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_trailing_slashes(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_c_str()));
    if dir_mode {
        for s in &[
            "coffee/drip/",
            "coffee/coldbrew//",
            "coffee/turkish///",
            "coffee/tubruk////",
            "coffee/vietnamese/////",
            "coffee/thai//////",
        ] {
            assert_ok(lfs_mkdir(lfs, path_bytes(s).as_c_str()));
        }
    } else {
        for s in &[
            "coffee/drip/",
            "coffee/coldbrew//",
            "coffee/turkish///",
            "coffee/tubruk////",
            "coffee/vietnamese/////",
            "coffee/thai//////",
        ] {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_err(
                Error::NotDir,
                lfs_file_open(
                    lfs,
                    file,
                    path_bytes(s).as_c_str(),
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                ),
            );
        }
        for name in PATHS {
            let path = path_bytes(&format!("coffee/{name}"));
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path.as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }

    let stat_slashes = &[
        "coffee/drip//////",
        "coffee/coldbrew/////",
        "coffee/turkish////",
        "coffee/tubruk///",
        "coffee/vietnamese//",
        "coffee/thai/",
    ];
    for (i, path_str) in stat_slashes.iter().enumerate() {
        let path = path_bytes(path_str);
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        let err = lfs_stat(lfs, path.as_c_str(), info);
        if dir_mode {
            assert_ok(err);
            assert_eq!(info_name_str(&info), PATHS[i]);
            assert_eq!(info.type_, LFS_TYPE_DIR as u8);
        } else {
            assert_err(Error::NotDir, err);
        }
    }

    assert_ok(lfs_mkdir(lfs, path_bytes("espresso").as_c_str()));
    if dir_mode {
        let renames = &[
            ("coffee/drip//////", "espresso/espresso/"),
            ("coffee/coldbrew/////", "espresso/americano//"),
            ("coffee/turkish////", "espresso/macchiato///"),
            ("coffee/tubruk///", "espresso/latte////"),
            ("coffee/vietnamese//", "espresso/cappuccino/////"),
            ("coffee/thai/", "espresso/mocha//////"),
        ];
        for (old, new) in renames {
            assert_ok(lfs_rename(
                lfs,
                path_bytes(old).as_c_str(),
                path_bytes(new).as_c_str(),
            ));
        }
        for s in &[
            "espresso/espresso/",
            "espresso/americano//",
            "espresso/macchiato///",
            "espresso/latte////",
            "espresso/cappuccino/////",
            "espresso/mocha//////",
        ] {
            assert_ok(lfs_remove(lfs, path_bytes(s).as_c_str()));
        }
    }
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_dots(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_c_str()));
    let create_paths = &[
        "/coffee/drip",
        "/./coffee/./coldbrew",
        "/././coffee/././turkish",
        "/./././coffee/./././tubruk",
        "/././././coffee/././././vietnamese",
        "/./././././coffee/./././././thai",
    ];
    for path_str in create_paths {
        let path = path_bytes(path_str);
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_c_str()));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path.as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }

    let stat_paths = &[
        "/no/no/../../no/no/../../coffee/drip",
        "/no/no/../../coffee/no/../coldbrew",
        "/no/no/../../coffee/turkish",
        "/coffee/no/../tubruk",
        "/no/../coffee/vietnamese",
        "/coffee/thai",
    ];
    let expect_names = [
        "drip",
        "coldbrew",
        "turkish",
        "tubruk",
        "vietnamese",
        "thai",
    ];
    for (path_str, expect) in stat_paths.iter().zip(expect_names) {
        let path = path_bytes(path_str);
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path.as_c_str(), info));
        assert_eq!(info_name_str(&info), expect);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }

    assert_ok(lfs_mkdir(lfs, path_bytes("espresso").as_c_str()));
    let renames = &[
        ("/no/no/../../no/no/../../coffee/drip", "/espresso/espresso"),
        (
            "/no/no/../../coffee/no/../coldbrew",
            "/./espresso/./americano",
        ),
        ("/no/no/../../coffee/turkish", "/././espresso/././macchiato"),
        ("/coffee/no/../tubruk", "/./././espresso/./././latte"),
        (
            "/no/../coffee/vietnamese",
            "/././././espresso/././././cappuccino",
        ),
        ("/coffee/thai", "/./././././espresso/./././././mocha"),
    ];
    for (old, new) in renames {
        assert_ok(lfs_rename(
            lfs,
            path_bytes(old).as_c_str(),
            path_bytes(new).as_c_str(),
        ));
    }
    for s in &[
        "/espresso/espresso",
        "/./espresso/./americano",
        "/././espresso/././macchiato",
        "/./././espresso/./././latte",
        "/././././espresso/././././cappuccino",
        "/./././././espresso/./././././mocha",
    ] {
        assert_ok(lfs_remove(lfs, path_bytes(s).as_c_str()));
    }
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_trailing_dots(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_c_str()));
    if dir_mode {
        for s in &[
            "coffee/drip/.",
            "coffee/coldbrew/./.",
            "coffee/turkish/././.",
            "coffee/tubruk/./././.",
            "coffee/vietnamese/././././.",
            "coffee/thai/./././././.",
        ] {
            assert_err(Error::NoEntry, lfs_mkdir(lfs, path_bytes(s).as_c_str()));
        }
        for name in PATHS {
            assert_ok(lfs_mkdir(
                lfs,
                path_bytes(&format!("coffee/{name}")).as_c_str(),
            ));
        }
    } else {
        for s in &[
            "coffee/drip/.",
            "coffee/coldbrew/./.",
            "coffee/turkish/././.",
            "coffee/tubruk/./././.",
            "coffee/vietnamese/././././.",
            "coffee/thai/./././././.",
        ] {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_err(
                Error::NoEntry,
                lfs_file_open(
                    lfs,
                    file,
                    path_bytes(s).as_c_str(),
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                ),
            );
        }
        for name in PATHS {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path_bytes(&format!("coffee/{name}")).as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }

    let stat_dots = &[
        "coffee/drip/./././././.",
        "coffee/coldbrew/././././.",
        "coffee/turkish/./././.",
        "coffee/tubruk/././.",
        "coffee/vietnamese/./.",
        "coffee/thai/.",
    ];
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    for (i, path_str) in stat_dots.iter().enumerate() {
        let path = path_bytes(path_str);
        let err = lfs_stat(lfs, path.as_c_str(), info);
        if dir_mode {
            assert_ok(err);
            assert_eq!(info_name_str(info), PATHS[i]);
            assert_eq!(info.type_, LFS_TYPE_DIR as u8);
        } else {
            assert_err(Error::NotDir, err);
        }
    }

    assert_ok(lfs_mkdir(lfs, path_bytes("espresso").as_c_str()));
    if dir_mode {
        let renames_ok = &[
            ("coffee/drip/./././././.", "espresso/espresso"),
            ("coffee/coldbrew/././././.", "espresso/americano"),
            ("coffee/turkish/./././.", "espresso/macchiato"),
            ("coffee/tubruk/././.", "espresso/latte"),
            ("coffee/vietnamese/./.", "espresso/cappuccino"),
            ("coffee/thai/.", "espresso/mocha"),
        ];
        for (old, new) in renames_ok {
            assert_ok(lfs_rename(
                lfs,
                path_bytes(old).as_c_str(),
                path_bytes(new).as_c_str(),
            ));
        }
        for s in &[
            "espresso/espresso/.",
            "espresso/americano/./.",
            "espresso/macchiato/././.",
            "espresso/latte/./././.",
            "espresso/cappuccino/././././.",
            "espresso/mocha/./././././.",
        ] {
            assert_ok(lfs_remove(lfs, path_bytes(s).as_c_str()));
        }
    }
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_dotdots(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("no").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("no/no").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("coffee/no").as_c_str()));
    let create_paths = &[
        "/coffee/drip",
        "/no/../coffee/coldbrew",
        "/coffee/no/../turkish",
        "/no/no/../../coffee/tubruk",
        "/no/no/../../coffee/no/../vietnamese",
        "/no/no/../../no/no/../../coffee/thai",
    ];
    for path_str in create_paths {
        let path = path_bytes(path_str);
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_c_str()));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path.as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }

    let stat_paths = &[
        "/./././././coffee/./././././drip",
        "/././././coffee/././././coldbrew",
        "/./././coffee/./././turkish",
        "/././coffee/././tubruk",
        "/./coffee/./vietnamese",
        "/coffee/thai",
    ];
    let expect_names = [
        "drip",
        "coldbrew",
        "turkish",
        "tubruk",
        "vietnamese",
        "thai",
    ];
    for (path_str, expect) in stat_paths.iter().zip(expect_names) {
        let path = path_bytes(path_str);
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path.as_c_str(), info));
        assert_eq!(info_name_str(&info), expect);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }

    assert_ok(lfs_mkdir(lfs, path_bytes("espresso").as_c_str()));
    let renames = &[
        ("/./././././coffee/./././././drip", "/espresso/espresso"),
        (
            "/././././coffee/././././coldbrew",
            "/./espresso/./americano",
        ),
        ("/./././coffee/./././turkish", "/././espresso/././macchiato"),
        ("/./././coffee/././tubruk", "/./././espresso/./././latte"),
        (
            "/./coffee/./vietnamese",
            "/././././espresso/././././cappuccino",
        ),
        ("/coffee/thai", "/./././././espresso/./././././mocha"),
    ];
    for (old, new) in renames {
        assert_ok(lfs_rename(
            lfs,
            path_bytes(old).as_c_str(),
            path_bytes(new).as_c_str(),
        ));
    }
    for s in &[
        "/espresso/espresso",
        "/./espresso/./americano",
        "/././espresso/././macchiato",
        "/./././espresso/./././latte",
        "/././././espresso/././././cappuccino",
        "/./././././espresso/./././././mocha",
    ] {
        assert_ok(lfs_remove(lfs, path_bytes(s).as_c_str()));
    }
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_trailing_dotdots(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_c_str()));

    if dir_mode {
        assert_err(
            Error::Exists,
            lfs_mkdir(lfs, path_bytes("coffee/drip/..").as_c_str()),
        );
        assert_err(
            Error::Exists,
            lfs_mkdir(lfs, path_bytes("coffee/coldbrew/../..").as_c_str()),
        );
        assert_err(
            Error::Invalid,
            lfs_mkdir(lfs, path_bytes("coffee/turkish/../../..").as_c_str()),
        );
        assert_err(
            Error::Invalid,
            lfs_mkdir(lfs, path_bytes("coffee/tubruk/../../../..").as_c_str()),
        );
        assert_err(
            Error::Invalid,
            lfs_mkdir(
                lfs,
                path_bytes("coffee/vietnamese/../../../../..").as_c_str(),
            ),
        );
        assert_err(
            Error::Invalid,
            lfs_mkdir(lfs, path_bytes("coffee/thai/../../../../../..").as_c_str()),
        );
        for name in PATHS {
            assert_ok(lfs_mkdir(
                lfs,
                path_bytes(&format!("coffee/{name}")).as_c_str(),
            ));
        }
    } else {
        assert_err(
            Error::Exists,
            lfs_file_open(
                lfs,
                unsafe { &mut *core::mem::MaybeUninit::<LfsFile>::zeroed().as_mut_ptr() },
                path_bytes("coffee/drip/..").as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        assert_err(
            Error::Exists,
            lfs_file_open(
                lfs,
                unsafe { &mut *core::mem::MaybeUninit::<LfsFile>::zeroed().as_mut_ptr() },
                path_bytes("coffee/coldbrew/../..").as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        assert_err(
            Error::Invalid,
            lfs_file_open(
                lfs,
                unsafe { &mut *core::mem::MaybeUninit::<LfsFile>::zeroed().as_mut_ptr() },
                path_bytes("coffee/turkish/../../..").as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        assert_err(
            Error::Invalid,
            lfs_file_open(
                lfs,
                unsafe { &mut *core::mem::MaybeUninit::<LfsFile>::zeroed().as_mut_ptr() },
                path_bytes("coffee/tubruk/../../../..").as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        assert_err(
            Error::Invalid,
            lfs_file_open(
                lfs,
                unsafe { &mut *core::mem::MaybeUninit::<LfsFile>::zeroed().as_mut_ptr() },
                path_bytes("coffee/vietnamese/../../../../..").as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        assert_err(
            Error::Invalid,
            lfs_file_open(
                lfs,
                unsafe { &mut *core::mem::MaybeUninit::<LfsFile>::zeroed().as_mut_ptr() },
                path_bytes("coffee/thai/../../../../../..").as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        for name in PATHS {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path_bytes(&format!("coffee/{name}")).as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }

    // stat paths
    let mut info_err = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_err(
        Error::Invalid,
        lfs_stat(lfs, c"coffee/drip/../../../../../..", info_err.as_mut_ptr()),
    );
    assert_err(
        Error::Invalid,
        lfs_stat(
            lfs,
            c"coffee/coldbrew/../../../../..",
            info_err.as_mut_ptr(),
        ),
    );
    assert_err(
        Error::Invalid,
        lfs_stat(lfs, c"coffee/turkish/../../../..", info_err.as_mut_ptr()),
    );
    assert_err(
        Error::Invalid,
        lfs_stat(lfs, c"coffee/tubruk/../../..", info_err.as_mut_ptr()),
    );

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, c"coffee/vietnamese/../..", info));
    assert_eq!(info_name_str(&info), "/");
    assert_eq!(info.type_, LFS_TYPE_DIR as u8);

    let mut info2 = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(lfs, c"coffee/thai/..", info2.as_mut_ptr()));
    let info2 = unsafe { info2.assume_init() };
    assert_eq!(info_name_str(&info2), "coffee");
    assert_eq!(info2.type_, LFS_TYPE_DIR as u8);

    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_dot_dotdots(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("no").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("no/no").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("coffee/no").as_c_str()));

    if dir_mode {
        assert_ok(lfs_mkdir(lfs, path_bytes("/coffee/drip").as_c_str()));
        assert_ok(lfs_mkdir(
            lfs,
            path_bytes("/no/./../coffee/coldbrew").as_c_str(),
        ));
        assert_ok(lfs_mkdir(
            lfs,
            path_bytes("/coffee/no/./../turkish").as_c_str(),
        ));
        assert_ok(lfs_mkdir(
            lfs,
            path_bytes("/no/no/./.././../coffee/tubruk").as_c_str(),
        ));
        assert_ok(lfs_mkdir(
            lfs,
            path_bytes("/no/no/./.././../coffee/no/./../vietnamese").as_c_str(),
        ));
        assert_ok(lfs_mkdir(
            lfs,
            path_bytes("/no/no/./.././../no/no/./.././../coffee/thai").as_c_str(),
        ));
    } else {
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            path_bytes("/coffee/drip").as_c_str(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs, file));
        assert_ok(lfs_file_open(
            lfs,
            file,
            path_bytes("/no/./../coffee/coldbrew").as_c_str(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs, file));
        for path in [
            "/coffee/no/./../turkish",
            "/no/no/./.././../coffee/tubruk",
            "/no/no/./.././../coffee/no/./../vietnamese",
            "/no/no/./.././../no/no/./.././../coffee/thai",
        ] {
            let mut f = unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                &mut f,
                path_bytes(path).as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, &mut f));
        }
    }

    // stat paths
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(
        lfs,
        c"/no/no/./.././../no/no/./.././../coffee/drip",
        info,
    ));
    assert_eq!(info_name_str(&info), "drip");
    assert_eq!(
        info.type_,
        if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
    );

    let mut info2 = core::mem::MaybeUninit::<LfsInfo>::zeroed();
    assert_ok(lfs_stat(
        lfs,
        c"/no/no/./.././../coffee/no/./../coldbrew",
        info2.as_mut_ptr(),
    ));
    let info2 = unsafe { info2.assume_init() };
    assert_eq!(info_name_str(&info2), "coldbrew");
    assert_eq!(
        info2.type_,
        if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
    );

    for (path, expected_name) in [
        ("/no/no/./.././../coffee/turkish", "turkish"),
        ("/coffee/no/./../tubruk", "tubruk"),
        ("/no/./../coffee/vietnamese", "vietnamese"),
        ("/coffee/thai", "thai"),
    ] {
        let mut i = core::mem::MaybeUninit::<LfsInfo>::zeroed();
        assert_ok(lfs_stat(lfs, path_bytes(path).as_c_str(), i.as_mut_ptr()));
        let i = unsafe { i.assume_init() };
        assert_eq!(info_name_str(&i), expected_name);
        assert_eq!(
            i.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }

    if dir_mode {
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        for path in [
            "/coffee/drip",
            "/no/./../coffee/coldbrew",
            "/coffee/no/./../turkish",
            "/no/no/./.././../coffee/tubruk",
            "/no/no/./.././../coffee/no/./../vietnamese",
            "/no/no/./.././../no/no/./.././../coffee/thai",
        ] {
            assert_err(
                Error::IsDir,
                lfs_file_open(lfs, file, path_bytes(path).as_c_str(), LFS_O_RDONLY),
            );
            assert_err(
                Error::IsDir,
                lfs_file_open(
                    lfs,
                    file,
                    path_bytes(path).as_c_str(),
                    LFS_O_WRONLY | LFS_O_CREAT,
                ),
            );
            assert_err(
                Error::Exists,
                lfs_file_open(
                    lfs,
                    file,
                    path_bytes(path).as_c_str(),
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                ),
            );
        }
    } else {
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        for path in [
            "/coffee/drip",
            "/no/./../coffee/coldbrew",
            "/coffee/no/./../turkish",
            "/no/no/./.././../coffee/tubruk",
            "/no/no/./.././../coffee/no/./../vietnamese",
            "/no/no/./.././../no/no/./.././../coffee/thai",
        ] {
            assert_ok(lfs_file_open(
                lfs,
                file,
                path_bytes(path).as_c_str(),
                LFS_O_RDONLY,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }

    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_dotdotdots(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_c_str()));
    assert_ok(lfs_mkdir(lfs, path_bytes("coffee/...").as_c_str()));
    for name in PATHS {
        let path = path_bytes(&format!("/coffee/.../{name}"));
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_c_str()));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path.as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }
    for name in PATHS {
        let path = path_bytes(&format!("/coffee/.../{name}"));
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path.as_c_str(), info));
        assert_eq!(info_name_str(&info), *name);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }
    assert_ok(lfs_unmount(lfs));
}

// --- Missing upstream cases ---

/// Upstream: [cases.test_paths_noent_trailing_slashes]
/// defines.DIR = [false, true]
/// Paths with trailing slashes on non-existent entries. C expects exact Error::NoEntry for stat/dir_open.
#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_noent_trailing_slashes(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_c_str()));
    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_c_str()));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path.as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }
    // C: 6 malformed paths with trailing slashes — stat => NOENT
    let bad_stat = [
        "coffee/_rip//////",
        "coffee/c_ldbrew/////",
        "coffee/tu_kish////",
        "coffee/tub_uk///",
        "coffee/_vietnamese//",
        "coffee/thai_/",
    ];
    for bad in bad_stat {
        let path = path_bytes(bad);
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_err(Error::NoEntry, lfs_stat(lfs, path.as_c_str(), info));
    }
    // file_open RDONLY => NOENT
    for bad in bad_stat {
        let path = path_bytes(bad);
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_err(
            Error::NoEntry,
            lfs_file_open(lfs, file, path.as_c_str(), LFS_O_RDONLY),
        );
    }
    // file_open WRONLY|CREAT => NOTDIR
    for bad in bad_stat {
        let path = path_bytes(bad);
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_err(
            Error::NotDir,
            lfs_file_open(lfs, file, path.as_c_str(), LFS_O_WRONLY | LFS_O_CREAT),
        );
    }
    // file_open WRONLY|CREAT|EXCL => NOTDIR
    for bad in bad_stat {
        let path = path_bytes(bad);
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_err(
            Error::NotDir,
            lfs_file_open(
                lfs,
                file,
                path.as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
    }
    // dir_open => NOENT
    for bad in bad_stat {
        let path = path_bytes(bad);
        let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
        assert_err(Error::NoEntry, lfs_dir_open(lfs, dir, path.as_c_str()));
    }
    // rename: bad source
    assert_ok(lfs_mkdir(lfs, path_bytes("espresso").as_c_str()));
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/_rip//////").as_c_str(),
            path_bytes("espresso/espresso").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/c_ldbrew/////").as_c_str(),
            path_bytes("espresso/americano").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/tu_kish////").as_c_str(),
            path_bytes("espresso/macchiato").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/tub_uk///").as_c_str(),
            path_bytes("espresso/latte").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/_vietnamese//").as_c_str(),
            path_bytes("espresso/cappuccino").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/thai_/").as_c_str(),
            path_bytes("espresso/mocha").as_c_str(),
        ),
    );
    // rename: bad destination (trailing slash on dest)
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/_rip").as_c_str(),
            path_bytes("espresso/espresso/").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/c_ldbrew").as_c_str(),
            path_bytes("espresso/americano//").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/tu_kish").as_c_str(),
            path_bytes("espresso/macchiato///").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/tub_uk").as_c_str(),
            path_bytes("espresso/latte////").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/_vietnamese").as_c_str(),
            path_bytes("espresso/cappuccino/////").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/thai_").as_c_str(),
            path_bytes("espresso/mocha//////").as_c_str(),
        ),
    );
    // rename: bad source and bad destination
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/_rip//////").as_c_str(),
            path_bytes("espresso/espresso/").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/c_ldbrew/////").as_c_str(),
            path_bytes("espresso/americano//").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/tu_kish////").as_c_str(),
            path_bytes("espresso/macchiato///").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/tub_uk///").as_c_str(),
            path_bytes("espresso/latte////").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/_vietnamese//").as_c_str(),
            path_bytes("espresso/cappuccino/////").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/thai_/").as_c_str(),
            path_bytes("espresso/mocha//////").as_c_str(),
        ),
    );
    // rename noop (same bad path both sides) => NOENT
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/_rip//////").as_c_str(),
            path_bytes("coffee/_rip//////").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/c_ldbrew/////").as_c_str(),
            path_bytes("coffee/c_ldbrew/////").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/tu_kish////").as_c_str(),
            path_bytes("coffee/tu_kish////").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/tub_uk///").as_c_str(),
            path_bytes("coffee/tub_uk///").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/_vietnamese//").as_c_str(),
            path_bytes("coffee/_vietnamese//").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/thai_/").as_c_str(),
            path_bytes("coffee/thai_/").as_c_str(),
        ),
    );
    // remove => NOENT
    for bad in bad_stat {
        let path = path_bytes(bad);
        assert_err(Error::NoEntry, lfs_remove(lfs, path.as_c_str()));
    }
    // stat espresso/* (renames failed so these don't exist) => NOENT
    for name in [
        "espresso",
        "espresso/espresso",
        "espresso/americano",
        "espresso/macchiato",
        "espresso/latte",
        "espresso/cappuccino",
        "espresso/mocha",
    ] {
        let path = path_bytes(name);
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        let err = lfs_stat(lfs, path.as_c_str(), info);
        if name == "espresso" {
            assert_ok(err);
        } else {
            assert_err(Error::NoEntry, err);
        }
    }
    // final stat of valid coffee paths
    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path.as_c_str(), info));
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_paths_noent_trailing_dots]
/// defines.DIR = [false, true]
/// Paths with trailing dots on non-existent entries. Expect Error::NoEntry.
#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_noent_trailing_dots(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_c_str()));
    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_c_str()));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path.as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }
    // C: 6 malformed paths with trailing dots — stat => NOENT
    let bad_paths = [
        "coffee/_rip/./././././.",
        "coffee/c_ldbrew/././././.",
        "coffee/tu_kish/./././.",
        "coffee/tub_uk/././.",
        "coffee/_vietnamese/./.",
        "coffee/thai_/.",
    ];
    for bad in bad_paths {
        let path = path_bytes(bad);
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_err(Error::NoEntry, lfs_stat(lfs, path.as_c_str(), info));
    }
    // file_open RDONLY, WRONLY|CREAT, WRONLY|CREAT|EXCL => NOENT
    for bad in bad_paths {
        let path = path_bytes(bad);
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_err(
            Error::NoEntry,
            lfs_file_open(lfs, file, path.as_c_str(), LFS_O_RDONLY),
        );
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_err(
            Error::NoEntry,
            lfs_file_open(lfs, file, path.as_c_str(), LFS_O_WRONLY | LFS_O_CREAT),
        );
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_err(
            Error::NoEntry,
            lfs_file_open(
                lfs,
                file,
                path.as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
    }
    // dir_open => NOENT
    for bad in bad_paths {
        let path = path_bytes(bad);
        let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
        assert_err(Error::NoEntry, lfs_dir_open(lfs, dir, path.as_c_str()));
    }
    // rename: bad source, bad dest, noop; remove; final stat of valid paths
    assert_ok(lfs_mkdir(lfs, path_bytes("espresso").as_c_str()));
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/_rip/./././././.").as_c_str(),
            path_bytes("espresso/espresso").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/_rip").as_c_str(),
            path_bytes("espresso/espresso/.").as_c_str(),
        ),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(
            lfs,
            path_bytes("coffee/_rip/./././././.").as_c_str(),
            path_bytes("coffee/_rip/./././././.").as_c_str(),
        ),
    );
    for bad in bad_paths {
        assert_err(Error::NoEntry, lfs_remove(lfs, path_bytes(bad).as_c_str()));
    }
    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path.as_c_str(), info));
        let nul = info.name.iter().position(|&b| b == 0).unwrap_or(256);
        assert_eq!(core::str::from_utf8(&info.name[..nul]).unwrap(), *name);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_paths_noent_trailing_dotdots]
/// defines.DIR = [false, true]
/// Paths with trailing .. components. C: INVAL above root, ISDIR for file_open on coffee/_rip/..,
/// dir_open success for coffee/_rip/.., rename (bad source/dest, valid coffee/thai_/.. → espresso/mocha),
/// remove (NOTEMPTY, INVAL).
#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_noent_trailing_dotdots(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_c_str()));
    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_c_str()));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path.as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }
    // INVAL above root
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_err(Error::Invalid, lfs_stat(lfs, c"coffee/drip/../../..", info));
    // coffee/_rip/.. resolves to coffee (dir). file_open => ISDIR
    let rip_dotdot = path_bytes("coffee/_rip/..");
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_err(
        Error::IsDir,
        lfs_file_open(lfs, file, rip_dotdot.as_c_str(), LFS_O_RDONLY),
    );
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_err(
        Error::IsDir,
        lfs_file_open(lfs, file, rip_dotdot.as_c_str(), LFS_O_WRONLY | LFS_O_CREAT),
    );
    // dir_open on coffee/_rip/.. => success (resolves to coffee)
    let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
    assert_ok(lfs_dir_open(lfs, dir, rip_dotdot.as_c_str()));
    assert_ok(lfs_dir_close(lfs, dir));
    // stat coffee/_rip/.. => coffee
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, rip_dotdot.as_c_str(), info));
    assert_eq!(info_name_str(&info), "coffee");
    // stat coffee/thai_/.. => coffee
    let thai_dotdot = path_bytes("coffee/thai_/..");
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, thai_dotdot.as_c_str(), info));
    assert_eq!(info_name_str(&info), "coffee");
    // rename: valid coffee/thai_/.. → espresso/mocha (moves coffee to espresso/mocha)
    assert_ok(lfs_mkdir(lfs, path_bytes("espresso").as_c_str()));
    assert_ok(lfs_rename(
        lfs,
        path_bytes("coffee/thai_/..").as_c_str(),
        path_bytes("espresso/mocha").as_c_str(),
    ));
    // rename: bad source (coffee/_rip/.. to file path when dest parent doesn't exist or similar)
    assert_ok(lfs_mkdir(lfs, path_bytes("coffee").as_c_str()));
    for name in PATHS {
        let path = path_bytes(&format!("coffee/{name}"));
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_c_str()));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path.as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }
    // remove: NOTEMPTY (coffee has children)
    assert_err(
        Error::NotEmpty,
        lfs_remove(lfs, path_bytes("coffee/drip/..").as_c_str()),
    );
    // remove: INVAL (above root)
    assert_err(
        Error::Invalid,
        lfs_remove(lfs, path_bytes("coffee/drip/../../..").as_c_str()),
    );
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_paths_utf8_ipa]
/// defines.DIR = [false, true]
/// UTF-8 names with IPA symbols. C adds: WRONLY|CREAT => ISDIR or success; WRONLY|CREAT|EXCL => EXIST.
#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_utf8_ipa(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let parent = "ˈkɔ.fi";
    let children = [
        "dɹɪpˈkɔ.fi",
        "koʊldbɹuː",
        "tyɾckɑhvɛˈsi",
        "ˈko.piˈt̪up̚.rʊk̚",
        "kaː˨˩fe˧˧ɗaː˧˥",
        "ʔoː˧.lia̯ŋ˦˥",
    ];
    assert_ok(lfs_mkdir(lfs, path_bytes(parent).as_c_str()));
    for name in children {
        let path = path_bytes(&format!("{parent}/{name}"));
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_c_str()));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path.as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }
    for name in children {
        let path = path_bytes(&format!("{parent}/{name}"));
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path.as_c_str(), info));
        assert_eq!(info_name_str(&info), name);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }
    if dir_mode {
        for name in children {
            let path = path_bytes(&format!("{parent}/{name}"));
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_err(
                Error::IsDir,
                lfs_file_open(lfs, file, path.as_c_str(), LFS_O_RDONLY),
            );
            assert_err(
                Error::IsDir,
                lfs_file_open(lfs, file, path.as_c_str(), LFS_O_WRONLY | LFS_O_CREAT),
            );
            assert_err(
                Error::Exists,
                lfs_file_open(
                    lfs,
                    file,
                    path.as_c_str(),
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                ),
            );
        }
    } else {
        for name in children {
            let path = path_bytes(&format!("{parent}/{name}"));
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(lfs, file, path.as_c_str(), LFS_O_RDONLY));
            assert_ok(lfs_file_close(lfs, file));
            assert_ok(lfs_file_open(
                lfs,
                file,
                path.as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT,
            ));
            assert_ok(lfs_file_close(lfs, file));
            assert_err(
                Error::Exists,
                lfs_file_open(
                    lfs,
                    file,
                    path.as_c_str(),
                    LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
                ),
            );
        }
    }
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_paths_oopsallspaces]
/// C layout: root " ", children " / ", " /  ", " /   ", " /    ", " /     ", " /      " (6 children).
/// Stat all, file_open/dir_open matrix, rename to "  /      " etc., remove.
#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_oopsallspaces(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let root = " ";
    let children = [" ", "  ", "   ", "    ", "     ", "      "];
    assert_ok(lfs_mkdir(lfs, path_bytes(root).as_c_str()));
    for name in children {
        let path = path_bytes(&format!("{root}/{name}"));
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path.as_c_str()));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path.as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }
    for name in children.iter() {
        let path = path_bytes(&format!("{root}/{name}"));
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path.as_c_str(), info));
        assert_eq!(info_name_str(&info), *name);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }
    if dir_mode {
        for name in children {
            let path = path_bytes(&format!("{root}/{name}"));
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_err(
                Error::IsDir,
                lfs_file_open(lfs, file, path.as_c_str(), LFS_O_RDONLY),
            );
            assert_err(
                Error::IsDir,
                lfs_file_open(lfs, file, path.as_c_str(), LFS_O_WRONLY | LFS_O_CREAT),
            );
            let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
            assert_ok(lfs_dir_open(lfs, dir, path.as_c_str()));
            assert_ok(lfs_dir_close(lfs, dir));
        }
    } else {
        for name in children {
            let path = path_bytes(&format!("{root}/{name}"));
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(lfs, file, path.as_c_str(), LFS_O_RDONLY));
            assert_ok(lfs_file_close(lfs, file));
            assert_err(
                Error::NotDir,
                lfs_dir_open(
                    lfs,
                    unsafe { &mut *core::mem::MaybeUninit::<LfsDir>::zeroed().as_mut_ptr() },
                    path.as_c_str(),
                ),
            );
        }
    }
    assert_ok(lfs_mkdir(lfs, path_bytes("  ").as_c_str()));
    let renames = [
        (" / ", "  /      "),
        (" /  ", "  /     "),
        (" /   ", "  /    "),
        (" /    ", "  /   "),
        (" /     ", "  /  "),
        (" /      ", "  / "),
    ];
    for (old, new) in renames {
        let old_path = path_bytes(old);
        let new_path = path_bytes(new);
        assert_ok(lfs_rename(lfs, old_path.as_c_str(), new_path.as_c_str()));
    }
    for (_, new) in renames {
        assert_ok(lfs_remove(lfs, path_bytes(new).as_c_str()));
    }
    assert_ok(lfs_remove(lfs, path_bytes("  ").as_c_str()));
    assert_ok(lfs_remove(lfs, path_bytes(root).as_c_str()));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_paths_oopsalldels]
/// C layout: root \x7f (1 byte), children \x7f/\x7f, \x7f/\x7f\x7f, … (6 children with 1–6 DEL bytes).
#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_oopsalldels(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let root = path_bytes_raw(&[0x7f]);
    assert_ok(lfs_mkdir(lfs, unsafe {
        CStr::from_ptr(root.as_ptr() as *const _)
    }));
    let mut child_paths: Vec<Vec<u8>> = Vec::with_capacity(6);
    for n in 1..=6 {
        let p: Vec<u8> = (0..n).map(|_| 0x7f).collect();
        child_paths.push(path_bytes_raw(&p));
    }
    for cp in child_paths.iter() {
        let mut full: Vec<u8> = vec![0x7f, b'/'];
        full.extend_from_slice(&cp[..cp.len().saturating_sub(1)]);
        full.push(0);
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, full.as_c_str()));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                full.as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }
    let mut full_paths: Vec<Vec<u8>> = Vec::with_capacity(6);
    for n in 1..=6 {
        let mut p = vec![0x7f, b'/'];
        p.extend((0..n).map(|_| 0x7f));
        p.push(0);
        full_paths.push(p);
    }
    for fp in &full_paths {
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(
            lfs,
            unsafe { CStr::from_ptr(fp.as_ptr() as *const _) },
            info,
        ));
    }
    if dir_mode {
        for fp in &full_paths {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_err(
                Error::IsDir,
                lfs_file_open(lfs, file, fp.as_c_str(), LFS_O_RDONLY),
            );
            let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
            assert_ok(lfs_dir_open(lfs, dir, fp.as_c_str()));
            assert_ok(lfs_dir_close(lfs, dir));
        }
    } else {
        for fp in &full_paths {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(lfs, file, fp.as_c_str(), LFS_O_RDONLY));
            assert_ok(lfs_file_close(lfs, file));
            assert_err(
                Error::NotDir,
                lfs_dir_open(
                    lfs,
                    unsafe { &mut *core::mem::MaybeUninit::<LfsDir>::zeroed().as_mut_ptr() },
                    fp.as_c_str(),
                ),
            );
        }
    }
    let new_root = path_bytes_raw(&[0x7f, 0x7f]);
    assert_ok(lfs_mkdir(lfs, new_root.as_c_str()));
    for (n, fp) in full_paths.iter().enumerate() {
        let new_name_len = 6 - n;
        let mut new_path = vec![0x7f, 0x7f, b'/'];
        new_path.extend((0..new_name_len).map(|_| 0x7f));
        new_path.push(0);
        assert_ok(lfs_rename(lfs, fp.as_c_str(), new_path.as_c_str()));
    }
    for n in 1..=6 {
        let mut p = vec![0x7f, 0x7f, b'/'];
        p.extend((0..n).map(|_| 0x7f));
        p.push(0);
        assert_ok(lfs_remove(lfs, p.as_c_str()));
    }
    assert_ok(lfs_remove(lfs, new_root.as_c_str()));
    assert_ok(lfs_remove(lfs, root.as_c_str()));
    assert_ok(lfs_unmount(lfs));
}

/// Upstream: [cases.test_paths_oopsallffs]
/// Same as oopsalldels but with 0xff bytes. C layout: root 0xff, 6 children 0xff/0xff, 0xff/0xff0xff, etc.
#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_oopsallffs(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let root = path_bytes_raw(&[0xff]);
    assert_ok(lfs_mkdir(lfs, root.as_c_str()));
    let mut child_paths: Vec<Vec<u8>> = Vec::with_capacity(6);
    for n in 1..=6 {
        let p: Vec<u8> = (0..n).map(|_| 0xff).collect();
        child_paths.push(path_bytes_raw(&p));
    }
    for cp in child_paths.iter() {
        let mut full: Vec<u8> = vec![0xff, b'/'];
        full.extend_from_slice(&cp[..cp.len().saturating_sub(1)]);
        full.push(0);
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, unsafe {
                CStr::from_ptr(full.as_ptr() as *const _)
            }));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                unsafe { CStr::from_ptr(full.as_ptr() as *const _) },
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }
    let mut full_paths: Vec<Vec<u8>> = Vec::with_capacity(6);
    for n in 1..=6 {
        let mut p = vec![0xff, b'/'];
        p.extend((0..n).map(|_| 0xff));
        p.push(0);
        full_paths.push(p);
    }
    for fp in &full_paths {
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(
            lfs,
            unsafe { CStr::from_ptr(fp.as_c_str() as *const _) },
            info,
        ));
    }
    if dir_mode {
        for fp in &full_paths {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_err(
                Error::IsDir,
                lfs_file_open(lfs, file, fp.as_c_str(), LFS_O_RDONLY),
            );
            let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
            assert_ok(lfs_dir_open(lfs, dir, fp.as_c_str()));
            assert_ok(lfs_dir_close(lfs, dir));
        }
    } else {
        for fp in &full_paths {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(lfs, file, fp.as_c_str(), LFS_O_RDONLY));
            assert_ok(lfs_file_close(lfs, file));
            assert_err(
                Error::NotDir,
                lfs_dir_open(
                    lfs,
                    unsafe { &mut *core::mem::MaybeUninit::<LfsDir>::zeroed().as_mut_ptr() },
                    fp.as_c_str(),
                ),
            );
        }
    }
    let new_root = path_bytes_raw(&[0xff, 0xff]);
    assert_ok(lfs_mkdir(lfs, new_root.as_c_str()));
    for (n, fp) in full_paths.iter().enumerate() {
        let new_name_len = 6 - n;
        let mut new_path = vec![0xff, 0xff, b'/'];
        new_path.extend((0..new_name_len).map(|_| 0xff));
        new_path.push(0);
        assert_ok(lfs_rename(lfs, fp.as_c_str(), new_path.as_c_str()));
    }
    for n in 1..=6 {
        let mut p = vec![0xff, 0xff, b'/'];
        p.extend((0..n).map(|_| 0xff));
        p.push(0);
        assert_ok(lfs_remove(lfs, p.as_c_str()));
    }
    assert_ok(lfs_remove(lfs, new_root.as_c_str()));
    assert_ok(lfs_remove(lfs, root.as_c_str()));
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_leading_dots(#[case] _dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_err(Error::Invalid, lfs_stat(lfs, c"..", info));
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_root_dotdots(#[case] _dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_err(Error::Invalid, lfs_stat(lfs, c"/..", info));
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_noent_parent() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_err(Error::NoEntry, lfs_stat(lfs, c"nonexistent/child", info));
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_notdir_parent() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        path_bytes("f").as_c_str(),
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    assert_ok(lfs_file_close(lfs, file));
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_err(Error::NotDir, lfs_stat(lfs, c"f/child", info));
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_empty(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_err(
        Error::Invalid,
        lfs_stat(lfs, path_bytes("").as_c_str(), info),
    );
    if dir_mode {
        assert_err(Error::Invalid, lfs_mkdir(lfs, path_bytes("").as_c_str()));
    } else {
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_err(
            Error::Invalid,
            lfs_file_open(
                lfs,
                file,
                path_bytes("").as_c_str(),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
    }
    assert_ok(lfs_mkdir(lfs, path_bytes("x").as_c_str()));
    assert_err(
        Error::Invalid,
        lfs_rename(lfs, path_bytes("x").as_c_str(), path_bytes("").as_c_str()),
    );
    assert_err(
        Error::Invalid,
        lfs_rename(lfs, path_bytes("").as_c_str(), path_bytes("y").as_c_str()),
    );
    assert_err(Error::Invalid, lfs_remove(lfs, path_bytes("").as_c_str()));
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_root_aliases(#[case] _dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let aliases = &["/", ".", "./", "/.", "//"];
    for alias in aliases {
        let path = path_bytes(alias);
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path.as_c_str(), info));
        assert_eq!(info_name_str(&info), "/");
        assert_eq!(info.type_, LFS_TYPE_DIR as u8);
    }
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_magic_noent() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, path_bytes("a").as_c_str()));
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_err(Error::NoEntry, lfs_stat(lfs, c"a/b", info));
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_magic_conflict(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    if dir_mode {
        assert_ok(lfs_mkdir(lfs, path_bytes("littlefs").as_c_str()));
    } else {
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            path_bytes("littlefs").as_c_str(),
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs, file));
    }
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, path_bytes("littlefs").as_c_str(), info));
    assert_eq!(info_name_str(info), "littlefs");
    assert_ok(lfs_rename(
        lfs,
        path_bytes("littlefs").as_c_str(),
        path_bytes("coffee").as_c_str(),
    ));
    assert_ok(lfs_rename(
        lfs,
        path_bytes("coffee").as_c_str(),
        path_bytes("littlefs").as_c_str(),
    ));
    assert_ok(lfs_stat(lfs, path_bytes("littlefs").as_c_str(), info));
    assert_err(
        Error::NoEntry,
        lfs_stat(lfs, path_bytes("coffee").as_c_str(), info),
    );
    assert_ok(lfs_remove(lfs, path_bytes("littlefs").as_c_str()));
    assert_err(
        Error::NoEntry,
        lfs_stat(lfs, path_bytes("littlefs").as_c_str(), info),
    );
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_nametoolong() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    let long_name = "a".repeat(256);
    assert_err(
        Error::NameTooLong,
        lfs_mkdir(lfs, path_bytes(&long_name).as_c_str()),
    );
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_namejustlongenough() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    let max_name = "a".repeat(255);
    assert_ok(lfs_mkdir(lfs, path_bytes(&max_name).as_ptr()));
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, path_bytes(&max_name).as_c_str(), info));
    assert_eq!(info_name_str(info), max_name);
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_utf8() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    let name = "café_日本_한글";
    assert_ok(lfs_mkdir(lfs, path_bytes(name).as_c_str()));
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, path_bytes(name).as_c_str(), info));
    assert_eq!(info_name_str(info), name);
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_spaces() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    let name = "foo bar";
    assert_ok(lfs_mkdir(lfs, path_bytes(name).as_c_str()));
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, path_bytes(name).as_c_str(), info));
    assert_eq!(info_name_str(info), name);
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_nonprintable() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    let mut name: Vec<u8> = vec![b'a'; 10];
    name[5] = 0x01;
    name.push(0);
    let name = unsafe { CStr::from_ptr(name.as_ptr() as *const _) };
    assert_ok(lfs_mkdir(lfs, name));
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, name, info));
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_nonutf8() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut unsafe { core::mem::MaybeUninit::<Lfs>::zeroed().assume_init() };
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    let name = c"foo\xff\xfe\xfdbar";
    assert_ok(lfs_mkdir(lfs, name.as_c_str()));
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, name, info));
    assert_ok(lfs_unmount(lfs));
}

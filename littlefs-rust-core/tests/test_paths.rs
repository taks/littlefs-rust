//! Path resolution integration tests.
//!
//! Upstream: tests/test_paths.toml
//! Source: https://github.com/littlefs-project/littlefs/blob/master/tests/test_paths.toml

mod common;

use common::{assert_err, assert_ok, default_config, init_context, init_logger};
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

    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let coffee = "coffee";
    assert_ok(lfs_mkdir(lfs, coffee));

    for name in PATHS {
        let path = &format!("coffee/{name}");
        assert_ok(lfs_mkdir(lfs, path));
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path, info));
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

    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let coffee = "coffee";
    assert_ok(lfs_mkdir(lfs, coffee));

    for name in PATHS {
        let path = &format!("coffee/{name}");
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs, file));
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path, info));
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

    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let coffee = "coffee";
    assert_ok(lfs_mkdir(lfs, coffee));

    for name in PATHS {
        let path = &format!("/coffee/{name}");
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            path,
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs, file));
    }
    for name in PATHS {
        let path = &format!("/coffee/{name}");
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path, info));
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

    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let coffee = "coffee";
    assert_ok(lfs_mkdir(lfs, coffee));

    for name in PATHS {
        let path = &format!("/coffee/{name}");
        assert_ok(lfs_mkdir(lfs, path));
    }
    for name in PATHS {
        let path = &format!("/coffee/{name}");
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path, info));
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

    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let coffee = "coffee";
    assert_ok(lfs_mkdir(lfs, coffee));
    for name in PATHS {
        let path = &format!("coffee/{name}");
        assert_ok(lfs_mkdir(lfs, path));
    }

    for bad in &[
        "_rip",
        "c_ldbrew",
        "tu_kish",
        "tub_uk",
        "_vietnamese",
        "thai_",
    ] {
        let path = &format!("coffee/{bad}");
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        let err = lfs_stat(lfs, path, info);
        assert_err(Error::NoEntry, err);

        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        let err = lfs_file_open(lfs, file, path, LFS_O_RDONLY);
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

    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let root_path = "/";
    let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
    assert_ok(lfs_dir_open(lfs, dir, root_path));
    assert_ok(lfs_dir_close(lfs, dir));

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, root_path, info));
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
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, "coffee"));
    let create_paths = &[
        "/coffee/drip",
        "//coffee//coldbrew",
        "///coffee///turkish",
        "////coffee////tubruk",
        "/////coffee/////vietnamese",
        "//////coffee//////thai",
    ];
    for path_str in create_paths {
        let path = path_str;
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path,
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
        let path = path_str;
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path, info));
        assert_eq!(info_name_str(info), expect);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }

    assert_ok(lfs_mkdir(lfs, "espresso"));
    let renames = &[
        ("//////coffee//////drip", "/espresso/espresso"),
        ("/////coffee/////coldbrew", "//espresso//americano"),
        ("////coffee////turkish", "///espresso///macchiato"),
        ("///coffee///tubruk", "////espresso////latte"),
        ("//coffee//vietnamese", "/////espresso/////cappuccino"),
        ("/coffee/thai", "//////espresso//////mocha"),
    ];
    for (old, new) in renames {
        assert_ok(lfs_rename(lfs, old, new));
    }
    let info_dummy = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    for path_str in stat_paths {
        let path = path_str;
        assert_err(Error::NoEntry, lfs_stat(lfs, path, info_dummy));
    }
    for path_str in &[
        "/espresso/espresso",
        "//espresso//americano",
        "///espresso///macchiato",
        "////espresso////latte",
        "/////espresso/////cappuccino",
        "/espresso/mocha",
    ] {
        assert_ok(lfs_remove(lfs, path_str));
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
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, "coffee"));
    if dir_mode {
        for s in &[
            "coffee/drip/",
            "coffee/coldbrew//",
            "coffee/turkish///",
            "coffee/tubruk////",
            "coffee/vietnamese/////",
            "coffee/thai//////",
        ] {
            assert_ok(lfs_mkdir(lfs, s));
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
                lfs_file_open(lfs, file, s, LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL),
            );
        }
        for name in PATHS {
            let path = &format!("coffee/{name}");
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path,
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
        let path = path_str;
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        let err = lfs_stat(lfs, path, info);
        if dir_mode {
            assert_ok(err);
            assert_eq!(info_name_str(info), PATHS[i]);
            assert_eq!(info.type_, LFS_TYPE_DIR as u8);
        } else {
            assert_err(Error::NotDir, err);
        }
    }

    assert_ok(lfs_mkdir(lfs, "espresso"));
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
            assert_ok(lfs_rename(lfs, old, new));
        }
        for s in &[
            "espresso/espresso/",
            "espresso/americano//",
            "espresso/macchiato///",
            "espresso/latte////",
            "espresso/cappuccino/////",
            "espresso/mocha//////",
        ] {
            assert_ok(lfs_remove(lfs, s));
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
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, "coffee"));
    let create_paths = &[
        "/coffee/drip",
        "/./coffee/./coldbrew",
        "/././coffee/././turkish",
        "/./././coffee/./././tubruk",
        "/././././coffee/././././vietnamese",
        "/./././././coffee/./././././thai",
    ];
    for path_str in create_paths {
        let path = path_str;
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path,
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
        let path = path_str;
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path, info));
        assert_eq!(info_name_str(info), expect);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }

    assert_ok(lfs_mkdir(lfs, "espresso"));
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
        assert_ok(lfs_rename(lfs, old, new));
    }
    for s in &[
        "/espresso/espresso",
        "/./espresso/./americano",
        "/././espresso/././macchiato",
        "/./././espresso/./././latte",
        "/././././espresso/././././cappuccino",
        "/./././././espresso/./././././mocha",
    ] {
        assert_ok(lfs_remove(lfs, s));
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
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, "coffee"));
    if dir_mode {
        for s in &[
            "coffee/drip/.",
            "coffee/coldbrew/./.",
            "coffee/turkish/././.",
            "coffee/tubruk/./././.",
            "coffee/vietnamese/././././.",
            "coffee/thai/./././././.",
        ] {
            assert_err(Error::NoEntry, lfs_mkdir(lfs, s));
        }
        for name in PATHS {
            assert_ok(lfs_mkdir(lfs, &format!("coffee/{name}")));
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
                lfs_file_open(lfs, file, s, LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL),
            );
        }
        for name in PATHS {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                &format!("coffee/{name}"),
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
        let path = path_str;
        let err = lfs_stat(lfs, path, info);
        if dir_mode {
            assert_ok(err);
            assert_eq!(info_name_str(info), PATHS[i]);
            assert_eq!(info.type_, LFS_TYPE_DIR as u8);
        } else {
            assert_err(Error::NotDir, err);
        }
    }

    assert_ok(lfs_mkdir(lfs, "espresso"));
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
            assert_ok(lfs_rename(lfs, old, new));
        }
        for s in &[
            "espresso/espresso/.",
            "espresso/americano/./.",
            "espresso/macchiato/././.",
            "espresso/latte/./././.",
            "espresso/cappuccino/././././.",
            "espresso/mocha/./././././.",
        ] {
            assert_ok(lfs_remove(lfs, s));
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
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, "no"));
    assert_ok(lfs_mkdir(lfs, "no/no"));
    assert_ok(lfs_mkdir(lfs, "coffee"));
    assert_ok(lfs_mkdir(lfs, "coffee/no"));
    let create_paths = &[
        "/coffee/drip",
        "/no/../coffee/coldbrew",
        "/coffee/no/../turkish",
        "/no/no/../../coffee/tubruk",
        "/no/no/../../coffee/no/../vietnamese",
        "/no/no/../../no/no/../../coffee/thai",
    ];
    for path_str in create_paths {
        let path = path_str;
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path,
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
        let path = path_str;
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path, info));
        assert_eq!(info_name_str(info), expect);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }

    assert_ok(lfs_mkdir(lfs, "espresso"));
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
        assert_ok(lfs_rename(lfs, old, new));
    }
    for s in &[
        "/espresso/espresso",
        "/./espresso/./americano",
        "/././espresso/././macchiato",
        "/./././espresso/./././latte",
        "/././././espresso/././././cappuccino",
        "/./././././espresso/./././././mocha",
    ] {
        assert_ok(lfs_remove(lfs, s));
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
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, "coffee"));

    if dir_mode {
        assert_err(Error::Exists, lfs_mkdir(lfs, "coffee/drip/.."));
        assert_err(Error::Exists, lfs_mkdir(lfs, "coffee/coldbrew/../.."));
        assert_err(Error::Invalid, lfs_mkdir(lfs, "coffee/turkish/../../.."));
        assert_err(Error::Invalid, lfs_mkdir(lfs, "coffee/tubruk/../../../.."));
        assert_err(
            Error::Invalid,
            lfs_mkdir(lfs, "coffee/vietnamese/../../../../.."),
        );
        assert_err(
            Error::Invalid,
            lfs_mkdir(lfs, "coffee/thai/../../../../../.."),
        );
        for name in PATHS {
            assert_ok(lfs_mkdir(lfs, &format!("coffee/{name}")));
        }
    } else {
        assert_err(
            Error::Exists,
            lfs_file_open(
                lfs,
                unsafe { &mut *core::mem::MaybeUninit::<LfsFile>::zeroed().as_mut_ptr() },
                "coffee/drip/..",
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        assert_err(
            Error::Exists,
            lfs_file_open(
                lfs,
                unsafe { &mut *core::mem::MaybeUninit::<LfsFile>::zeroed().as_mut_ptr() },
                "coffee/coldbrew/../..",
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        assert_err(
            Error::Invalid,
            lfs_file_open(
                lfs,
                unsafe { &mut *core::mem::MaybeUninit::<LfsFile>::zeroed().as_mut_ptr() },
                "coffee/turkish/../../..",
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        assert_err(
            Error::Invalid,
            lfs_file_open(
                lfs,
                unsafe { &mut *core::mem::MaybeUninit::<LfsFile>::zeroed().as_mut_ptr() },
                "coffee/tubruk/../../../..",
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        assert_err(
            Error::Invalid,
            lfs_file_open(
                lfs,
                unsafe { &mut *core::mem::MaybeUninit::<LfsFile>::zeroed().as_mut_ptr() },
                "coffee/vietnamese/../../../../..",
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        assert_err(
            Error::Invalid,
            lfs_file_open(
                lfs,
                unsafe { &mut *core::mem::MaybeUninit::<LfsFile>::zeroed().as_mut_ptr() },
                "coffee/thai/../../../../../..",
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ),
        );
        for name in PATHS {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                &format!("coffee/{name}"),
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }

    // stat paths
    let info_err = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    assert_err(
        Error::Invalid,
        lfs_stat(lfs, "coffee/drip/../../../../../..", info_err),
    );
    assert_err(
        Error::Invalid,
        lfs_stat(lfs, "coffee/coldbrew/../../../../..", info_err),
    );
    assert_err(
        Error::Invalid,
        lfs_stat(lfs, "coffee/turkish/../../../..", info_err),
    );
    assert_err(
        Error::Invalid,
        lfs_stat(lfs, "coffee/tubruk/../../..", info_err),
    );

    let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    assert_ok(lfs_stat(lfs, "coffee/vietnamese/../..", info));
    assert_eq!(info_name_str(info), "/");
    assert_eq!(info.type_, LFS_TYPE_DIR as u8);

    let info2 = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    assert_ok(lfs_stat(lfs, "coffee/thai/..", info2));
    assert_eq!(info_name_str(info2), "coffee");
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
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, "no"));
    assert_ok(lfs_mkdir(lfs, "no/no"));
    assert_ok(lfs_mkdir(lfs, "coffee"));
    assert_ok(lfs_mkdir(lfs, "coffee/no"));

    if dir_mode {
        assert_ok(lfs_mkdir(lfs, "/coffee/drip"));
        assert_ok(lfs_mkdir(lfs, "/no/./../coffee/coldbrew"));
        assert_ok(lfs_mkdir(lfs, "/coffee/no/./../turkish"));
        assert_ok(lfs_mkdir(lfs, "/no/no/./.././../coffee/tubruk"));
        assert_ok(lfs_mkdir(lfs, "/no/no/./.././../coffee/no/./../vietnamese"));
        assert_ok(lfs_mkdir(
            lfs,
            "/no/no/./.././../no/no/./.././../coffee/thai",
        ));
    } else {
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            "/coffee/drip",
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs, file));
        assert_ok(lfs_file_open(
            lfs,
            file,
            "/no/./../coffee/coldbrew",
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
                path,
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, &mut f));
        }
    }

    // stat paths
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(
        lfs,
        "/no/no/./.././../no/no/./.././../coffee/drip",
        info,
    ));
    assert_eq!(info_name_str(info), "drip");
    assert_eq!(
        info.type_,
        if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
    );

    let info2 = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
    assert_ok(lfs_stat(
        lfs,
        "/no/no/./.././../coffee/no/./../coldbrew",
        info2,
    ));
    assert_eq!(info_name_str(info2), "coldbrew");
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
        let info = &mut unsafe { core::mem::zeroed::<LfsInfo>() };
        assert_ok(lfs_stat(lfs, path, info));
        assert_eq!(info_name_str(info), expected_name);
        assert_eq!(
            info.type_,
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
            assert_err(Error::IsDir, lfs_file_open(lfs, file, path, LFS_O_RDONLY));
            assert_err(
                Error::IsDir,
                lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT),
            );
            assert_err(
                Error::Exists,
                lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL),
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
            assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
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
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, "coffee"));
    assert_ok(lfs_mkdir(lfs, "coffee/..."));
    for name in PATHS {
        let path = &format!("/coffee/.../{name}");
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path,
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }
    for name in PATHS {
        let path = &format!("/coffee/.../{name}");
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path, info));
        assert_eq!(info_name_str(info), *name);
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
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, "coffee"));
    for name in PATHS {
        let path = &format!("coffee/{name}");
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path,
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
        let path = bad;
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_err(Error::NoEntry, lfs_stat(lfs, path, info));
    }
    // file_open RDONLY => NOENT
    for bad in bad_stat {
        let path = bad;
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_err(Error::NoEntry, lfs_file_open(lfs, file, path, LFS_O_RDONLY));
    }
    // file_open WRONLY|CREAT => NOTDIR
    for bad in bad_stat {
        let path = bad;
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_err(
            Error::NotDir,
            lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT),
        );
    }
    // file_open WRONLY|CREAT|EXCL => NOTDIR
    for bad in bad_stat {
        let path = bad;
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_err(
            Error::NotDir,
            lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL),
        );
    }
    // dir_open => NOENT
    for bad in bad_stat {
        let path = bad;
        let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
        assert_err(Error::NoEntry, lfs_dir_open(lfs, dir, path));
    }
    // rename: bad source
    assert_ok(lfs_mkdir(lfs, "espresso"));
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/_rip//////", "espresso/espresso"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/c_ldbrew/////", "espresso/americano"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/tu_kish////", "espresso/macchiato"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/tub_uk///", "espresso/latte"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/_vietnamese//", "espresso/cappuccino"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/thai_/", "espresso/mocha"),
    );
    // rename: bad destination (trailing slash on dest)
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/_rip", "espresso/espresso/"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/c_ldbrew", "espresso/americano//"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/tu_kish", "espresso/macchiato///"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/tub_uk", "espresso/latte////"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/_vietnamese", "espresso/cappuccino/////"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/thai_", "espresso/mocha//////"),
    );
    // rename: bad source and bad destination
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/_rip//////", "espresso/espresso/"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/c_ldbrew/////", "espresso/americano//"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/tu_kish////", "espresso/macchiato///"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/tub_uk///", "espresso/latte////"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/_vietnamese//", "espresso/cappuccino/////"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/thai_/", "espresso/mocha//////"),
    );
    // rename noop (same bad path both sides) => NOENT
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/_rip//////", "coffee/_rip//////"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/c_ldbrew/////", "coffee/c_ldbrew/////"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/tu_kish////", "coffee/tu_kish////"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/tub_uk///", "coffee/tub_uk///"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/_vietnamese//", "coffee/_vietnamese//"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/thai_/", "coffee/thai_/"),
    );
    // remove => NOENT
    for bad in bad_stat {
        let path = bad;
        assert_err(Error::NoEntry, lfs_remove(lfs, path));
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
        let path = name;
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        let err = lfs_stat(lfs, path, info);
        if name == "espresso" {
            assert_ok(err);
        } else {
            assert_err(Error::NoEntry, err);
        }
    }
    // final stat of valid coffee paths
    for name in PATHS {
        let path = &format!("coffee/{name}");
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path, info));
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
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, "coffee"));
    for name in PATHS {
        let path = &format!("coffee/{name}");
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path,
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
        let path = bad;
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_err(Error::NoEntry, lfs_stat(lfs, path, info));
    }
    // file_open RDONLY, WRONLY|CREAT, WRONLY|CREAT|EXCL => NOENT
    for bad in bad_paths {
        let path = bad;
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_err(Error::NoEntry, lfs_file_open(lfs, file, path, LFS_O_RDONLY));
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_err(
            Error::NoEntry,
            lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT),
        );
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_err(
            Error::NoEntry,
            lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL),
        );
    }
    // dir_open => NOENT
    for bad in bad_paths {
        let path = bad;
        let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
        assert_err(Error::NoEntry, lfs_dir_open(lfs, dir, path));
    }
    // rename: bad source, bad dest, noop; remove; final stat of valid paths
    assert_ok(lfs_mkdir(lfs, "espresso"));
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/_rip/./././././.", "espresso/espresso"),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/_rip", "espresso/espresso/."),
    );
    assert_err(
        Error::NoEntry,
        lfs_rename(lfs, "coffee/_rip/./././././.", "coffee/_rip/./././././."),
    );
    for bad in bad_paths {
        assert_err(Error::NoEntry, lfs_remove(lfs, bad));
    }
    for name in PATHS {
        let path = &format!("coffee/{name}");
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path, info));
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
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, "coffee"));
    for name in PATHS {
        let path = &format!("coffee/{name}");
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path,
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }
    // INVAL above root
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_err(Error::Invalid, lfs_stat(lfs, "coffee/drip/../../..", info));
    // coffee/_rip/.. resolves to coffee (dir). file_open => ISDIR
    let rip_dotdot = "coffee/_rip/..";
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_err(
        Error::IsDir,
        lfs_file_open(lfs, file, rip_dotdot, LFS_O_RDONLY),
    );
    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_err(
        Error::IsDir,
        lfs_file_open(lfs, file, rip_dotdot, LFS_O_WRONLY | LFS_O_CREAT),
    );
    // dir_open on coffee/_rip/.. => success (resolves to coffee)
    let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
    assert_ok(lfs_dir_open(lfs, dir, rip_dotdot));
    assert_ok(lfs_dir_close(lfs, dir));
    // stat coffee/_rip/.. => coffee
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, rip_dotdot, info));
    assert_eq!(info_name_str(info), "coffee");
    // stat coffee/thai_/.. => coffee
    let thai_dotdot = "coffee/thai_/..";
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, thai_dotdot, info));
    assert_eq!(info_name_str(info), "coffee");
    // rename: valid coffee/thai_/.. → espresso/mocha (moves coffee to espresso/mocha)
    assert_ok(lfs_mkdir(lfs, "espresso"));
    assert_ok(lfs_rename(lfs, "coffee/thai_/..", "espresso/mocha"));
    // rename: bad source (coffee/_rip/.. to file path when dest parent doesn't exist or similar)
    assert_ok(lfs_mkdir(lfs, "coffee"));
    for name in PATHS {
        let path = &format!("coffee/{name}");
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path,
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }
    // remove: NOTEMPTY (coffee has children)
    assert_err(Error::NotEmpty, lfs_remove(lfs, "coffee/drip/.."));
    // remove: INVAL (above root)
    assert_err(Error::Invalid, lfs_remove(lfs, "coffee/drip/../../.."));
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
    let lfs = &mut Lfs::default();
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
    assert_ok(lfs_mkdir(lfs, parent));
    for name in children {
        let path = &format!("{parent}/{name}");
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path,
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }
    for name in children {
        let path = &format!("{parent}/{name}");
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path, info));
        assert_eq!(info_name_str(info), name);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }
    if dir_mode {
        for name in children {
            let path = &format!("{parent}/{name}");
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_err(Error::IsDir, lfs_file_open(lfs, file, path, LFS_O_RDONLY));
            assert_err(
                Error::IsDir,
                lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT),
            );
            assert_err(
                Error::Exists,
                lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL),
            );
        }
    } else {
        for name in children {
            let path = &format!("{parent}/{name}");
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
            assert_ok(lfs_file_close(lfs, file));
            assert_ok(lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT));
            assert_ok(lfs_file_close(lfs, file));
            assert_err(
                Error::Exists,
                lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL),
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
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let root = " ";
    let children = [" ", "  ", "   ", "    ", "     ", "      "];
    assert_ok(lfs_mkdir(lfs, root));
    for name in children {
        let path = &format!("{root}/{name}");
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, path));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                path,
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }
    for name in children.iter() {
        let path = &format!("{root}/{name}");
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path, info));
        assert_eq!(info_name_str(info), *name);
        assert_eq!(
            info.type_,
            if dir_mode { LFS_TYPE_DIR } else { LFS_TYPE_REG } as u8
        );
    }
    if dir_mode {
        for name in children {
            let path = &format!("{root}/{name}");
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_err(Error::IsDir, lfs_file_open(lfs, file, path, LFS_O_RDONLY));
            assert_err(
                Error::IsDir,
                lfs_file_open(lfs, file, path, LFS_O_WRONLY | LFS_O_CREAT),
            );
            let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
            assert_ok(lfs_dir_open(lfs, dir, path));
            assert_ok(lfs_dir_close(lfs, dir));
        }
    } else {
        for name in children {
            let path = &format!("{root}/{name}");
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(lfs, file, path, LFS_O_RDONLY));
            assert_ok(lfs_file_close(lfs, file));
            assert_err(
                Error::NotDir,
                lfs_dir_open(
                    lfs,
                    unsafe { &mut *core::mem::MaybeUninit::<LfsDir>::zeroed().as_mut_ptr() },
                    path,
                ),
            );
        }
    }
    assert_ok(lfs_mkdir(lfs, "  "));
    let renames = [
        (" / ", "  /      "),
        (" /  ", "  /     "),
        (" /   ", "  /    "),
        (" /    ", "  /   "),
        (" /     ", "  /  "),
        (" /      ", "  / "),
    ];
    for (old, new) in renames {
        let old_path = old;
        let new_path = new;
        assert_ok(lfs_rename(lfs, old_path, new_path));
    }
    for (_, new) in renames {
        assert_ok(lfs_remove(lfs, new));
    }
    assert_ok(lfs_remove(lfs, "  "));
    assert_ok(lfs_remove(lfs, root));
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
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let root = &[0x7f];
    assert_ok(lfs_mkdir(lfs, unsafe { str::from_utf8_unchecked(root) }));
    let mut child_paths: Vec<Vec<u8>> = Vec::with_capacity(6);
    for n in 1..=6 {
        let p: Vec<u8> = (0..n).map(|_| 0x7f).collect();
        child_paths.push(p);
    }
    for cp in child_paths.iter() {
        let mut full: Vec<u8> = vec![0x7f, b'/'];
        full.extend_from_slice(cp);
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, unsafe { str::from_utf8_unchecked(&full) }));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                unsafe { str::from_utf8_unchecked(&full) },
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }
    let mut full_paths: Vec<Vec<u8>> = Vec::with_capacity(6);
    for n in 1..=6 {
        let mut p = vec![0x7f, b'/'];
        p.extend((0..n).map(|_| 0x7f));
        full_paths.push(p);
    }
    for fp in &full_paths {
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, unsafe { str::from_utf8_unchecked(fp) }, info));
    }
    if dir_mode {
        for fp in &full_paths {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_err(
                Error::IsDir,
                lfs_file_open(
                    lfs,
                    file,
                    unsafe { str::from_utf8_unchecked(fp) },
                    LFS_O_RDONLY,
                ),
            );
            let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
            assert_ok(lfs_dir_open(lfs, dir, unsafe {
                str::from_utf8_unchecked(fp)
            }));
            assert_ok(lfs_dir_close(lfs, dir));
        }
    } else {
        for fp in &full_paths {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                unsafe { str::from_utf8_unchecked(fp) },
                LFS_O_RDONLY,
            ));
            assert_ok(lfs_file_close(lfs, file));
            assert_err(
                Error::NotDir,
                lfs_dir_open(
                    lfs,
                    unsafe { &mut *core::mem::MaybeUninit::<LfsDir>::zeroed().as_mut_ptr() },
                    unsafe { str::from_utf8_unchecked(fp) },
                ),
            );
        }
    }
    let new_root = &[0x7f, 0x7f];
    assert_ok(lfs_mkdir(lfs, unsafe {
        str::from_utf8_unchecked(new_root)
    }));
    for (n, fp) in full_paths.iter().enumerate() {
        let new_name_len = 6 - n;
        let mut new_path = vec![0x7f, 0x7f, b'/'];
        new_path.extend((0..new_name_len).map(|_| 0x7f));
        assert_ok(lfs_rename(
            lfs,
            unsafe { str::from_utf8_unchecked(fp) },
            unsafe { str::from_utf8_unchecked(&new_path) },
        ));
    }
    for n in 1..=6 {
        let mut p = vec![0x7f, 0x7f, b'/'];
        p.extend((0..n).map(|_| 0x7f));
        assert_ok(lfs_remove(lfs, unsafe { str::from_utf8_unchecked(&p) }));
    }
    assert_ok(lfs_remove(lfs, unsafe {
        str::from_utf8_unchecked(new_root)
    }));
    assert_ok(lfs_remove(lfs, unsafe { str::from_utf8_unchecked(root) }));
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
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let root = &[0xff];
    #[allow(invalid_from_utf8_unchecked)]
    assert_ok(lfs_mkdir(lfs, unsafe { str::from_utf8_unchecked(root) }));
    let mut child_paths: Vec<Vec<u8>> = Vec::with_capacity(6);
    for n in 1..=6 {
        let p: Vec<u8> = (0..n).map(|_| 0xff).collect();
        child_paths.push(p);
    }
    for cp in child_paths.iter() {
        let mut full: Vec<u8> = vec![0xff, b'/'];
        full.extend_from_slice(cp);
        if dir_mode {
            assert_ok(lfs_mkdir(lfs, unsafe { str::from_utf8_unchecked(&full) }));
        } else {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                unsafe { str::from_utf8_unchecked(&full) },
                LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
            ));
            assert_ok(lfs_file_close(lfs, file));
        }
    }
    let mut full_paths: Vec<Vec<u8>> = Vec::with_capacity(6);
    for n in 1..=6 {
        let mut p = vec![0xff, b'/'];
        p.extend((0..n).map(|_| 0xff));
        full_paths.push(p);
    }
    for fp in &full_paths {
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, unsafe { str::from_utf8_unchecked(fp) }, info));
    }
    if dir_mode {
        for fp in &full_paths {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_err(
                Error::IsDir,
                lfs_file_open(
                    lfs,
                    file,
                    unsafe { str::from_utf8_unchecked(fp) },
                    LFS_O_RDONLY,
                ),
            );
            let dir = &mut unsafe { core::mem::MaybeUninit::<LfsDir>::zeroed().assume_init() };
            assert_ok(lfs_dir_open(lfs, dir, unsafe {
                str::from_utf8_unchecked(fp)
            }));
            assert_ok(lfs_dir_close(lfs, dir));
        }
    } else {
        for fp in &full_paths {
            let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
            assert_ok(lfs_file_open(
                lfs,
                file,
                unsafe { str::from_utf8_unchecked(fp) },
                LFS_O_RDONLY,
            ));
            assert_ok(lfs_file_close(lfs, file));
            assert_err(
                Error::NotDir,
                lfs_dir_open(
                    lfs,
                    unsafe { &mut *core::mem::MaybeUninit::<LfsDir>::zeroed().as_mut_ptr() },
                    unsafe { str::from_utf8_unchecked(fp) },
                ),
            );
        }
    }
    let new_root = &[0xff, 0xff];
    #[allow(invalid_from_utf8_unchecked)]
    assert_ok(lfs_mkdir(lfs, unsafe {
        str::from_utf8_unchecked(new_root)
    }));
    for (n, fp) in full_paths.iter().enumerate() {
        let new_name_len = 6 - n;
        let mut new_path = vec![0xff, 0xff, b'/'];
        new_path.extend((0..new_name_len).map(|_| 0xff));
        assert_ok(lfs_rename(
            lfs,
            unsafe { str::from_utf8_unchecked(fp) },
            unsafe { str::from_utf8_unchecked(&new_path) },
        ));
    }
    for n in 1..=6 {
        let mut p = vec![0xff, 0xff, b'/'];
        p.extend((0..n).map(|_| 0xff));
        assert_ok(lfs_remove(lfs, unsafe { str::from_utf8_unchecked(&p) }));
    }
    assert_ok(lfs_remove(lfs, unsafe {
        #[allow(invalid_from_utf8_unchecked)]
        str::from_utf8_unchecked(new_root)
    }));
    assert_ok(lfs_remove(lfs, unsafe {
        #[allow(invalid_from_utf8_unchecked)]
        str::from_utf8_unchecked(root)
    }));
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_leading_dots(#[case] _dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_err(Error::Invalid, lfs_stat(lfs, "..", info));
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_root_dotdots(#[case] _dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_err(Error::Invalid, lfs_stat(lfs, "/..", info));
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_noent_parent() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_err(Error::NoEntry, lfs_stat(lfs, "nonexistent/child", info));
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_notdir_parent() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
    assert_ok(lfs_file_open(
        lfs,
        file,
        "f",
        LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
    ));
    assert_ok(lfs_file_close(lfs, file));
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_err(Error::NotDir, lfs_stat(lfs, "f/child", info));
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_empty(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_err(Error::Invalid, lfs_stat(lfs, "", info));
    if dir_mode {
        assert_err(Error::Invalid, lfs_mkdir(lfs, ""));
    } else {
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_err(
            Error::Invalid,
            lfs_file_open(lfs, file, "", LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL),
        );
    }
    assert_ok(lfs_mkdir(lfs, "x"));
    assert_err(Error::Invalid, lfs_rename(lfs, "x", ""));
    assert_err(Error::Invalid, lfs_rename(lfs, "", "y"));
    assert_err(Error::Invalid, lfs_remove(lfs, ""));
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_root_aliases(#[case] _dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    let aliases = &["/", ".", "./", "/.", "//"];
    for alias in aliases {
        let path = alias;
        let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
        assert_ok(lfs_stat(lfs, path, info));
        assert_eq!(info_name_str(info), "/");
        assert_eq!(info.type_, LFS_TYPE_DIR as u8);
    }
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_magic_noent() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    assert_ok(lfs_mkdir(lfs, "a"));
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_err(Error::NoEntry, lfs_stat(lfs, "a/b", info));
    assert_ok(lfs_unmount(lfs));
}

#[rstest]
#[case::dirs(true)]
#[case::files(false)]
fn test_paths_magic_conflict(#[case] dir_mode: bool) {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));

    if dir_mode {
        assert_ok(lfs_mkdir(lfs, "littlefs"));
    } else {
        let file = &mut unsafe { core::mem::MaybeUninit::<LfsFile>::zeroed().assume_init() };
        assert_ok(lfs_file_open(
            lfs,
            file,
            "littlefs",
            LFS_O_WRONLY | LFS_O_CREAT | LFS_O_EXCL,
        ));
        assert_ok(lfs_file_close(lfs, file));
    }
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, "littlefs", info));
    assert_eq!(info_name_str(info), "littlefs");
    assert_ok(lfs_rename(lfs, "littlefs", "coffee"));
    assert_ok(lfs_rename(lfs, "coffee", "littlefs"));
    assert_ok(lfs_stat(lfs, "littlefs", info));
    assert_err(Error::NoEntry, lfs_stat(lfs, "coffee", info));
    assert_ok(lfs_remove(lfs, "littlefs"));
    assert_err(Error::NoEntry, lfs_stat(lfs, "littlefs", info));
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_nametoolong() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    let long_name = "a".repeat(256);
    assert_err(Error::NameTooLong, lfs_mkdir(lfs, &long_name));
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_namejustlongenough() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    let max_name = "a".repeat(255);
    assert_ok(lfs_mkdir(lfs, &max_name));
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, &max_name, info));
    assert_eq!(info_name_str(info), max_name);
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_utf8() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    let name = "café_日本_한글";
    assert_ok(lfs_mkdir(lfs, name));
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, name, info));
    assert_eq!(info_name_str(info), name);
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_spaces() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    let name = "foo bar";
    assert_ok(lfs_mkdir(lfs, name));
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, name, info));
    assert_eq!(info_name_str(info), name);
    assert_ok(lfs_unmount(lfs));
}

#[test]
fn test_paths_nonprintable() {
    init_logger();
    let mut env = default_config(128);
    init_context(&mut env);
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    let mut name: Vec<u8> = vec![b'a'; 10];
    name[5] = 0x01;
    let name = unsafe { str::from_utf8_unchecked(&name) };
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
    let lfs = &mut Lfs::default();
    assert_ok(lfs_format(lfs, &env.config));
    assert_ok(lfs_mount(lfs, &env.config));
    #[allow(invalid_from_utf8_unchecked)]
    let name = unsafe { str::from_utf8_unchecked(b"foo\xff\xfe\xfdbar") };
    assert_ok(lfs_mkdir(lfs, name));
    let info = &mut unsafe { core::mem::MaybeUninit::<LfsInfo>::zeroed().assume_init() };
    assert_ok(lfs_stat(lfs, name, info));
    assert_ok(lfs_unmount(lfs));
}

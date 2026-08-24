//! Benchmarks for the safe `littlefs-rust` API.
//!
//! Every benchmark runs against a RAM-backed block device, so the measurements
//! reflect the cost of the filesystem logic itself (metadata handling, block
//! allocation, CTZ skip-list traversal, ...) rather than real flash latency.

use divan::{Bencher, black_box};
use littlefs_rust::{Config, Filesystem, OpenFlags, SeekFrom};

const BLOCK_SIZE: u32 = 512;
const BLOCK_COUNT: u32 = 256;

type Ram = littlefs_rust::RamStorage<BLOCK_SIZE, BLOCK_COUNT>;
type Fs = Filesystem<Ram>;

/// File sizes exercised by the I/O benchmarks: inline, single block, and
/// multi-block (CTZ skip list) payloads.
const SIZES: [usize; 3] = [64, 2048, 16384];

fn config() -> Config {
    Config::new(BLOCK_SIZE, BLOCK_COUNT)
}

fn formatted() -> Ram {
    let mut storage = Ram::new();
    Filesystem::format(&mut storage, &config()).expect("format");
    storage
}

fn mounted() -> Fs {
    Filesystem::mount(formatted(), config())
        .map_err(|(e, _)| e)
        .expect("mount")
}

/// Deterministic pseudo-random payload so results are stable across runs.
fn payload(size: usize) -> Vec<u8> {
    let mut state = 0x12345678u32;
    (0..size)
        .map(|_| {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            (state >> 24) as u8
        })
        .collect()
}

fn main() {
    divan::main();
}

// ── Mount lifecycle ─────────────────────────────────────────────────────────

mod lifecycle {
    use super::*;

    #[divan::bench]
    fn format(bencher: Bencher) {
        bencher
            .with_inputs(Ram::new)
            .bench_refs(|storage| Filesystem::format(storage, &config()).unwrap());
    }

    #[divan::bench]
    fn mount(bencher: Bencher) {
        bencher.with_inputs(formatted).bench_values(|storage| {
            Filesystem::mount(storage, config())
                .map_err(|(e, _)| e)
                .unwrap()
        });
    }

    #[divan::bench]
    fn mount_unmount(bencher: Bencher) {
        bencher.with_inputs(formatted).bench_values(|storage| {
            let fs = Filesystem::mount(storage, config())
                .map_err(|(e, _)| e)
                .unwrap();
            fs.unmount().unwrap()
        });
    }

    #[divan::bench]
    fn fs_size(bencher: Bencher) {
        let fs = mounted();
        for i in 0..16 {
            fs.write_file(&format!("/f{i}.bin"), &payload(1024))
                .unwrap();
        }
        bencher.bench_local(|| black_box(fs.fs_size().unwrap()));
    }
}

// ── File I/O ────────────────────────────────────────────────────────────────

mod file_io {
    use super::*;

    /// Create and fill a file in one call (open + write + close).
    #[divan::bench(args = SIZES)]
    fn write_file(bencher: Bencher, size: usize) {
        let data = payload(size);
        bencher
            .with_inputs(mounted)
            .bench_refs(|fs| fs.write_file("/data.bin", black_box(&data)).unwrap());
    }

    /// Read a whole file back into a freshly allocated buffer.
    #[divan::bench(args = SIZES)]
    fn read_to_vec(bencher: Bencher, size: usize) {
        let fs = mounted();
        fs.write_file("/data.bin", &payload(size)).unwrap();
        bencher.bench_local(|| black_box(fs.read_to_vec(black_box("/data.bin")).unwrap()));
    }

    /// Streaming write in 64-byte chunks, exercising the prog cache path.
    #[divan::bench(args = SIZES)]
    fn write_chunked(bencher: Bencher, size: usize) {
        let data = payload(size);
        bencher.with_inputs(mounted).bench_refs(|fs| {
            let mut file = fs
                .open(
                    "/chunked.bin",
                    OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNC,
                )
                .unwrap();
            for chunk in data.chunks(64) {
                file.write(chunk).unwrap();
            }
            file.close().unwrap();
        });
    }

    /// Streaming read in 64-byte chunks, exercising the read cache path.
    #[divan::bench(args = SIZES)]
    fn read_chunked(bencher: Bencher, size: usize) {
        let fs = mounted();
        fs.write_file("/chunked.bin", &payload(size)).unwrap();
        bencher.bench_local(|| {
            let mut file = fs.open("/chunked.bin", OpenFlags::READ).unwrap();
            let mut buf = [0u8; 64];
            let mut total = 0u32;
            loop {
                let n = file.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                total += n;
            }
            file.close().unwrap();
            black_box(total)
        });
    }

    /// Random access: seek back and forth in a multi-block file. This walks the
    /// CTZ skip list on every seek.
    #[divan::bench]
    fn seek_read(bencher: Bencher) {
        let fs = mounted();
        let size = 16384u32;
        fs.write_file("/seek.bin", &payload(size as usize)).unwrap();
        let offsets: Vec<u32> = (0..64).map(|i| (i * 7919) % (size - 128)).collect();
        bencher.bench_local(|| {
            let mut file = fs.open("/seek.bin", OpenFlags::READ).unwrap();
            let mut buf = [0u8; 128];
            let mut acc = 0u32;
            for &off in &offsets {
                file.seek(SeekFrom::Start(off)).unwrap();
                acc += file.read(&mut buf).unwrap();
            }
            file.close().unwrap();
            black_box(acc)
        });
    }

    /// Append to an existing file, then sync it to storage.
    #[divan::bench]
    fn append_and_sync(bencher: Bencher) {
        let data = payload(256);
        bencher
            .with_inputs(|| {
                let fs = mounted();
                fs.write_file("/log.bin", &payload(2048)).unwrap();
                fs
            })
            .bench_refs(|fs| {
                let mut file = fs
                    .open("/log.bin", OpenFlags::WRITE | OpenFlags::APPEND)
                    .unwrap();
                file.write(black_box(&data)).unwrap();
                file.sync().unwrap();
                file.close().unwrap();
            });
    }

    /// Truncating an existing multi-block file frees blocks back to the allocator.
    #[divan::bench]
    fn truncate(bencher: Bencher) {
        bencher
            .with_inputs(|| {
                let fs = mounted();
                fs.write_file("/trunc.bin", &payload(16384)).unwrap();
                fs
            })
            .bench_refs(|fs| {
                let mut file = fs.open("/trunc.bin", OpenFlags::WRITE).unwrap();
                file.truncate(black_box(512)).unwrap();
                file.close().unwrap();
            });
    }
}

// ── Metadata & directories ──────────────────────────────────────────────────

mod metadata {
    use super::*;

    const ENTRIES: usize = 32;

    fn populated_dir() -> Fs {
        let fs = mounted();
        fs.mkdir("/dir").unwrap();
        for i in 0..ENTRIES {
            fs.write_file(&format!("/dir/file{i:03}.bin"), &payload(64))
                .unwrap();
        }
        fs
    }

    /// Create a batch of directories in the root metadata pair.
    #[divan::bench]
    fn mkdir_many(bencher: Bencher) {
        bencher.with_inputs(mounted).bench_refs(|fs| {
            for i in 0..ENTRIES {
                fs.mkdir(&format!("/d{i:03}")).unwrap();
            }
        });
    }

    /// Create a deeply nested directory chain.
    #[divan::bench]
    fn mkdir_nested(bencher: Bencher) {
        let paths: Vec<String> = (1..=8)
            .map(|depth| {
                let mut p = String::new();
                for level in 0..depth {
                    p.push_str(&format!("/level{level}"));
                }
                p
            })
            .collect();
        bencher.with_inputs(mounted).bench_refs(|fs| {
            for path in &paths {
                fs.mkdir(path).unwrap();
            }
        });
    }

    /// List a directory holding 32 files.
    #[divan::bench]
    fn list_dir(bencher: Bencher) {
        let fs = populated_dir();
        bencher.bench_local(|| black_box(fs.list_dir("/dir").unwrap().len()));
    }

    /// Stat every entry of a populated directory.
    #[divan::bench]
    fn stat_entries(bencher: Bencher) {
        let fs = populated_dir();
        let paths: Vec<String> = (0..ENTRIES)
            .map(|i| format!("/dir/file{i:03}.bin"))
            .collect();
        bencher.bench_local(|| {
            let mut total = 0u32;
            for path in &paths {
                total += fs.stat(path).unwrap().size;
            }
            black_box(total)
        });
    }

    /// Lookup of a path that does not exist (worst-case directory scan).
    #[divan::bench]
    fn exists_missing(bencher: Bencher) {
        let fs = populated_dir();
        bencher.bench_local(|| black_box(fs.exists("/dir/missing.bin")));
    }

    /// Rename within the same directory: pure metadata commit.
    #[divan::bench]
    fn rename(bencher: Bencher) {
        bencher
            .with_inputs(populated_dir)
            .bench_refs(|fs| fs.rename("/dir/file000.bin", "/dir/renamed.bin").unwrap());
    }

    /// Remove every file of a populated directory.
    #[divan::bench]
    fn remove_many(bencher: Bencher) {
        bencher.with_inputs(populated_dir).bench_refs(|fs| {
            for i in 0..ENTRIES {
                fs.remove(&format!("/dir/file{i:03}.bin")).unwrap();
            }
        });
    }
}

// ── End-to-end workloads ────────────────────────────────────────────────────

mod workload {
    use super::*;

    /// Format, mount, build a small tree, read it back and unmount.
    #[divan::bench]
    fn full_session(bencher: Bencher) {
        let data = payload(1024);
        bencher.bench_local(|| {
            let fs = mounted();
            fs.mkdir("/cfg").unwrap();
            fs.mkdir("/cfg/nested").unwrap();
            for i in 0..8 {
                fs.write_file(&format!("/cfg/nested/item{i}.bin"), &data)
                    .unwrap();
            }
            let mut total = 0usize;
            for i in 0..8 {
                total += fs
                    .read_to_vec(&format!("/cfg/nested/item{i}.bin"))
                    .unwrap()
                    .len();
            }
            fs.unmount().unwrap();
            black_box(total)
        });
    }

    /// Repeatedly rewrite the same file, which forces block reuse and metadata
    /// compaction.
    #[divan::bench]
    fn rewrite_churn(bencher: Bencher) {
        let data = payload(2048);
        bencher.with_inputs(mounted).bench_refs(|fs| {
            for _ in 0..8 {
                fs.write_file("/churn.bin", black_box(&data)).unwrap();
            }
        });
    }
}

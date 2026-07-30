//! Storage inspection and safe cache cleanup for the Takokit home directory.

use crate::args::{StorageArgs, StorageCommand};
use serde::Serialize;
use takokit_package::{automatic_cleanup_state, clean_uv_cache};
use std::{
    collections::HashSet,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

const HARDLINK_IDENTITY_MIN_BYTES: u64 = 256 * 1024;
const CATEGORY_ORDER: &[&str] = &[
    "models",
    "blobs",
    "tools",
    "runners",
    "cache",
    "voices",
    "datasets",
    "outputs",
    "logs",
    "manifests",
    "progress",
    "runtime",
];

#[derive(Debug, Clone, Serialize)]
pub(crate) struct StorageCategoryReport {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) files: u64,
    pub(crate) logical_bytes: u64,
    pub(crate) unique_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct StorageReport {
    pub(crate) root: PathBuf,
    pub(crate) files: u64,
    pub(crate) logical_bytes: u64,
    pub(crate) unique_bytes: u64,
    pub(crate) hardlink_savings_bytes: u64,
    pub(crate) hardlink_identity_threshold_bytes: u64,
    pub(crate) uv_cache_logical_bytes: u64,
    pub(crate) categories: Vec<StorageCategoryReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum FileIdentity {
    #[cfg(windows)]
    Windows {
        volume: u32,
        index_high: u32,
        index_low: u32,
    },
    #[cfg(unix)]
    Unix {
        device: u64,
        inode: u64,
    },
    Fallback(PathBuf),
}

#[derive(Debug, Default)]
struct ScanTotals {
    files: u64,
    logical_bytes: u64,
    unique_bytes: u64,
}

pub(crate) fn run_storage_command(
    root: &Path,
    args: StorageArgs,
    json: bool,
) -> anyhow::Result<()> {
    match args.command {
        None => {
            let json_requested = args.json || json;
            if !json_requested {
                eprintln!("Scanning Takokit storage at {}...", root.display());
                io::stderr().flush()?;
            }
            let report = inspect_storage(root)?;
            if json_requested {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print_storage_report(&report);
            }
        }
        Some(StorageCommand::Status) => {
            let state = automatic_cleanup_state(root)?;
            if args.json || json {
                println!("{}", serde_json::to_string_pretty(&state)?);
            } else {
                println!("Takokit automatic storage cleanup");
                println!("  enabled      {}", if state.enabled { "yes" } else { "no" });
                println!("  status       {}", state.status);
                println!(
                    "  reclaimed    {}",
                    format_bytes(state.reclaimed_bytes)
                );
                if let Some(reason) = state.skip_reason {
                    println!("  skipped      {reason}");
                }
                if let Some(error) = state.error {
                    println!("  error        {error}");
                }
                println!(
                    "  configuration TAKOKIT_AUTO_STORAGE_CLEANUP=0 disables background cleanup"
                );
            }
        }
        Some(StorageCommand::Clean { dry_run }) => {
            let report = clean_uv_cache(root, dry_run)?;
            if args.json || json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("Takokit storage cleanup");
                println!("  target       {}", report.target.display());
                println!(
                    "  cache size   {}",
                    format_bytes(report.reclaimed_bytes)
                );
                println!(
                    "  mode         {}",
                    if report.dry_run { "dry-run" } else { "clean" }
                );
                println!(
                    "  removed      {}",
                    if report.removed { "yes" } else { "no" }
                );
            }
        }
    }
    Ok(())
}

pub(crate) fn inspect_storage(root: &Path) -> io::Result<StorageReport> {
    let mut seen = HashSet::new();
    let mut categories = Vec::new();
    let mut total = ScanTotals::default();

    for name in CATEGORY_ORDER {
        let path = root.join(name);
        let category = scan_category(name, &path, &mut seen)?;
        total.files += category.files;
        total.logical_bytes += category.logical_bytes;
        total.unique_bytes += category.unique_bytes;
        categories.push(category);
    }

    let other = scan_other(root, &mut seen)?;
    total.files += other.files;
    total.logical_bytes += other.logical_bytes;
    total.unique_bytes += other.unique_bytes;
    if other.files > 0 || other.logical_bytes > 0 {
        categories.push(other);
    }

    let uv_cache_logical_bytes = scan_logical_only(&root.join("cache").join("uv"))?.logical_bytes;
    Ok(StorageReport {
        root: root.to_path_buf(),
        files: total.files,
        logical_bytes: total.logical_bytes,
        unique_bytes: total.unique_bytes,
        hardlink_savings_bytes: total.logical_bytes.saturating_sub(total.unique_bytes),
        hardlink_identity_threshold_bytes: HARDLINK_IDENTITY_MIN_BYTES,
        uv_cache_logical_bytes,
        categories,
    })
}

fn scan_category(
    name: &str,
    path: &Path,
    seen: &mut HashSet<FileIdentity>,
) -> io::Result<StorageCategoryReport> {
    let totals = scan_path(path, seen)?;
    Ok(StorageCategoryReport {
        name: name.to_string(),
        path: path.to_path_buf(),
        files: totals.files,
        logical_bytes: totals.logical_bytes,
        unique_bytes: totals.unique_bytes,
    })
}

fn scan_other(root: &Path, seen: &mut HashSet<FileIdentity>) -> io::Result<StorageCategoryReport> {
    let mut totals = ScanTotals::default();
    if root.exists() {
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let name = entry.file_name();
            if CATEGORY_ORDER.iter().any(|known| name == *known) {
                continue;
            }
            merge_totals(&mut totals, scan_path(&entry.path(), seen)?);
        }
    }
    Ok(StorageCategoryReport {
        name: "other".to_string(),
        path: root.to_path_buf(),
        files: totals.files,
        logical_bytes: totals.logical_bytes,
        unique_bytes: totals.unique_bytes,
    })
}

fn scan_path(path: &Path, seen: &mut HashSet<FileIdentity>) -> io::Result<ScanTotals> {
    if !path.exists() {
        return Ok(ScanTotals::default());
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok(ScanTotals::default());
    }
    if metadata.is_file() {
        let length = metadata.len();
        let unique = if length >= HARDLINK_IDENTITY_MIN_BYTES {
            seen.insert(file_identity(path, &metadata))
        } else {
            true
        };
        return Ok(ScanTotals {
            files: 1,
            logical_bytes: length,
            unique_bytes: if unique { length } else { 0 },
        });
    }

    let mut totals = ScanTotals::default();
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            merge_totals(&mut totals, scan_path(&entry.path(), seen)?);
        }
    }
    Ok(totals)
}

fn scan_logical_only(path: &Path) -> io::Result<ScanTotals> {
    if !path.exists() {
        return Ok(ScanTotals::default());
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok(ScanTotals::default());
    }
    if metadata.is_file() {
        return Ok(ScanTotals {
            files: 1,
            logical_bytes: metadata.len(),
            unique_bytes: metadata.len(),
        });
    }

    let mut totals = ScanTotals::default();
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            merge_totals(&mut totals, scan_logical_only(&entry.path())?);
        }
    }
    Ok(totals)
}

fn merge_totals(target: &mut ScanTotals, source: ScanTotals) {
    target.files += source.files;
    target.logical_bytes += source.logical_bytes;
    target.unique_bytes += source.unique_bytes;
}

fn print_storage_report(report: &StorageReport) {
    println!("Takokit storage");
    println!("  root              {}", report.root.display());
    println!("  estimated unique  {}", format_bytes(report.unique_bytes));
    println!("  logical paths     {}", format_bytes(report.logical_bytes));
    println!(
        "  hardlink savings  {}",
        format_bytes(report.hardlink_savings_bytes)
    );
    println!("  files             {}", report.files);
    println!(
        "  cleanable UV cache {}",
        format_bytes(report.uv_cache_logical_bytes)
    );
    println!(
        "  scan mode         fast (deduplicates files >= {})",
        format_bytes(report.hardlink_identity_threshold_bytes)
    );
    println!();
    println!("CATEGORY       UNIQUE      LOGICAL       FILES");
    for category in &report.categories {
        println!(
            "{:<12} {:>10} {:>12} {:>11}",
            category.name,
            format_bytes(category.unique_bytes),
            format_bytes(category.logical_bytes),
            category.files
        );
    }
    println!();
    println!("Use `tako storage clean --dry-run` to preview UV-cache cleanup.");
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}

#[cfg(windows)]
fn file_identity(path: &Path, _metadata: &fs::Metadata) -> FileIdentity {
    use std::{ffi::c_void, fs::File, mem::MaybeUninit, os::windows::io::AsRawHandle};

    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    #[repr(C)]
    struct ByHandleFileInformation {
        attributes: u32,
        creation_time: FileTime,
        access_time: FileTime,
        write_time: FileTime,
        volume_serial_number: u32,
        file_size_high: u32,
        file_size_low: u32,
        number_of_links: u32,
        file_index_high: u32,
        file_index_low: u32,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetFileInformationByHandle(
            handle: *mut c_void,
            information: *mut ByHandleFileInformation,
        ) -> i32;
    }

    if let Ok(file) = File::open(path) {
        let mut information = MaybeUninit::<ByHandleFileInformation>::uninit();
        let success =
            unsafe { GetFileInformationByHandle(file.as_raw_handle(), information.as_mut_ptr()) };
        if success != 0 {
            let information = unsafe { information.assume_init() };
            return FileIdentity::Windows {
                volume: information.volume_serial_number,
                index_high: information.file_index_high,
                index_low: information.file_index_low,
            };
        }
    }
    FileIdentity::Fallback(path.to_path_buf())
}

#[cfg(unix)]
fn file_identity(path: &Path, metadata: &fs::Metadata) -> FileIdentity {
    use std::os::unix::fs::MetadataExt;
    let device = metadata.dev();
    let inode = metadata.ino();
    if inode == 0 {
        FileIdentity::Fallback(path.to_path_buf())
    } else {
        FileIdentity::Unix { device, inode }
    }
}

#[cfg(not(any(windows, unix)))]
fn file_identity(path: &Path, _metadata: &fs::Metadata) -> FileIdentity {
    FileIdentity::Fallback(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_deduplicates_large_hardlinked_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let tools = temp.path().join("tools");
        let runners = temp.path().join("runners");
        fs::create_dir_all(&tools).expect("tools");
        fs::create_dir_all(&runners).expect("runners");
        let original = tools.join("torch.dll");
        let bytes = HARDLINK_IDENTITY_MIN_BYTES as usize + 4096;
        fs::write(&original, vec![7_u8; bytes]).expect("write original");
        fs::hard_link(&original, runners.join("torch.dll")).expect("hardlink");

        let report = inspect_storage(temp.path()).expect("inspect");
        assert_eq!(report.logical_bytes, (bytes * 2) as u64);
        assert_eq!(report.unique_bytes, bytes as u64);
        assert_eq!(report.hardlink_savings_bytes, bytes as u64);
    }

    #[test]
    fn clean_dry_run_preserves_uv_cache() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cache = temp.path().join("cache").join("uv");
        fs::create_dir_all(&cache).expect("cache");
        fs::write(cache.join("package.whl"), vec![1_u8; 128]).expect("cache file");

        let preview = clean_uv_cache(temp.path(), true).expect("preview");
        assert!(!preview.removed);
        assert!(cache.join("package.whl").exists());

        let cleaned = clean_uv_cache(temp.path(), false).expect("clean");
        assert!(cleaned.removed);
        assert!(cache.exists());
        assert!(!cache.join("package.whl").exists());
    }
}

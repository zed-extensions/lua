use std::{
    fs, io,
    path::{Path, PathBuf},
};

use semver::Version;

pub(crate) fn resolve_binary_path(
    user_binary_path: Option<String>,
    cached_binary_path: &mut Option<String>,
    work_dir: &Path,
    binary_name: &str,
    download_binary: impl FnOnce() -> Result<String, String>,
) -> Result<String, String> {
    if let Some(path) = user_binary_path {
        return Ok(path);
    }

    if let Some(path) = cached_binary_path.as_ref() {
        if fs::metadata(path).is_ok_and(|stat| stat.is_file()) {
            return Ok(path.clone());
        }
    }

    let path = match download_binary() {
        Ok(path) => path,
        Err(download_error) => {
            let path = find_cached_binary(work_dir, binary_name)
                .map_err(|cache_error| format!("{download_error}; {cache_error}"))?
                .ok_or_else(|| download_error.clone())?;
            eprintln!(
                "failed to update lua-language-server: {download_error}; using {}",
                path.display()
            );
            path.into_os_string()
                .into_string()
                .map_err(|path| format!("language server path is not UTF-8: {path:?}"))?
        }
    };

    *cached_binary_path = Some(path.clone());
    Ok(path)
}

pub(crate) fn install_binary(
    version_dir: &Path,
    binary_name: &str,
    download: impl FnOnce(&str) -> Result<(), String>,
) -> Result<(), String> {
    let name = version_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            format!(
                "invalid language server directory: {}",
                version_dir.display()
            )
        })?;
    // A failed extraction must not leave a version that offline lookup considers installed.
    let mut attempt = 0u32;
    let (download_dir, download_path) = loop {
        let directory = version_dir.with_file_name(format!(".{name}.tmp-{attempt}"));
        let path = directory
            .to_str()
            .ok_or_else(|| format!("invalid download directory: {}", directory.display()))?
            .to_owned();
        match fs::create_dir(&directory) {
            Ok(()) => break (directory, path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                attempt = attempt
                    .checked_add(1)
                    .ok_or("too many temporary downloads")?;
            }
            Err(error) => return Err(format!("failed to create download directory: {error}")),
        }
    };

    let result = (|| {
        download(&download_path)?;
        let binary_path = download_dir.join("bin").join(binary_name);
        if !fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            return Err(format!(
                "download did not contain {}",
                binary_path.display()
            ));
        }

        match fs::rename(&download_dir, version_dir) {
            Ok(()) => Ok(()),
            Err(_) if version_dir.join("bin").join(binary_name).is_file() => {
                // Another instance may have finished installing this version first.
                remove_directory_if_exists(&download_dir)
            }
            Err(_) if version_dir.is_dir() => {
                remove_directory_if_exists(version_dir)?;
                fs::rename(&download_dir, version_dir)
                    .map_err(|error| format!("failed to replace incomplete installation: {error}"))
            }
            Err(error) => Err(format!("failed to install language server: {error}")),
        }
    })();

    if result.is_err() {
        if let Err(error) = remove_directory_if_exists(&download_dir) {
            eprintln!("failed to clean up language server download: {error}");
        }
    }
    result
}

fn remove_directory_if_exists(path: &Path) -> Result<(), String> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("failed to remove {}: {error}", path.display())),
    }
}

fn find_cached_binary(work_dir: &Path, binary_name: &str) -> Result<Option<PathBuf>, String> {
    let entries = match fs::read_dir(work_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("failed to list cached language servers: {error}")),
    };
    let mut latest_binary = None;

    for entry in entries {
        let entry =
            entry.map_err(|error| format!("failed to read cached language server: {error}"))?;
        let name = entry.file_name();
        let Some(version) = name
            .to_str()
            .and_then(|name| name.strip_prefix("lua-language-server-"))
        else {
            continue;
        };
        let Ok(version) = Version::parse(version.strip_prefix('v').unwrap_or(version)) else {
            continue;
        };
        if !version.pre.is_empty() {
            continue;
        }

        let path = entry.path().join("bin").join(binary_name);
        if !fs::metadata(&path).is_ok_and(|stat| stat.is_file()) {
            continue;
        }

        let candidate = (version, path);
        if latest_binary
            .as_ref()
            .map_or(true, |latest| candidate > *latest)
        {
            latest_binary = Some(candidate);
        }
    }

    Ok(latest_binary.map(|(_, path)| path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::Cell,
        error::Error,
        sync::atomic::{AtomicUsize, Ordering},
    };

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> io::Result<Self> {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
            loop {
                let path = std::env::temp_dir().join(format!(
                    "zed-lua-binary-test-{}-{}",
                    std::process::id(),
                    NEXT_ID.fetch_add(1, Ordering::Relaxed)
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Ok(Self(path)),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => return Err(error),
                }
            }
        }

        fn binary(&self, version: &str, name: &str) -> io::Result<PathBuf> {
            let path = self
                .0
                .join(format!("lua-language-server-{version}/bin/{name}"));
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, "language server")?;
            Ok(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            if let Err(error) = fs::remove_dir_all(&self.0) {
                eprintln!(
                    "failed to remove test directory {}: {error}",
                    self.0.display()
                );
            }
        }
    }

    #[test]
    fn uses_user_binary_without_downloading() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        let cached_path = directory.binary("3.9.0", "lua-language-server")?;
        for mut cached_binary in [None, Some(cached_path.to_string_lossy().into_owned())] {
            let original_cache = cached_binary.clone();
            let download_count = Cell::new(0);

            let path = resolve_binary_path(
                Some("/usr/bin/lua-language-server".into()),
                &mut cached_binary,
                &directory.0,
                "lua-language-server",
                || {
                    download_count.set(download_count.get() + 1);
                    Err("offline".into())
                },
            )?;

            assert_eq!(path, "/usr/bin/lua-language-server");
            assert_eq!(download_count.get(), 0);
            assert_eq!(cached_binary, original_cache);
        }
        Ok(())
    }

    #[test]
    fn uses_memory_cache_without_downloading() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        let cached_path = directory.binary("3.9.0", "lua-language-server")?;
        let mut cached_binary = Some(cached_path.to_string_lossy().into_owned());
        let download_count = Cell::new(0);

        let path = resolve_binary_path(
            None,
            &mut cached_binary,
            &directory.0,
            "lua-language-server",
            || {
                download_count.set(download_count.get() + 1);
                Err("offline".into())
            },
        )?;

        assert_eq!(Path::new(&path), cached_path);
        assert_eq!(download_count.get(), 0);
        Ok(())
    }

    #[test]
    fn falls_back_to_downloaded_binary_after_restart() -> Result<(), Box<dyn Error>> {
        for name in ["lua-language-server", "lua-language-server.exe"] {
            let directory = TestDirectory::new()?;
            let cached_path = directory.binary("3.9.0", name)?;
            let mut cached_binary = None;
            let download_count = Cell::new(0);

            let path = resolve_binary_path(None, &mut cached_binary, &directory.0, name, || {
                download_count.set(download_count.get() + 1);
                Err("GitHub unavailable".into())
            })?;

            assert_eq!(Path::new(&path), cached_path);
            assert_eq!(cached_binary.as_deref(), Some(path.as_str()));
            assert_eq!(download_count.get(), 1);
        }
        Ok(())
    }

    #[test]
    fn prefers_successful_update_to_downloaded_cache() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.binary("3.9.0", "lua-language-server")?;
        let updated_path = directory.binary("3.10.0", "lua-language-server")?;
        let mut cached_binary = None;

        let path = resolve_binary_path(
            None,
            &mut cached_binary,
            &directory.0,
            "lua-language-server",
            || Ok(updated_path.to_string_lossy().into_owned()),
        )?;

        assert_eq!(Path::new(&path), updated_path);
        assert_eq!(cached_binary.as_deref(), Some(path.as_str()));
        Ok(())
    }

    #[test]
    fn downloads_when_no_binary_is_installed() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        let mut cached_binary = None;
        let download_count = Cell::new(0);

        let path = resolve_binary_path(
            None,
            &mut cached_binary,
            &directory.0,
            "lua-language-server",
            || {
                download_count.set(download_count.get() + 1);
                let path = directory
                    .binary("3.10.0", "lua-language-server")
                    .map_err(|error| error.to_string())?;
                Ok(path.to_string_lossy().into_owned())
            },
        )?;

        assert!(Path::new(&path).is_file());
        assert_eq!(download_count.get(), 1);
        assert_eq!(cached_binary.as_deref(), Some(path.as_str()));
        Ok(())
    }

    #[test]
    fn preserves_download_error_without_usable_cache() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        fs::create_dir_all(
            directory
                .0
                .join("lua-language-server-3.10.0/bin/lua-language-server"),
        )?;
        directory.binary("3.9.0", "lua-language-server.exe")?;
        let mut cached_binary = None;

        let result = resolve_binary_path(
            None,
            &mut cached_binary,
            &directory.0,
            "lua-language-server",
            || Err("GitHub rate limit exceeded".into()),
        );

        assert_eq!(result, Err("GitHub rate limit exceeded".into()));
        assert_eq!(cached_binary, None);
        Ok(())
    }

    #[test]
    fn chooses_latest_complete_stable_download() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.binary("3.9.0", "lua-language-server")?;
        let latest = directory.binary("v3.10.0", "lua-language-server")?;
        directory.binary("invalid", "lua-language-server")?;
        directory.binary("3.11.0-beta.1", "lua-language-server")?;
        directory.binary("3.12.0.tmp", "lua-language-server")?;
        fs::create_dir_all(directory.0.join("lua-language-server-3.11.0/bin"))?;
        fs::create_dir_all(directory.0.join("unrelated-4.0.0/bin"))?;
        fs::write(
            directory.0.join("unrelated-4.0.0/bin/lua-language-server"),
            "unrelated",
        )?;

        assert_eq!(
            find_cached_binary(&directory.0, "lua-language-server")?,
            Some(latest)
        );
        Ok(())
    }

    #[test]
    fn ignores_missing_memory_cached_binary() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        let cached_path = directory.binary("3.9.0", "lua-language-server")?;
        let mut cached_binary = Some(directory.0.join("missing").to_string_lossy().into_owned());

        let path = resolve_binary_path(
            None,
            &mut cached_binary,
            &directory.0,
            "lua-language-server",
            || Err("offline".into()),
        )?;

        assert_eq!(Path::new(&path), cached_path);
        assert_eq!(cached_binary.as_deref(), Some(path.as_str()));
        Ok(())
    }

    #[test]
    fn reports_cache_read_errors_with_download_error() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        let not_a_directory = directory.0.join("file");
        fs::write(&not_a_directory, "not a directory")?;
        let mut cached_binary = None;

        let result = resolve_binary_path(
            None,
            &mut cached_binary,
            &not_a_directory,
            "lua-language-server",
            || Err("offline".into()),
        );

        assert!(result.as_ref().is_err_and(|error| error.contains("offline")
            && error.contains("failed to list cached language servers")));
        assert_eq!(cached_binary, None);
        Ok(())
    }

    #[test]
    fn installs_only_after_download_completes() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        let version_dir = directory.0.join("lua-language-server-3.10.0");
        let destination = version_dir.join("bin/lua-language-server");

        install_binary(&version_dir, "lua-language-server", |download_dir| {
            assert!(!destination.exists());
            let download_dir = Path::new(download_dir);
            fs::create_dir_all(download_dir.join("bin")).map_err(|error| error.to_string())?;
            fs::write(download_dir.join("bin/lua-language-server"), "new binary")
                .map_err(|error| error.to_string())?;
            fs::write(download_dir.join("support.lua"), "support files")
                .map_err(|error| error.to_string())?;
            assert_eq!(
                find_cached_binary(&directory.0, "lua-language-server")?,
                None
            );
            Ok(())
        })?;

        assert_eq!(fs::read_to_string(destination)?, "new binary");
        assert_eq!(
            fs::read_to_string(version_dir.join("support.lua"))?,
            "support files"
        );
        assert!(!directory
            .0
            .join(".lua-language-server-3.10.0.tmp-0")
            .exists());
        Ok(())
    }

    #[test]
    fn failed_extraction_keeps_previous_binary_usable() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        let previous_binary = directory.binary("3.9.0", "lua-language-server")?;
        let version_dir = directory.0.join("lua-language-server-3.10.0");
        let mut cached_binary = None;

        let path = resolve_binary_path(
            None,
            &mut cached_binary,
            &directory.0,
            "lua-language-server",
            || {
                install_binary(&version_dir, "lua-language-server", |download_dir| {
                    let download_dir = Path::new(download_dir);
                    fs::create_dir_all(download_dir.join("bin"))
                        .map_err(|error| error.to_string())?;
                    fs::write(
                        download_dir.join("bin/lua-language-server"),
                        "partial binary",
                    )
                    .map_err(|error| error.to_string())?;
                    Err("archive extraction failed".into())
                })?;
                Ok(version_dir
                    .join("bin/lua-language-server")
                    .to_string_lossy()
                    .into_owned())
            },
        )?;

        assert_eq!(Path::new(&path), previous_binary);
        assert_eq!(fs::read_to_string(previous_binary)?, "language server");
        assert!(!version_dir.exists());
        assert!(!directory
            .0
            .join(".lua-language-server-3.10.0.tmp-0")
            .exists());
        Ok(())
    }

    #[test]
    fn rejects_download_without_expected_binary() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        let previous_binary = directory.binary("3.9.0", "lua-language-server")?;
        let version_dir = directory.0.join("lua-language-server-3.10.0");

        let result = install_binary(&version_dir, "lua-language-server", |download_dir| {
            fs::create_dir_all(download_dir).map_err(|error| error.to_string())?;
            fs::write(Path::new(download_dir).join("other-file"), "not a binary")
                .map_err(|error| error.to_string())
        });

        assert!(result.is_err_and(|error| error.contains("download did not contain")));
        assert!(previous_binary.is_file());
        assert!(!version_dir.exists());
        assert!(!directory
            .0
            .join(".lua-language-server-3.10.0.tmp-0")
            .exists());
        Ok(())
    }

    #[test]
    fn does_not_reuse_existing_temporary_downloads() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        let stale_download = directory.0.join(".lua-language-server-3.10.0.tmp-0/bin");
        fs::create_dir_all(&stale_download)?;
        fs::write(stale_download.join("lua-language-server"), "partial binary")?;
        let version_dir = directory.0.join("lua-language-server-3.10.0");

        let result = install_binary(&version_dir, "lua-language-server", |download_dir| {
            assert!(Path::new(download_dir).is_dir());
            assert!(!Path::new(download_dir)
                .join("bin/lua-language-server")
                .exists());
            Ok(())
        });

        assert!(result.is_err_and(|error| error.contains("download did not contain")));
        assert!(!version_dir.exists());
        assert!(stale_download.join("lua-language-server").is_file());
        assert!(!directory
            .0
            .join(".lua-language-server-3.10.0.tmp-1")
            .exists());
        Ok(())
    }

    #[test]
    fn replaces_an_incomplete_installation_after_successful_download() -> Result<(), Box<dyn Error>>
    {
        let directory = TestDirectory::new()?;
        let version_dir = directory.0.join("lua-language-server-3.10.0");
        fs::create_dir_all(&version_dir)?;
        fs::write(version_dir.join("incomplete"), "old partial download")?;

        install_binary(&version_dir, "lua-language-server", |download_dir| {
            assert!(version_dir.join("incomplete").is_file());
            let binary = Path::new(download_dir).join("bin/lua-language-server");
            fs::create_dir_all(Path::new(download_dir).join("bin"))
                .map_err(|error| error.to_string())?;
            fs::write(binary, "complete binary").map_err(|error| error.to_string())
        })?;

        assert!(!version_dir.join("incomplete").exists());
        assert_eq!(
            fs::read_to_string(version_dir.join("bin/lua-language-server"))?,
            "complete binary"
        );
        Ok(())
    }

    #[test]
    fn overlapping_downloads_preserve_the_completed_installation() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        let version_dir = directory.0.join("lua-language-server-3.10.0");

        install_binary(&version_dir, "lua-language-server", |first_download| {
            let first_download = Path::new(first_download);
            fs::create_dir_all(first_download.join("bin")).map_err(|error| error.to_string())?;
            fs::write(
                first_download.join("bin/lua-language-server"),
                "finishes second",
            )
            .map_err(|error| error.to_string())?;

            install_binary(&version_dir, "lua-language-server", |second_download| {
                let second_download = Path::new(second_download);
                assert_ne!(first_download, second_download);
                fs::create_dir_all(second_download.join("bin"))
                    .map_err(|error| error.to_string())?;
                fs::write(
                    second_download.join("bin/lua-language-server"),
                    "finishes first",
                )
                .map_err(|error| error.to_string())
            })?;
            assert!(first_download.join("bin/lua-language-server").is_file());
            Ok(())
        })?;

        assert_eq!(
            fs::read_to_string(version_dir.join("bin/lua-language-server"))?,
            "finishes first"
        );
        assert!(!directory
            .0
            .join(".lua-language-server-3.10.0.tmp-0")
            .exists());
        assert!(!directory
            .0
            .join(".lua-language-server-3.10.0.tmp-1")
            .exists());
        Ok(())
    }
}

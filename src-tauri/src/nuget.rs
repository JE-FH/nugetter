use crate::models::{LocalPackageInfo, PendingCopyRequest};
use regex::Regex;
use semver::Version;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

pub fn is_package_file(path: &Path) -> bool {
    path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "nupkg" | "nuget"))
        .unwrap_or(false)
}

pub fn read_package_metadata(path: &Path) -> Result<(String, String), String> {
    let file = fs::File::open(path)
        .map_err(|e| format!("Failed to open package {}: {e}", path.display()))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Failed to read package archive {}: {e}", path.display()))?;

    let mut nuspec_content = String::new();
    let mut found = false;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to access package entry: {e}"))?;

        if entry.name().ends_with(".nuspec") {
            entry
                .read_to_string(&mut nuspec_content)
                .map_err(|e| format!("Failed to read nuspec from {}: {e}", path.display()))?;
            found = true;
            break;
        }
    }

    if !found {
        return Err(format!("No .nuspec file found in {}", path.display()));
    }

    let id_re = Regex::new(r"<id>\s*([^<]+)\s*</id>").map_err(|e| e.to_string())?;
    let version_re = Regex::new(r"<version>\s*([^<]+)\s*</version>").map_err(|e| e.to_string())?;

    let package_id = id_re
        .captures(&nuspec_content)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().trim().to_string()))
        .ok_or_else(|| format!("Could not find package id in {}", path.display()))?;

    let version = version_re
        .captures(&nuspec_content)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().trim().to_string()))
        .ok_or_else(|| format!("Could not find version in {}", path.display()))?;

    Ok((package_id, version))
}

pub fn compute_next_version(
    destination_path: &Path,
    package_id: &str,
    current_version: &str,
    configured_start_version: Option<&str>,
) -> String {
    let highest_in_destination = highest_version_in_destination(destination_path, package_id);
    next_version_from_known_versions(current_version, highest_in_destination, configured_start_version)
}

pub fn list_local_packages(destination_path: &Path) -> Result<Vec<LocalPackageInfo>, String> {
    if !destination_path.exists() || !destination_path.is_dir() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(destination_path)
        .map_err(|e| format!("Failed to read destination {}: {e}", destination_path.display()))?;

    let mut by_package: HashMap<String, String> = HashMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_package_file(&path) || !path.is_file() {
            continue;
        }

        let Ok((package_id, version)) = read_package_metadata(&path) else {
            continue;
        };

        match by_package.get(&package_id) {
            Some(existing) if !is_version_higher(&version, existing) => {}
            _ => {
                by_package.insert(package_id, version);
            }
        }
    }

    let mut packages: Vec<LocalPackageInfo> = by_package
        .into_iter()
        .map(|(package_id, latest_version)| LocalPackageInfo {
            package_id,
            latest_version,
        })
        .collect();
    packages.sort_by(|a, b| a.package_id.cmp(&b.package_id));

    Ok(packages)
}

fn is_version_higher(candidate: &str, existing: &str) -> bool {
    match (Version::parse(candidate), Version::parse(existing)) {
        (Ok(left), Ok(right)) => left > right,
        _ => candidate > existing,
    }
}

fn next_version_from_known_versions(
    current_version: &str,
    highest_in_destination: Option<Version>,
    configured_start_version: Option<&str>,
) -> String {
    let current_parsed = Version::parse(current_version).ok();
    let start_parsed = configured_start_version.and_then(|v| Version::parse(v).ok());

    if highest_in_destination.is_none() {
        if let Some(start) = start_parsed {
            let selected = match current_parsed {
                Some(current) if current > start => current,
                _ => start,
            };
            return selected.to_string();
        }
    }

    let mut base = match (current_parsed, highest_in_destination, start_parsed) {
        (Some(current), Some(highest), maybe_start) => {
            let mut candidate = if current > highest { current } else { highest };
            if let Some(start) = maybe_start {
                if start > candidate {
                    candidate = start;
                }
            }
            candidate
        }
        (Some(current), None, Some(start)) => {
            if start > current {
                start
            } else {
                current
            }
        }
        (Some(current), None, None) => current,
        (None, Some(highest), Some(start)) => {
            if start > highest {
                start
            } else {
                highest
            }
        }
        (None, Some(highest), None) => highest,
        (None, None, _) => return format!("{}.1", current_version),
    };

    base.patch += 1;
    base.pre = semver::Prerelease::EMPTY;
    base.build = semver::BuildMetadata::EMPTY;
    base.to_string()
}

fn highest_version_in_destination(destination_path: &Path, package_id: &str) -> Option<Version> {
    let mut highest: Option<Version> = None;

    let entries = fs::read_dir(destination_path).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_package_file(&path) || !path.is_file() {
            continue;
        }

        let Ok((id, version)) = read_package_metadata(&path) else {
            continue;
        };
        if id != package_id {
            continue;
        }

        let Ok(parsed) = Version::parse(&version) else {
            continue;
        };

        highest = match highest {
            Some(existing) if existing >= parsed => Some(existing),
            _ => Some(parsed),
        };
    }

    highest
}

pub fn repackage_with_new_version(request: &PendingCopyRequest) -> Result<PathBuf, String> {
    let source_file = fs::File::open(&request.source_path)
        .map_err(|e| format!("Failed to open source package: {e}"))?;
    let mut source_archive =
        ZipArchive::new(source_file).map_err(|e| format!("Failed to read source package: {e}"))?;

    let temp_name = format!(
        "nugetter-{}-{}.tmp",
        request.request_id,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );
    let temp_path = std::env::temp_dir().join(temp_name);
    let temp_file = fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temporary file: {e}"))?;
    let mut writer = ZipWriter::new(temp_file);

    let version_re = Regex::new(r"(?s)<version>\s*[^<]*\s*</version>").map_err(|e| e.to_string())?;

    for i in 0..source_archive.len() {
        let mut entry = source_archive
            .by_index(i)
            .map_err(|e| format!("Failed to access source entry: {e}"))?;

        let options = SimpleFileOptions::default()
            .compression_method(entry.compression())
            .unix_permissions(entry.unix_mode().unwrap_or(0o644));

        if entry.is_dir() {
            writer
                .add_directory(entry.name(), options)
                .map_err(|e| format!("Failed to write directory entry: {e}"))?;
            continue;
        }

        writer
            .start_file(entry.name(), options)
            .map_err(|e| format!("Failed to write file entry: {e}"))?;

        if entry.name().ends_with(".nuspec") {
            let mut content = String::new();
            entry
                .read_to_string(&mut content)
                .map_err(|e| format!("Failed reading nuspec entry: {e}"))?;

            let replacement = format!("<version>{}</version>", request.next_version);
            let updated = version_re.replace(&content, replacement).to_string();

            writer
                .write_all(updated.as_bytes())
                .map_err(|e| format!("Failed writing updated nuspec: {e}"))?;
        } else {
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| format!("Failed reading package entry: {e}"))?;
            writer
                .write_all(&buf)
                .map_err(|e| format!("Failed writing package entry: {e}"))?;
        }
    }

    writer
        .finish()
        .map_err(|e| format!("Failed finalizing temporary package: {e}"))?;

    let target_path = request.destination_path.join(&request.destination_file_name);
    fs::copy(&temp_path, &target_path).map_err(|e| {
        format!(
            "Failed copying package to destination {}: {e}",
            target_path.display()
        )
    })?;
    let _ = fs::remove_file(temp_path);

    Ok(target_path)
}

#[cfg(test)]
mod tests {
    use super::{is_package_file, next_version_from_known_versions};
    use semver::Version;
    use std::path::Path;

    #[test]
    fn bumps_patch_when_no_destination_versions() {
        assert_eq!(next_version_from_known_versions("1.2.3", None, None), "1.2.4");
    }

    #[test]
    fn bumps_above_highest_destination_version() {
        let highest = Version::parse("1.2.9").ok();
        assert_eq!(next_version_from_known_versions("1.2.3", highest, None), "1.2.10");
    }

    #[test]
    fn handles_non_semver_fallback() {
        assert_eq!(next_version_from_known_versions("custom", None, None), "custom.1");
    }

    #[test]
    fn starts_from_configured_version_for_first_local_copy() {
        assert_eq!(
            next_version_from_known_versions("1.2.3", None, Some("2.0.0")),
            "2.0.0"
        );
    }

    #[test]
    fn still_increments_when_existing_local_versions_present() {
        let highest = Version::parse("2.0.5").ok();
        assert_eq!(
            next_version_from_known_versions("1.2.3", highest, Some("2.0.0")),
            "2.0.6"
        );
    }

    #[test]
    fn package_extension_matching_is_case_insensitive() {
        assert!(is_package_file(Path::new("A.1.0.0.nupkg")));
        assert!(is_package_file(Path::new("A.1.0.0.NUPKG")));
        assert!(is_package_file(Path::new("A.1.0.0.NuGeT")));
        assert!(!is_package_file(Path::new("A.1.0.0.zip")));
    }
}

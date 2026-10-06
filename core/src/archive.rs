//! Stateless ZIP operations. Browser persistence is performed only after verification.
mod zip_directory;

use crate::model::{ArchiveAssetBuffer, ArchiveEntry, ArchiveReply, ArchiveRequest};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

#[derive(Clone, Copy)]
struct Limits {
    compressed: usize,
    project: usize,
    entry: usize,
    total: usize,
    entries: usize,
}

const MIB: usize = 1024 * 1024;
const LIMITS: Limits = Limits {
    compressed: 128 * MIB,
    project: 8 * MIB,
    entry: 64 * MIB,
    total: 256 * MIB,
    entries: 256,
};
const SIZE_ERROR: &str = "Project archive exceeds size limit";
type ArchiveResult = Result<(ArchiveReply, Vec<Vec<u8>>), String>;

pub fn request(metadata: &str, buffers: &[Vec<u8>]) -> (String, Vec<Vec<u8>>) {
    let result = serde_json::from_str(metadata)
        .map_err(|error| error.to_string())
        .and_then(|request| dispatch(request, buffers, LIMITS));
    let (reply, buffers) =
        result.unwrap_or_else(|message| (ArchiveReply::Error { message }, vec![]));
    (
        serde_json::to_string(&reply).expect("archive reply serializes"),
        buffers,
    )
}

fn dispatch(request: ArchiveRequest, buffers: &[Vec<u8>], limits: Limits) -> ArchiveResult {
    match request {
        ArchiveRequest::PackProject {
            project_json,
            archive_json,
            assets,
        } => {
            if project_json.len() > limits.project {
                return Err(SIZE_ERROR.into());
            }
            let references = asset_references(&project_json)?;
            let mut files = vec![("project.json", project_json.as_bytes())];
            if let Some(metadata) = &archive_json {
                files.push(("archive.json", metadata.as_bytes()));
            }
            for entry in &assets {
                if !asset_path(&entry.path) {
                    return Err("Invalid asset path".into());
                }
                files.push((&entry.path, buffer(buffers, entry.buffer_index)?));
            }
            check_project_files(&files, limits)?;
            let indexed: BTreeMap<_, _> = files.iter().copied().collect();
            verify_references(&references, &indexed)?;
            for (name, bytes) in &files {
                if asset_path(name) && digest(bytes) != name[7..] {
                    return Err(format!("Asset hash mismatch: {name}"));
                }
            }
            let bytes = pack(&files, Some(limits.compressed))?;
            Ok((ArchiveReply::Packed, vec![bytes]))
        }
        ArchiveRequest::PackFiles { entries } => {
            let files = entry_buffers(&entries, buffers)?;
            Ok((ArchiveReply::Packed, vec![pack(&files, None)?]))
        }
        ArchiveRequest::UnpackProject => {
            if buffers.len() != 1 {
                return Err("Expected one project archive buffer".into());
            }
            let mut files = unpack(&buffers[0], limits)?;
            let project_json = String::from_utf8(
                files
                    .remove("project.json")
                    .ok_or("Archive has no project.json")?,
            )
            .map_err(|_| "Project JSON is not UTF-8")?;
            let references = asset_references(&project_json)?;
            let (project_json, _) = crate::migrate::migrate_json(&project_json)?;
            let indexed = files
                .iter()
                .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
                .collect();
            verify_references(&references, &indexed)?;
            let mut assets = Vec::new();
            let mut outputs = Vec::new();
            let mut seen = BTreeSet::new();
            for (hash, _) in references {
                if seen.insert(hash.clone()) {
                    let bytes = files
                        .remove(&format!("assets/{hash}"))
                        .expect("verified asset exists");
                    assets.push(ArchiveAssetBuffer {
                        sha256: hash,
                        buffer_index: outputs.len() as u32,
                    });
                    outputs.push(bytes);
                }
            }
            Ok((
                ArchiveReply::Unpacked {
                    project_json,
                    assets,
                },
                outputs,
            ))
        }
    }
}

fn buffer(buffers: &[Vec<u8>], index: u32) -> Result<&[u8], String> {
    buffers
        .get(index as usize)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("Missing archive buffer: {index}"))
}

fn entry_buffers<'a>(
    entries: &'a [ArchiveEntry],
    buffers: &'a [Vec<u8>],
) -> Result<Vec<(&'a str, &'a [u8])>, String> {
    entries
        .iter()
        .map(|entry| Ok((entry.path.as_str(), buffer(buffers, entry.buffer_index)?)))
        .collect()
}

fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn asset_path(path: &str) -> bool {
    path.strip_prefix("assets/").is_some_and(valid_hash)
}

fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', '\0', ':'])
        && path
            .split('/')
            .all(|component| !matches!(component, "" | "." | ".."))
}

fn asset_references(json: &str) -> Result<Vec<(String, String)>, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|error| format!("Invalid project JSON: {error}"))?;
    if value.get("format").and_then(|item| item.as_str()) != Some("boardstudio/v2")
        || !value.get("parts").is_some_and(serde_json::Value::is_array)
        || !value.get("boards").is_some_and(serde_json::Value::is_array)
    {
        return Err("Unsupported project format".into());
    }
    let assets = value
        .get("assets")
        .and_then(|item| item.as_array())
        .ok_or("Project assets must be an array")?;
    assets
        .iter()
        .map(|asset| {
            let hash = asset
                .get("sha256")
                .and_then(|item| item.as_str())
                .filter(|hash| valid_hash(hash))
                .ok_or("Invalid asset hash")?;
            let name = asset
                .get("name")
                .and_then(|item| item.as_str())
                .unwrap_or(hash);
            Ok((hash.to_owned(), name.to_owned()))
        })
        .collect()
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn verify_references(
    references: &[(String, String)],
    files: &BTreeMap<&str, &[u8]>,
) -> Result<(), String> {
    let mut verified = BTreeSet::new();
    for (hash, name) in references {
        if !verified.insert(hash) {
            continue;
        }
        let bytes = files
            .get(format!("assets/{hash}").as_str())
            .ok_or_else(|| format!("Archive has no asset: {name}"))?;
        if digest(bytes) != *hash {
            return Err(format!("Asset hash mismatch: {name}"));
        }
    }
    Ok(())
}

fn project_entry_limit(name: &str, limits: Limits) -> Result<usize, String> {
    match name {
        "project.json" => Ok(limits.project),
        "archive.json" => Ok(limits.entry),
        _ if asset_path(name) => Ok(limits.entry),
        _ => Err("Project archive contains an unknown entry".into()),
    }
}

fn check_project_files(files: &[(&str, &[u8])], limits: Limits) -> Result<(), String> {
    if files.len() > limits.entries {
        return Err(SIZE_ERROR.into());
    }
    let mut total = 0usize;
    let mut names = BTreeSet::new();
    for (name, bytes) in files {
        if !names.insert(name) {
            return Err("Duplicate archive entry".into());
        }
        total = total.checked_add(bytes.len()).ok_or(SIZE_ERROR)?;
        if bytes.len() > project_entry_limit(name, limits)? || total > limits.total {
            return Err(SIZE_ERROR.into());
        }
    }
    Ok(())
}

fn pack(files: &[(&str, &[u8])], limit: Option<usize>) -> Result<Vec<u8>, String> {
    let mut names = BTreeSet::new();
    for (name, _) in files {
        if !safe_path(name) {
            return Err("Unsafe archive path".into());
        }
        if !names.insert(name) {
            return Err("Duplicate archive entry".into());
        }
    }
    let output = BoundedWriter {
        inner: Cursor::new(Vec::new()),
        limit,
    };
    let mut writer = ZipWriter::new(output);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, bytes) in files {
        writer
            .start_file(*name, options)
            .map_err(|error| error.to_string())?;
        writer.write_all(bytes).map_err(|error| error.to_string())?;
    }
    Ok(writer
        .finish()
        .map_err(|error| error.to_string())?
        .inner
        .into_inner())
}

struct BoundedWriter {
    inner: Cursor<Vec<u8>>,
    limit: Option<usize>,
}

impl Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.limit.is_some_and(|limit| {
            self.inner
                .position()
                .checked_add(bytes.len() as u64)
                .is_none_or(|end| end > limit as u64)
        }) {
            return Err(std::io::Error::other(SIZE_ERROR));
        }
        self.inner.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl std::io::Seek for BoundedWriter {
    fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
        std::io::Seek::seek(&mut self.inner, position)
    }
}

fn unpack(bytes: &[u8], limits: Limits) -> Result<BTreeMap<String, Vec<u8>>, String> {
    if bytes.len() > limits.compressed {
        return Err(SIZE_ERROR.into());
    }
    let entries = zip_directory::inspect(bytes, limits.entries)?;
    let mut declared_total = 0u64;
    for entry in &entries {
        declared_total = declared_total
            .checked_add(entry.uncompressed_size)
            .ok_or(SIZE_ERROR)?;
        if entry.uncompressed_size > project_entry_limit(&entry.name, limits)? as u64
            || declared_total > limits.total as u64
        {
            return Err(SIZE_ERROR.into());
        }
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|error| error.to_string())?;
    if archive.len() != entries.len() {
        return Err("Archive directory count mismatch".into());
    }
    let mut files = BTreeMap::new();
    let mut total = 0usize;
    for (index, entry) in entries.iter().enumerate() {
        let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
        if file.name_raw() != entry.name.as_bytes()
            || file.size() != entry.uncompressed_size
            || file.compressed_size() != entry.compressed_size
            || file.data_start() != Some(entry.data_start)
        {
            return Err("Archive directory mismatch".into());
        }
        let limit = project_entry_limit(&entry.name, limits)?;
        let data = read_bounded(&mut file, limit, &mut total, limits.total)?;
        if data.len() as u64 != entry.uncompressed_size {
            return Err("Archive decoded size mismatch".into());
        }
        files.insert(entry.name.clone(), data);
    }
    Ok(files)
}

fn read_bounded(
    reader: &mut impl Read,
    limit: usize,
    total: &mut usize,
    total_limit: usize,
) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 16 * 1024];
    loop {
        // Read one byte beyond the remaining allowance to detect overflow without
        // allocating from untrusted size declarations or draining a ZIP bomb.
        let remaining = limit
            .saturating_sub(data.len())
            .min(total_limit.saturating_sub(*total));
        let length = chunk.len().min(remaining.saturating_add(1));
        let read = reader
            .read(&mut chunk[..length])
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        if read > remaining {
            return Err(SIZE_ERROR.into());
        }
        data.extend_from_slice(&chunk[..read]);
        *total += read;
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn document(hashes: &[String]) -> String {
        json!({"format":"boardstudio/v2","formatVersion":2,"parts":[],"boards":[],"assets":hashes.iter().map(|hash| json!({"id":hash,"name":"fixture.step","sha256":hash})).collect::<Vec<_>>(),"future":{"keep":true}}).to_string()
    }

    fn pack_project(json: String, files: &[(String, Vec<u8>)], limits: Limits) -> ArchiveResult {
        dispatch(
            ArchiveRequest::PackProject {
                project_json: json,
                archive_json: Some("informational, not required to be JSON".into()),
                assets: files
                    .iter()
                    .enumerate()
                    .map(|(index, (path, _))| ArchiveEntry {
                        path: path.clone(),
                        buffer_index: index as u32,
                    })
                    .collect(),
            },
            &files
                .iter()
                .map(|(_, bytes)| bytes.clone())
                .collect::<Vec<_>>(),
            limits,
        )
    }

    #[test]
    fn unpacking_a_version_1_project_migrates_it() {
        let json = json!({
            "format": "boardstudio/v2", "parts": [], "boards": [], "assets": [],
            "definitions": [{ "id": "ergogen:ceoloide/switch_mx" }],
        })
        .to_string();
        let (_, packed) = pack_project(json, &[], LIMITS).unwrap();
        let (reply, _) = dispatch(ArchiveRequest::UnpackProject, &packed, LIMITS).unwrap();
        let ArchiveReply::Unpacked { project_json, .. } = reply else {
            panic!("expected project")
        };
        let project: serde_json::Value = serde_json::from_str(&project_json).unwrap();
        assert_eq!(project["formatVersion"], 2);
        assert_eq!(project["definitions"][0]["id"], "generator:ceoloide/switch_mx");
    }

    #[test]
    fn unpacking_a_project_from_a_newer_version_is_rejected() {
        let json = json!({
            "format": "boardstudio/v2", "formatVersion": 99, "parts": [], "boards": [], "assets": [],
        })
        .to_string();
        let (_, packed) = pack_project(json, &[], LIMITS).unwrap();
        let error = dispatch(ArchiveRequest::UnpackProject, &packed, LIMITS).unwrap_err();
        assert!(error.contains("newer than this app supports"), "{error}");
    }

    #[test]
    fn project_roundtrip_keeps_json_and_deduplicates_shared_references() {
        let data = b"component STEP".to_vec();
        let hash = digest(&data);
        let json = format!("  {}\n", document(&[hash.clone(), hash.clone()]));
        let (_, packed) = pack_project(
            json.clone(),
            &[(format!("assets/{hash}"), data.clone())],
            LIMITS,
        )
        .unwrap();
        let (reply, unpacked) = dispatch(ArchiveRequest::UnpackProject, &packed, LIMITS).unwrap();
        let ArchiveReply::Unpacked {
            project_json,
            assets,
        } = reply
        else {
            panic!("expected project")
        };
        assert_eq!(project_json, json);
        assert_eq!(
            assets,
            vec![ArchiveAssetBuffer {
                sha256: hash,
                buffer_index: 0
            }]
        );
        assert_eq!(unpacked, vec![data]);
    }

    #[test]
    fn pack_verifies_missing_hash_mismatch_and_duplicate_assets() {
        let bytes = b"asset".to_vec();
        let hash = digest(&bytes);
        let entry = (format!("assets/{hash}"), bytes);
        let json = document(std::slice::from_ref(&hash));
        assert!(
            pack_project(json.clone(), &[], LIMITS)
                .unwrap_err()
                .contains("no asset")
        );
        assert!(
            pack_project(
                json.clone(),
                &[(entry.0.clone(), b"wrong".to_vec())],
                LIMITS
            )
            .unwrap_err()
            .contains("hash mismatch")
        );
        assert!(
            pack_project(json, &[entry.clone(), entry], LIMITS)
                .unwrap_err()
                .contains("Duplicate")
        );
    }

    #[test]
    fn unpack_verifies_missing_assets_and_hashes_before_any_output() {
        let data = b"asset";
        let hash = digest(data);
        let json = document(std::slice::from_ref(&hash));
        let missing = pack(&[("project.json", json.as_bytes())], None).unwrap();
        assert!(
            dispatch(ArchiveRequest::UnpackProject, &[missing], LIMITS)
                .unwrap_err()
                .contains("no asset")
        );
        let wrong = pack(
            &[
                ("project.json", json.as_bytes()),
                (&format!("assets/{hash}"), b"wrong"),
            ],
            None,
        )
        .unwrap();
        let (reply, buffers) = request(r#"{"kind":"unpack-project"}"#, &[wrong]);
        assert!(reply.contains("hash mismatch"));
        assert!(buffers.is_empty());
    }

    #[test]
    fn unreferenced_assets_are_checked_for_archive_integrity_but_not_returned() {
        let hash = digest(b"unused");
        let (_, packed) = pack_project(
            document(&[]),
            &[(format!("assets/{hash}"), b"unused".to_vec())],
            LIMITS,
        )
        .unwrap();
        let (reply, outputs) = dispatch(ArchiveRequest::UnpackProject, &packed, LIMITS).unwrap();
        assert!(matches!(reply, ArchiveReply::Unpacked { assets, .. } if assets.is_empty()));
        assert!(outputs.is_empty());
    }

    #[test]
    fn limits_apply_to_pack_and_unpack_including_total_and_compressed_bytes() {
        let json = document(&[]);
        let (_, packed) = pack_project(json.clone(), &[], LIMITS).unwrap();
        for limits in [
            Limits {
                project: json.len() - 1,
                ..LIMITS
            },
            Limits {
                total: json.len() - 1,
                ..LIMITS
            },
            Limits { entry: 4, ..LIMITS },
            Limits {
                entries: 1,
                ..LIMITS
            },
            Limits {
                compressed: 10,
                ..LIMITS
            },
        ] {
            assert!(pack_project(json.clone(), &[], limits).is_err());
            assert!(dispatch(ArchiveRequest::UnpackProject, &packed, limits).is_err());
        }
    }

    #[test]
    fn actual_read_limits_stop_at_the_first_excess_byte() {
        let mut total = 0;
        let mut reader = Cursor::new(vec![1; 20]);
        assert!(
            read_bounded(&mut reader, 8, &mut total, 100)
                .unwrap_err()
                .contains("size limit")
        );
        assert_eq!(reader.position(), 9);
        let mut total = 7;
        let mut reader = Cursor::new(vec![1; 20]);
        assert!(read_bounded(&mut reader, 100, &mut total, 10).is_err());
        assert_eq!(reader.position(), 4);
    }

    #[test]
    fn generic_exports_preserve_paths_and_have_no_project_numeric_limits() {
        let entries = (0..257)
            .map(|index| ArchiveEntry {
                path: format!("models/{index}.step"),
                buffer_index: 0,
            })
            .collect();
        let (_, outputs) = dispatch(
            ArchiveRequest::PackFiles { entries },
            &[b"data".to_vec()],
            Limits {
                entries: 1,
                compressed: 1,
                entry: 1,
                total: 1,
                project: 1,
            },
        )
        .unwrap();
        let mut zip = ZipArchive::new(Cursor::new(&outputs[0])).unwrap();
        assert_eq!(zip.len(), 257);
        assert_eq!(zip.by_name("models/256.step").unwrap().size(), 4);
        assert!(pack(&[("BoardStudio.pretty/part..name.kicad_mod", b"a")], None).is_ok());
        for name in ["../x", "/x", "C:/x", "a/../x", "a\\x", "x/", "./x"] {
            assert!(pack(&[(name, b"a")], None).is_err(), "{name}");
        }
    }

    #[test]
    fn project_rejects_unknown_paths_uppercase_hashes_and_bad_envelopes() {
        let json = document(&[]);
        for name in [
            "extra",
            "assets/not-a-hash",
            &format!("assets/{}", "A".repeat(64)),
        ] {
            let bytes = pack(&[("project.json", json.as_bytes()), (name, b"data")], None).unwrap();
            assert!(dispatch(ArchiveRequest::UnpackProject, &[bytes], LIMITS).is_err());
        }
        for json in [
            "{",
            r#"{"format":"boardstudio/v1","parts":[],"boards":[],"assets":[]}"#,
            r#"{"format":"boardstudio/v2","parts":[],"boards":[],"assets":null}"#,
        ] {
            assert!(pack_project(json.into(), &[], LIMITS).is_err());
        }
    }

    #[test]
    fn stored_and_deflated_crc_corruption_is_rejected() {
        let json = document(&[]);
        for method in [CompressionMethod::Stored, CompressionMethod::Deflated] {
            let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
            writer
                .start_file(
                    "project.json",
                    SimpleFileOptions::default().compression_method(method),
                )
                .unwrap();
            writer.write_all(json.as_bytes()).unwrap();
            let mut bytes = writer.finish().unwrap().into_inner();
            let entries = zip_directory::inspect(&bytes, 256).unwrap();
            assert!(dispatch(ArchiveRequest::UnpackProject, &[bytes.clone()], LIMITS).is_ok());
            bytes[entries[0].data_start as usize] ^= 1;
            assert!(dispatch(ArchiveRequest::UnpackProject, &[bytes], LIMITS).is_err());
        }
    }
}

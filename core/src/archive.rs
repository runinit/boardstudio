//! Stateless ZIP operations. Browser persistence is performed only after verification.
mod zip_directory;

use crate::hash::sha256_hex;
use crate::model::{ArchiveAssetBuffer, ArchiveEntry, ArchiveReply, ArchiveRequest};
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
/// Maximum compressed project archive accepted by Core and browser file admission.
pub const MAX_PROJECT_ARCHIVE_BYTES: usize = 128 * MIB;
const LIMITS: Limits = Limits {
    compressed: MAX_PROJECT_ARCHIVE_BYTES,
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
                if asset_path(name) && sha256_hex(bytes) != name[7..] {
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
            // Inspect the version before any schema-dependent interpretation.
            let (project_json, _) = crate::migrate::migrate_json(&project_json)?;
            let references = asset_references(&project_json)?;
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

// The format check stays: PackProject calls this without a prior version gate.
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
        if sha256_hex(bytes) != *hash {
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
    let output = BoundedWriter::new(limit);
    let mut writer = ZipWriter::new(output);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, bytes) in files {
        writer
            .start_file(*name, options)
            .map_err(|error| error.to_string())?;
        writer.write_all(bytes).map_err(|error| error.to_string())?;
    }
    let output = writer.finish().map_err(|error| error.to_string())?;
    if output.exhausted {
        return Err(SIZE_ERROR.into());
    }
    Ok(output.inner.into_inner())
}

/// A size-bounded sink with sticky exhaustion.
///
/// The first write past the limit returns the size error. Later writes are discarded
/// while advancing a logical position, so `ZipWriter`'s fallible Drop finalization
/// succeeds against the exhausted sink and never writes its diagnostic to process
/// stderr. The backing buffer is never grown by those teardown writes.
struct BoundedWriter {
    inner: Cursor<Vec<u8>>,
    limit: Option<usize>,
    position: u64,
    length: u64,
    exhausted: bool,
}

impl BoundedWriter {
    fn new(limit: Option<usize>) -> Self {
        Self {
            inner: Cursor::new(Vec::new()),
            limit,
            position: 0,
            length: 0,
            exhausted: false,
        }
    }
}

impl Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let len = bytes.len() as u64;
        if self.exhausted {
            self.position = self.position.saturating_add(len);
            self.length = self.length.max(self.position);
            return Ok(bytes.len());
        }
        let end = self.position.checked_add(len).ok_or_else(|| {
            self.exhausted = true;
            std::io::Error::other(SIZE_ERROR)
        })?;
        if self.limit.is_some_and(|limit| end > limit as u64) {
            self.exhausted = true;
            return Err(std::io::Error::other(SIZE_ERROR));
        }
        std::io::Seek::seek(&mut self.inner, std::io::SeekFrom::Start(self.position))?;
        let written = self.inner.write(bytes)?;
        self.position = self.position.saturating_add(written as u64);
        self.length = self.length.max(self.position);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl std::io::Seek for BoundedWriter {
    fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
        let target = match position {
            std::io::SeekFrom::Start(offset) => offset,
            std::io::SeekFrom::Current(offset) => self
                .position
                .checked_add_signed(offset)
                .ok_or_else(|| std::io::Error::other("Archive position overflow"))?,
            std::io::SeekFrom::End(offset) => self
                .length
                .checked_add_signed(offset)
                .ok_or_else(|| std::io::Error::other("Archive position overflow"))?,
        };
        self.position = target;
        Ok(target)
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
    use crate::migrate::CURRENT_VERSION;
    use serde_json::json;

    fn document(hashes: &[String]) -> String {
        json!({"format":"boardstudio/v2","formatVersion":CURRENT_VERSION,"parts":[],"boards":[],"assets":hashes.iter().map(|hash| json!({"id":hash,"name":"fixture.step","sha256":hash})).collect::<Vec<_>>(),"future":{"keep":true}}).to_string()
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
    fn unpacking_an_unversioned_project_refuses_without_conversion() {
        let json = json!({
            "format": "boardstudio/v2", "parts": [], "boards": [], "assets": [],
            "definitions": [{ "id": "ergogen:ceoloide/switch_mx" }],
        })
        .to_string();
        let (_, packed) = pack_project(json, &[], LIMITS).unwrap();
        let error = dispatch(ArchiveRequest::UnpackProject, &packed, LIMITS).unwrap_err();
        assert!(
            error.contains("legacy") && error.contains("No conversion"),
            "{error}"
        );
    }

    #[test]
    fn unpacking_refuses_every_version_before_reading_the_asset_table() {
        // Each project has a malformed asset table; the version gate must win.
        for (version, explanation) in [
            (
                Some(json!(CURRENT_VERSION - 1)),
                "No conversion is available",
            ),
            (Some(json!(CURRENT_VERSION + 1)), "compatible newer build"),
            (Some(json!(0)), "invalid formatVersion"),
            (Some(json!("7")), "invalid formatVersion"),
            (Some(json!(7.5)), "invalid formatVersion"),
            (Some(json!(null)), "invalid formatVersion"),
            (None, "no formatVersion"),
        ] {
            let mut document = json!({"format": "boardstudio/v2", "assets": "broken"});
            if let Some(version) = version {
                document["formatVersion"] = version;
            }
            let packed =
                vec![pack(&[("project.json", document.to_string().as_bytes())], None).unwrap()];
            let error = dispatch(ArchiveRequest::UnpackProject, &packed, LIMITS).unwrap_err();
            assert!(error.contains(explanation), "{document}: {error}");
        }
    }

    #[test]
    fn unpacking_an_unrecognized_format_is_unsupported_without_conversion() {
        let json = json!({"format": "boardstudio/v1", "formatVersion": CURRENT_VERSION - 1,
            "parts": [], "boards": [], "assets": []})
        .to_string();
        let packed = vec![pack(&[("project.json", json.as_bytes())], None).unwrap()];
        let error = dispatch(ArchiveRequest::UnpackProject, &packed, LIMITS).unwrap_err();
        assert!(error.contains("Unsupported document format"), "{error}");
    }

    #[test]
    fn unpacking_malformed_json_text_is_refused_as_invalid_project_json() {
        let packed = vec![pack(&[("project.json", b"{not json")], None).unwrap()];
        let error = dispatch(ArchiveRequest::UnpackProject, &packed, LIMITS).unwrap_err();
        assert!(error.contains("Invalid project JSON"), "{error}");
    }

    #[test]
    fn unpacking_a_project_from_a_newer_version_is_rejected() {
        let json = json!({
            "format": "boardstudio/v2", "formatVersion": 99, "parts": [], "boards": [], "assets": [],
        })
        .to_string();
        let (_, packed) = pack_project(json, &[], LIMITS).unwrap();
        let error = dispatch(ArchiveRequest::UnpackProject, &packed, LIMITS).unwrap_err();
        assert!(error.contains("compatible newer build"), "{error}");
    }

    #[test]
    fn project_roundtrip_keeps_json_and_deduplicates_shared_references() {
        let data = b"component STEP".to_vec();
        let hash = sha256_hex(&data);
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
    fn version_5_mirrored_halves_are_refused_without_rewriting_their_payload() {
        let json = json!({
            "format": "boardstudio/v2", "formatVersion": 5, "parts": [], "boards": [], "assets": [],
            "hardware": { "pairs": [{
                "id": "pair", "members": ["left", "right"],
                "mapping": { "at": { "x": 200.0, "y": 0.0 }, "rotation": 0.0 },
                "construction": { "method": "printed", "mount": "tray", "integratedPlateFrame": false,
                    "plateThickness": 1.3, "plateFoamThickness": 2.0, "bottomFoamThickness": 2.0,
                    "bottomThickness": 3.0, "plateToPcb": 2.2, "wallThickness": 2.0, "clearance": 0.3 },
                "constructionSource": "left",
                "halves": { "sourceBoardId": "left-board", "mirrorBoardId": "right-board",
                    "correspondences": [{ "kind": "part", "sourceId": "l-key", "mirrorId": "r-key" }],
                    "exceptions": [{ "kind": "part", "mirrorId": "r-extra", "state": "omitted" }] }
            }] },
            "future": { "keep": true }
        })
        .to_string();
        let (_, packed) = pack_project(json.clone(), &[], LIMITS).unwrap();
        let error = dispatch(ArchiveRequest::UnpackProject, &packed, LIMITS).unwrap_err();
        assert!(
            error.contains("format 5") && error.contains("No conversion"),
            "{error}"
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&json).unwrap()["formatVersion"],
            5
        );
    }

    #[test]
    fn framed_endpoint_relations_cross_the_archive_boundary_without_reinterpretation() {
        let json = json!({
            "format": "boardstudio/v2", "formatVersion": CURRENT_VERSION,
            "constraints": [
                {"kind":"framed", "id":"c/offset",
                 "source":{"kind":"part", "partId":"left/source"},
                 "target":{"kind":"linked-mirror", "pairId":"pair", "sourcePartId":"left/target", "mirrorPartId":"right/target"},
                 "relation":{"kind":"offset", "offset":{"x":5.0,"y":-2.0},"rotation":15.0}},
                {"kind":"framed", "id":"c/part-offset",
                 "source":{"kind":"linked-mirror", "pairId":"pair", "sourcePartId":"left/source", "mirrorPartId":"right/source"},
                 "target":{"kind":"part", "partId":"local/target"},
                 "relation":{"kind":"part-offset", "offset":{"x":-3.0,"y":8.0},"rotation":-22.0}},
                {"kind":"framed", "id":"c/mirror-line",
                 "source":{"kind":"part", "partId":"local/source"},
                 "target":{"kind":"part", "partId":"local/target-2"},
                 "relation":{"kind":"mirror-line", "at":{"x":2.25,"y":-5.5},"angle":37.0}}
            ],
            "parts": [], "boards": [], "assets": []
        })
        .to_string();
        let (_, packed) = pack_project(json.clone(), &[], LIMITS).unwrap();

        let (reply, _) = dispatch(ArchiveRequest::UnpackProject, &packed, LIMITS).unwrap();

        let ArchiveReply::Unpacked { project_json, .. } = reply else {
            panic!("expected project")
        };
        assert_eq!(project_json, json);
    }

    #[test]
    fn pack_verifies_missing_hash_mismatch_and_duplicate_assets() {
        let bytes = b"asset".to_vec();
        let hash = sha256_hex(&bytes);
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
        let hash = sha256_hex(data);
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
        let hash = sha256_hex(b"unused");
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

    /// Child-process probe: packs one limit case and reports the error on stdout, so the
    /// parent can inspect the process's direct stderr separately from libtest capture.
    #[test]
    #[ignore = "spawned by the archive teardown regression tests"]
    fn archive_child_probe() {
        let Ok(case) = std::env::var("BOARDSTUDIO_ARCHIVE_LIMIT_CASE") else {
            return;
        };
        let incompressible: Vec<u8> = (0..4096).map(|index| (index % 251) as u8).collect();
        let result = match case.as_str() {
            "header" => pack(&[("project.json", b"payload")], Some(0)),
            "data" => pack(
                &[("project.json", &incompressible), ("second.txt", b"next")],
                Some(80),
            ),
            "central-directory" => pack(&[("project.json", b"hello")], Some(80)),
            other => {
                eprintln!("unknown archive limit case {other}");
                std::process::exit(2);
            }
        };
        match result {
            Ok(_) => {
                eprintln!("an over-limit pack returned an archive");
                std::process::exit(4);
            }
            Err(message) => {
                println!("ARCHIVE_RESULT={message}");
                if !message.contains(SIZE_ERROR) {
                    std::process::exit(3);
                }
            }
        }
    }

    fn archive_child(case: &str) -> std::process::Output {
        std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("archive::tests::archive_child_probe")
            .arg("--ignored")
            .arg("--nocapture")
            .env("BOARDSTUDIO_ARCHIVE_LIMIT_CASE", case)
            .output()
            .unwrap()
    }

    fn assert_clean_limit_failure(case: &str) {
        let output = archive_child(case);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "{case}: child failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        assert!(
            stdout.contains("ARCHIVE_RESULT=") && stdout.contains(SIZE_ERROR),
            "{case}: the size-limit error was not reported\nstdout:\n{stdout}"
        );
        assert!(
            !stderr.contains("ZipWriter::drop"),
            "{case}: ZipWriter wrote a Drop diagnostic to stderr:\n{stderr}"
        );
        assert!(
            !stderr.contains(SIZE_ERROR),
            "{case}: the Drop diagnostic leaked the production error:\n{stderr}"
        );
    }

    #[test]
    fn bounded_writer_discards_teardown_writes_without_growing_past_the_cap() {
        let mut writer = BoundedWriter::new(Some(8));
        writer.write_all(b"12345").unwrap();
        assert!(writer.write_all(b"6789").is_err());
        assert_eq!(
            writer.inner.get_ref().len(),
            5,
            "the refused write grew the buffer"
        );

        // Teardown writes after exhaustion are discarded but keep logical positions
        // coherent, so a finalizing ZipWriter sees a successful sink.
        assert_eq!(
            std::io::Seek::seek(&mut writer, std::io::SeekFrom::Start(100)).unwrap(),
            100
        );
        writer.write_all(&[0; 10]).unwrap();
        assert_eq!(writer.inner.get_ref().len(), 5, "teardown grew the buffer");
        assert_eq!(
            std::io::Seek::seek(&mut writer, std::io::SeekFrom::End(0)).unwrap(),
            110
        );
        assert_eq!(
            std::io::Seek::seek(&mut writer, std::io::SeekFrom::Current(-10)).unwrap(),
            100
        );
        assert!(writer.exhausted);
    }

    #[test]
    fn limit_failures_do_not_write_zipwriter_drop_diagnostics() {
        for case in ["header", "data", "central-directory"] {
            assert_clean_limit_failure(case);
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

use super::*;
use std::io::{Cursor, Read, Write};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

fn archive(names: &[&str], payload: &[u8]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for name in names {
        writer
            .start_file(
                *name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(payload).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
fn set16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}
fn set32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn central(bytes: &[u8]) -> usize {
    dword(bytes, bytes.len() - 6).unwrap() as usize
}
fn decoded(bytes: &[u8]) -> Vec<u8> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut output = Vec::new();
    zip.by_index(0).unwrap().read_to_end(&mut output).unwrap();
    output
}

// Produce descriptor variants from a known-good ZIP, retaining CRC and content.
fn descriptor(signed: bool, wide: bool) -> Vec<u8> {
    let mut bytes = archive(&["project.json"], b"fixture content");
    let directory = central(&bytes);
    let crc = dword(&bytes, directory + 16).unwrap();
    let compressed = dword(&bytes, directory + 20).unwrap();
    let size = dword(&bytes, directory + 24).unwrap();
    set16(&mut bytes, 6, 8);
    set16(&mut bytes, directory + 8, 8);
    for at in [14, 18, 22] {
        set32(&mut bytes, at, 0);
    }
    let mut record = Vec::new();
    if signed {
        record.extend_from_slice(&0x08074b50u32.to_le_bytes());
    }
    record.extend_from_slice(&crc.to_le_bytes());
    if wide {
        record.extend_from_slice(&(compressed as u64).to_le_bytes());
        record.extend_from_slice(&(size as u64).to_le_bytes());
    } else {
        record.extend_from_slice(&compressed.to_le_bytes());
        record.extend_from_slice(&size.to_le_bytes());
    }
    let new_directory = directory + record.len();
    bytes.splice(directory..directory, record);
    let eocd = bytes.len() - 22;
    set32(&mut bytes, eocd + 16, new_directory as u32);
    bytes
}

// A tiny ZIP64 file exercises wide metadata without allocating a large payload.
fn zip64() -> Vec<u8> {
    let original = archive(&["project.json"], b"fixture content");
    let old_directory = central(&original);
    let old_eocd = original.len() - 22;
    let name_end = 30 + word(&original, 26).unwrap() as usize;
    assert_eq!(word(&original, 28).unwrap(), 0);
    let size = dword(&original, old_directory + 24).unwrap() as u64;
    let mut local = original[..name_end].to_vec();
    set32(&mut local, 18, u32::MAX);
    set32(&mut local, 22, u32::MAX);
    set16(&mut local, 28, 20);
    local.extend_from_slice(&1u16.to_le_bytes());
    local.extend_from_slice(&16u16.to_le_bytes());
    local.extend_from_slice(&size.to_le_bytes());
    local.extend_from_slice(&size.to_le_bytes());
    local.extend_from_slice(&original[name_end..old_directory]);
    let directory_start = local.len();
    let mut directory = original[old_directory..old_eocd].to_vec();
    assert_eq!(word(&directory, 30).unwrap(), 0);
    set32(&mut directory, 20, u32::MAX);
    set32(&mut directory, 24, u32::MAX);
    set32(&mut directory, 42, u32::MAX);
    set16(&mut directory, 34, u16::MAX);
    set16(&mut directory, 30, 32);
    directory.extend_from_slice(&1u16.to_le_bytes());
    directory.extend_from_slice(&28u16.to_le_bytes());
    directory.extend_from_slice(&size.to_le_bytes());
    directory.extend_from_slice(&size.to_le_bytes());
    directory.extend_from_slice(&0u64.to_le_bytes());
    directory.extend_from_slice(&0u32.to_le_bytes());
    local.extend_from_slice(&directory);
    let record_start = local.len();
    local.extend_from_slice(&0x06064b50u32.to_le_bytes());
    local.extend_from_slice(&44u64.to_le_bytes());
    local.extend_from_slice(&45u16.to_le_bytes());
    local.extend_from_slice(&45u16.to_le_bytes());
    local.extend_from_slice(&0u32.to_le_bytes());
    local.extend_from_slice(&0u32.to_le_bytes());
    local.extend_from_slice(&1u64.to_le_bytes());
    local.extend_from_slice(&1u64.to_le_bytes());
    local.extend_from_slice(&(directory.len() as u64).to_le_bytes());
    local.extend_from_slice(&(directory_start as u64).to_le_bytes());
    local.extend_from_slice(&0x07064b50u32.to_le_bytes());
    local.extend_from_slice(&0u32.to_le_bytes());
    local.extend_from_slice(&(record_start as u64).to_le_bytes());
    local.extend_from_slice(&1u32.to_le_bytes());
    let mut eocd = original[old_eocd..].to_vec();
    set16(&mut eocd, 8, u16::MAX);
    set16(&mut eocd, 10, u16::MAX);
    set32(&mut eocd, 12, u32::MAX);
    set32(&mut eocd, 16, u32::MAX);
    local.extend_from_slice(&eocd);
    local
}

#[test]
fn accepts_payload_signatures_without_scanning_them_as_records() {
    let payload = b"PK\x01\x02PK\x05\x06PK\x06\x07payload";
    let bytes = archive(&["project.json"], payload);
    assert_eq!(inspect(&bytes, 256).unwrap().len(), 1);
    assert_eq!(decoded(&bytes), payload);
}

#[test]
fn accepts_signed_unsigned_and_wide_descriptors() {
    for signed in [false, true] {
        for wide in [false, true] {
            let bytes = descriptor(signed, wide);
            assert_eq!(inspect(&bytes, 256).unwrap().len(), 1);
            assert_eq!(decoded(&bytes), b"fixture content");
            let mut broken = bytes.clone();
            let entry = inspect(&bytes, 256).unwrap().remove(0);
            broken[entry.data_start as usize
                + entry.compressed_size as usize
                + usize::from(signed) * 4] ^= 1;
            assert!(inspect(&broken, 256).is_err());
        }
    }
}

#[test]
fn accepts_bounded_zip64_and_rejects_invalid_wide_bounds() {
    let bytes = zip64();
    assert_eq!(inspect(&bytes, 256).unwrap()[0].uncompressed_size, 15);
    assert_eq!(decoded(&bytes), b"fixture content");
    let mut broken = bytes.clone();
    let locator = broken.len() - 42;
    broken[locator + 8..locator + 16].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(inspect(&broken, 256).is_err());
    let mut split = bytes.clone();
    set32(&mut split, locator + 16, 2);
    assert!(inspect(&split, 256).is_err());
}

#[test]
fn rejects_raw_duplicate_names_before_library_indexing() {
    let mut bytes = archive(&["project.json", "archive.json"], b"{}");
    // Equal-length names let us create duplicates without corrupting offsets.
    let from = b"archive.json";
    let positions: Vec<_> = bytes
        .windows(from.len())
        .enumerate()
        .filter_map(|(i, value)| (value == from).then_some(i))
        .collect();
    for position in positions {
        bytes[position..position + from.len()].copy_from_slice(b"project.json");
    }
    assert_eq!(
        ZipArchive::new(Cursor::new(&bytes)).unwrap().len(),
        1,
        "library collapses duplicates"
    );
    assert!(inspect(&bytes, 256).unwrap_err().contains("Duplicate"));
}

#[test]
fn rejects_counts_offsets_flags_methods_and_disagreeing_local_headers() {
    let original = archive(&["project.json"], b"{}");
    let directory = central(&original);
    let eocd = original.len() - 22;
    for (at, value) in [
        (eocd + 4, 1),
        (eocd + 8, 2),
        (directory + 8, 1),
        (directory + 10, 12),
        (directory + 34, 1),
        (6, 8),
    ] {
        let mut bytes = original.clone();
        set16(&mut bytes, at, value);
        assert!(inspect(&bytes, 256).is_err(), "accepted field {at}");
    }
    for at in [directory + 42, directory + 20, eocd + 12, eocd + 16] {
        let mut bytes = original.clone();
        set32(&mut bytes, at, u32::MAX - 1);
        assert!(inspect(&bytes, 256).is_err(), "accepted field {at}");
    }
    let mut wrong_name = original.clone();
    wrong_name[30] = b'X';
    assert!(inspect(&wrong_name, 256).is_err());
    assert!(inspect(&original, 0).is_err());
    for length in 0..original.len() {
        assert!(inspect(&original[..length], 256).is_err());
    }
}

#[test]
fn rejects_missing_zip64_values_and_directory_bytes_outside_records() {
    let mut wide = zip64();
    let local_extra = 30 + word(&wide, 26).unwrap() as usize;
    set16(&mut wide, local_extra + 2, 8);
    assert!(inspect(&wide, 256).is_err());
    let mut bytes = archive(&["project.json"], b"{}");
    let eocd = bytes.len() - 22;
    bytes.insert(eocd, 0);
    let new_eocd = eocd + 1;
    let size = dword(&bytes, new_eocd + 12).unwrap();
    set32(&mut bytes, new_eocd + 12, size + 1);
    assert!(inspect(&bytes, 256).is_err());
}

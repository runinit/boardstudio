//! Inspect raw ZIP records before the library indexes (and deduplicates) names.
use std::collections::BTreeSet;

#[derive(Debug)]
pub(super) struct Entry {
    pub(super) name: String,
    pub(super) compressed_size: u64,
    pub(super) uncompressed_size: u64,
    pub(super) data_start: u64,
}

fn range(bytes: &[u8], offset: usize, length: usize) -> Result<&[u8], String> {
    bytes
        .get(offset..offset.checked_add(length).ok_or("ZIP offset overflow")?)
        .ok_or_else(|| "Truncated ZIP record".into())
}
fn word(bytes: &[u8], offset: usize) -> Result<u16, String> {
    Ok(u16::from_le_bytes(
        range(bytes, offset, 2)?.try_into().unwrap(),
    ))
}
fn dword(bytes: &[u8], offset: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        range(bytes, offset, 4)?.try_into().unwrap(),
    ))
}
fn qword(bytes: &[u8], offset: usize) -> Result<u64, String> {
    Ok(u64::from_le_bytes(
        range(bytes, offset, 8)?.try_into().unwrap(),
    ))
}
fn index(value: u64) -> Result<usize, String> {
    usize::try_from(value).map_err(|_| "ZIP offset exceeds platform limits".into())
}
fn end(offset: usize, length: usize, limit: usize) -> Result<usize, String> {
    offset
        .checked_add(length)
        .filter(|end| *end <= limit)
        .ok_or_else(|| "ZIP record exceeds its bounds".into())
}

// Values in ZIP64 extras occur only for corresponding sentinel fields, in this order.
fn extended(extra: &[u8], required: [bool; 4]) -> Result<[u64; 4], String> {
    let mut offset = 0;
    let mut values = None;
    while offset < extra.len() {
        let tag = word(extra, offset)?;
        let length = word(extra, offset + 2)? as usize;
        offset += 4;
        let payload = range(extra, offset, length)?;
        if tag == 1 {
            if values.is_some() {
                return Err("Duplicate ZIP64 extra field".into());
            }
            let mut parsed = [0; 4];
            let mut cursor = 0;
            for (position, needed) in required.into_iter().enumerate() {
                if needed {
                    parsed[position] = if position == 3 {
                        dword(payload, cursor)? as u64
                    } else {
                        qword(payload, cursor)?
                    };
                    cursor += if position == 3 { 4 } else { 8 };
                }
            }
            values = Some(parsed);
        }
        offset += length;
    }
    if required.contains(&true) {
        values.ok_or_else(|| "Missing ZIP64 extra field".into())
    } else {
        Ok(values.unwrap_or([0; 4]))
    }
}

pub(super) fn inspect(bytes: &[u8], max_entries: usize) -> Result<Vec<Entry>, String> {
    if bytes.len() < 22 {
        return Err("ZIP end record not found".into());
    }
    let start = bytes.len().saturating_sub(22 + u16::MAX as usize);
    let eocd = (start..=bytes.len() - 22)
        .rev()
        .find(|&offset| {
            range(bytes, offset, 4).is_ok_and(|signature| signature == b"PK\x05\x06")
                && word(bytes, offset + 20)
                    .is_ok_and(|length| offset + 22 + length as usize == bytes.len())
        })
        .ok_or("ZIP end record not found")?;
    if word(bytes, eocd + 4)? != 0 || word(bytes, eocd + 6)? != 0 {
        return Err("Split ZIP archives are unsupported".into());
    }
    let disk_count = word(bytes, eocd + 8)? as u64;
    let count32 = word(bytes, eocd + 10)? as u64;
    let size32 = dword(bytes, eocd + 12)? as u64;
    let offset32 = dword(bytes, eocd + 16)? as u64;
    let required64 = disk_count == u16::MAX as u64
        || count32 == u16::MAX as u64
        || size32 == u32::MAX as u64
        || offset32 == u32::MAX as u64;
    let locator = eocd.checked_sub(20).filter(|&offset| {
        (required64 || offset32.checked_add(size32) != Some(eocd as u64))
            && range(bytes, offset, 4).is_ok_and(|signature| signature == b"PK\x06\x07")
    });
    let (count, directory_size, directory_offset, terminal_start) = if let Some(locator) = locator {
        if dword(bytes, locator + 4)? != 0 || dword(bytes, locator + 16)? != 1 {
            return Err("Split ZIP archives are unsupported".into());
        }
        let record = index(qword(bytes, locator + 8)?)?;
        if dword(bytes, record)? != 0x06064b50 {
            return Err("Invalid ZIP64 end record".into());
        }
        let size = index(qword(bytes, record + 4)?)?;
        if size < 44
            || end(
                record,
                size.checked_add(12).ok_or("ZIP64 size overflow")?,
                locator,
            )? != locator
        {
            return Err("Invalid ZIP64 end record bounds".into());
        }
        if dword(bytes, record + 16)? != 0 || dword(bytes, record + 20)? != 0 {
            return Err("Split ZIP archives are unsupported".into());
        }
        let count = qword(bytes, record + 32)?;
        let size = qword(bytes, record + 40)?;
        let offset = qword(bytes, record + 48)?;
        if qword(bytes, record + 24)? != count
            || (disk_count != u16::MAX as u64 && disk_count != count)
            || (count32 != u16::MAX as u64 && count32 != count)
            || (size32 != u32::MAX as u64 && size32 != size)
            || (offset32 != u32::MAX as u64 && offset32 != offset)
        {
            return Err("ZIP64 end records disagree".into());
        }
        (count, size, offset, record)
    } else {
        if required64 {
            return Err("ZIP64 locator missing".into());
        }
        if disk_count != count32 {
            return Err("ZIP entry counts disagree".into());
        }
        (count32, size32, offset32, eocd)
    };
    if count > max_entries as u64 {
        return Err("Project archive exceeds size limit".into());
    }
    let directory_start = index(directory_offset)?;
    let directory_end = end(directory_start, index(directory_size)?, terminal_start)?;
    if directory_end != terminal_start {
        return Err("ZIP central directory does not end at its end record".into());
    }
    let mut cursor = directory_start;
    let mut names = BTreeSet::new();
    let mut entries = Vec::with_capacity(count as usize);
    let mut spans = Vec::with_capacity(count as usize);
    for _ in 0..count {
        end(cursor, 46, directory_end)?;
        if dword(bytes, cursor)? != 0x02014b50 {
            return Err("Invalid central directory record".into());
        }
        let flags = word(bytes, cursor + 8)?;
        let method = word(bytes, cursor + 10)?;
        if flags & !0x080e != 0 {
            return Err("Encrypted or unsupported ZIP flags".into());
        }
        if method != 0 && method != 8 {
            return Err("Unsupported ZIP compression method".into());
        }
        let crc = dword(bytes, cursor + 16)?;
        let compressed32 = dword(bytes, cursor + 20)?;
        let uncompressed32 = dword(bytes, cursor + 24)?;
        let name_length = word(bytes, cursor + 28)? as usize;
        let extra_length = word(bytes, cursor + 30)? as usize;
        let comment_length = word(bytes, cursor + 32)? as usize;
        let disk32 = word(bytes, cursor + 34)?;
        let local32 = dword(bytes, cursor + 42)?;
        let name_end = end(cursor + 46, name_length, directory_end)?;
        let extra_end = end(name_end, extra_length, directory_end)?;
        let record_end = end(extra_end, comment_length, directory_end)?;
        let raw_name = &bytes[cursor + 46..name_end];
        if !names.insert(raw_name) {
            return Err("Duplicate archive entry".into());
        }
        let name = std::str::from_utf8(raw_name)
            .map_err(|_| "ZIP filename is not UTF-8")?
            .to_owned();
        let required = [
            uncompressed32 == u32::MAX,
            compressed32 == u32::MAX,
            local32 == u32::MAX,
            disk32 == u16::MAX,
        ];
        let wide = extended(&bytes[name_end..extra_end], required)?;
        let uncompressed = if required[0] {
            wide[0]
        } else {
            uncompressed32 as u64
        };
        let compressed = if required[1] {
            wide[1]
        } else {
            compressed32 as u64
        };
        let local = index(if required[2] { wide[2] } else { local32 as u64 })?;
        let disk = if required[3] { wide[3] } else { disk32 as u64 };
        if disk != 0 {
            return Err("Split ZIP archives are unsupported".into());
        }
        if method == 0 && compressed != uncompressed {
            return Err("Stored ZIP sizes disagree".into());
        }
        end(local, 30, directory_start)?;
        if dword(bytes, local)? != 0x04034b50 {
            return Err("Invalid local ZIP header".into());
        }
        if word(bytes, local + 6)? != flags || word(bytes, local + 8)? != method {
            return Err("Local ZIP flags or method disagree".into());
        }
        let local_name_end = end(
            local + 30,
            word(bytes, local + 26)? as usize,
            directory_start,
        )?;
        let data_start = end(
            local_name_end,
            word(bytes, local + 28)? as usize,
            directory_start,
        )?;
        if &bytes[local + 30..local_name_end] != raw_name {
            return Err("Local and central ZIP names disagree".into());
        }
        let local_compressed = dword(bytes, local + 18)?;
        let local_uncompressed = dword(bytes, local + 22)?;
        let local_required = [
            local_uncompressed == u32::MAX,
            local_compressed == u32::MAX,
            false,
            false,
        ];
        let local_wide = extended(&bytes[local_name_end..data_start], local_required)?;
        let local_uncompressed = if local_required[0] {
            local_wide[0]
        } else {
            local_uncompressed as u64
        };
        let local_compressed = if local_required[1] {
            local_wide[1]
        } else {
            local_compressed as u64
        };
        let local_crc = dword(bytes, local + 14)?;
        let data_end = end(data_start, index(compressed)?, directory_start)?;
        let descriptor = flags & 8 != 0;
        if (!descriptor
            && (local_crc != crc
                || local_compressed != compressed
                || local_uncompressed != uncompressed))
            || (descriptor
                && ((local_crc != 0 && local_crc != crc)
                    || (local_compressed != 0 && local_compressed != compressed)
                    || (local_uncompressed != 0 && local_uncompressed != uncompressed)))
        {
            return Err("Local ZIP sizes or checksum disagree".into());
        }
        let entry_end = if descriptor {
            descriptor_end(
                bytes,
                data_end,
                directory_start,
                crc,
                compressed,
                uncompressed,
            )?
        } else {
            data_end
        };
        spans.push((local, entry_end));
        entries.push(Entry {
            name,
            compressed_size: compressed,
            uncompressed_size: uncompressed,
            data_start: data_start as u64,
        });
        cursor = record_end;
    }
    if cursor != directory_end {
        return Err("ZIP directory count or size mismatch".into());
    }
    spans.sort_unstable();
    if spans.windows(2).any(|pair| pair[1].0 < pair[0].1) {
        return Err("Overlapping ZIP entries".into());
    }
    Ok(entries)
}

fn descriptor_end(
    bytes: &[u8],
    offset: usize,
    limit: usize,
    crc: u32,
    compressed: u64,
    uncompressed: u64,
) -> Result<usize, String> {
    for signed in [true, false] {
        let start = if signed {
            if dword(bytes, offset).ok() != Some(0x08074b50) {
                continue;
            }
            offset + 4
        } else {
            offset
        };
        for wide in [true, false] {
            let length = if wide { 20 } else { 12 };
            let Ok(record_end) = end(start, length, limit) else {
                continue;
            };
            let (actual_compressed, actual_uncompressed) = if wide {
                (qword(bytes, start + 4)?, qword(bytes, start + 12)?)
            } else {
                (
                    dword(bytes, start + 4)? as u64,
                    dword(bytes, start + 8)? as u64,
                )
            };
            if dword(bytes, start)? == crc
                && actual_compressed == compressed
                && actual_uncompressed == uncompressed
            {
                return Ok(record_end);
            }
        }
    }
    Err("Invalid ZIP data descriptor".into())
}

#[cfg(test)]
#[path = "zip_directory_tests.rs"]
mod tests;

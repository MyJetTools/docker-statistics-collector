//! The layout of a metric day file - one for every metric.
//!
//! A file is a run of frames of [`FRAME_LEN`] bytes, numbered from 0. A frame is named by
//! its number, and 0 in the place of a frame number means there is none: frame 0 is the
//! header of the file, which nothing points at.
//!
//! - frame 0 - the header of the file: `magic · version u8 · record length u8`;
//! - frame 1 - the first index frame, always there;
//! - an index frame - `[length u8][service name][first data frame u32]` one after
//!   another, a zero length after the last one;
//! - a data frame - `[time u32][limit f64][value f64]` one after another, in the order
//!   they came; a zero time is no record.
//!
//! Every frame but the header starts with `kind u8 · 3 reserved bytes · next u32`, `next`
//! being the next frame of its chain: the index frames are one chain, the data frames of
//! a service are another. All the numbers are little-endian.

/// A file is a run of frames of this size, their headers included - the size of the
/// frames of my-logger-server.
pub const FRAME_LEN: usize = 32 * 1024;

/// `kind u8 · 3 reserved bytes · next u32`
pub const FRAME_HEADER_LEN: usize = 8;

pub const FRAME_BODY_LEN: usize = FRAME_LEN - FRAME_HEADER_LEN;

/// `time u32 · limit f64 · value f64`
pub const RECORD_LEN: usize = 20;

pub const RECORDS_PER_FRAME: usize = FRAME_BODY_LEN / RECORD_LEN;

/// Always the head of the chain of the index frames
pub const FIRST_INDEX_FRAME: u32 = 1;

/// The header of the file and the first index frame: what a file starts with. No other
/// frame has a number below this.
pub const INITIAL_FRAMES: u32 = 2;

pub const KIND_INDEX: u8 = 1;
pub const KIND_DATA: u8 = 2;

/// The length of a name is a byte in the index
pub const MAX_SERVICE_NAME_LEN: usize = u8::MAX as usize;

const FILE_MAGIC: [u8; 4] = *b"DSMF";
const FILE_VERSION: u8 = 1;

/// `magic · version u8 · record length u8`
pub const FILE_HEADER_LEN: usize = 6;

pub fn frame_offset(frame: u32) -> u64 {
    frame as u64 * FRAME_LEN as u64
}

/// How many frames a file of `len` bytes has - one it has only a part of counts
pub fn frames_in(len: u64) -> u32 {
    len.div_ceil(FRAME_LEN as u64).min(u32::MAX as u64) as u32
}

pub fn file_header() -> [u8; FILE_HEADER_LEN] {
    let mut result = [0u8; FILE_HEADER_LEN];
    result[..4].copy_from_slice(&FILE_MAGIC);
    result[4] = FILE_VERSION;
    result[5] = RECORD_LEN as u8;
    result
}

#[derive(Debug, PartialEq, Eq)]
pub enum FileHeader {
    /// Zeroes: nothing was written into the file yet, or its first write did not complete
    Missing,
    /// A file of this format and version
    Ours,
    /// Something else - a file of another version, or not a metric file at all
    Foreign,
}

pub fn check_file_header(bytes: &[u8; FILE_HEADER_LEN]) -> FileHeader {
    if bytes.iter().all(|b| *b == 0) {
        FileHeader::Missing
    } else if *bytes == file_header() {
        FileHeader::Ours
    } else {
        FileHeader::Foreign
    }
}

pub fn frame_header(kind: u8, next: Option<u32>) -> [u8; FRAME_HEADER_LEN] {
    let mut result = [0u8; FRAME_HEADER_LEN];
    result[0] = kind;
    result[4..8].copy_from_slice(&next.unwrap_or(0).to_le_bytes());
    result
}

/// `frame` starts with the header of a frame
pub fn frame_kind(frame: &[u8]) -> u8 {
    frame[0]
}

/// The next frame of the chain. A number no frame of a chain can have - of the header of
/// the file, of the first index frame - is no next frame.
pub fn frame_next(frame: &[u8]) -> Option<u32> {
    let next = u32::from_le_bytes(frame[4..8].try_into().unwrap());
    (next >= INITIAL_FRAMES).then_some(next)
}

/// One measurement of a service: when it was taken, and what the value and its limit were
#[derive(Debug, Clone, Copy)]
pub struct Record {
    /// Unix seconds. Never 0: that is how a place with no record reads.
    pub time: u32,
    /// NaN when the value has no limit
    pub limit: f64,
    pub value: f64,
}

impl Record {
    pub fn to_bytes(&self) -> [u8; RECORD_LEN] {
        let mut result = [0u8; RECORD_LEN];
        result[0..4].copy_from_slice(&self.time.to_le_bytes());
        result[4..12].copy_from_slice(&self.limit.to_le_bytes());
        result[12..20].copy_from_slice(&self.value.to_le_bytes());
        result
    }

    /// `None` where there is no record
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let time = u32::from_le_bytes(bytes[0..4].try_into().unwrap());

        if time == 0 {
            return None;
        }

        Some(Self {
            time,
            limit: f64::from_le_bytes(bytes[4..12].try_into().unwrap()),
            value: f64::from_le_bytes(bytes[12..20].try_into().unwrap()),
        })
    }
}

/// The last record the body of a data frame has, and how many places are taken up to it -
/// the next record goes after them. A place a write did not reach stays empty between the
/// records, and is counted as taken: nothing is written over what comes after it.
pub fn last_record(body: &[u8]) -> Option<(usize, Record)> {
    body.chunks_exact(RECORD_LEN)
        .enumerate()
        .rev()
        .find_map(|(index, bytes)| Some((index + 1, Record::from_bytes(bytes)?)))
}

/// `[length u8][name][first data frame u32]`
pub fn index_entry(name: &str, first_frame: u32) -> Vec<u8> {
    let mut result = Vec::with_capacity(1 + name.len() + 4);
    result.push(name.len() as u8);
    result.extend_from_slice(name.as_bytes());
    result.extend_from_slice(&first_frame.to_le_bytes());
    result
}

/// The entries the body of an index frame starts with, and the bytes they take: the next
/// entry goes after them. Reading stops at the zero length which ends the entries - and
/// at an entry which can not be one, as what a cut write leaves is not an entry.
pub fn parse_index_entries(body: &[u8]) -> (Vec<(String, u32)>, usize) {
    let mut result = Vec::new();
    let mut offset = 0;

    while offset < body.len() {
        let name_len = body[offset] as usize;
        let name_start = offset + 1;
        let end = name_start + name_len + 4;

        if name_len == 0 || end > body.len() {
            break;
        }

        let Ok(name) = std::str::from_utf8(&body[name_start..name_start + name_len]) else {
            break;
        };

        let first_frame = u32::from_le_bytes(body[end - 4..end].try_into().unwrap());

        if first_frame < INITIAL_FRAMES || name.contains('\0') {
            break;
        }

        result.push((name.to_string(), first_frame));
        offset = end;
    }

    (result, offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_is_filled_by_its_records_to_the_byte() {
        assert_eq!(RECORDS_PER_FRAME, 1638);
        assert_eq!(FRAME_HEADER_LEN + RECORDS_PER_FRAME * RECORD_LEN, FRAME_LEN);
    }

    #[test]
    fn a_record_comes_back_from_its_bytes() {
        let record = Record {
            time: 1_791_763_205,
            limit: 8.0,
            value: 0.37,
        };

        let read = Record::from_bytes(&record.to_bytes()).unwrap();
        assert_eq!((read.time, read.limit, read.value), (1_791_763_205, 8.0, 0.37));

        let no_limit = Record {
            limit: f64::NAN,
            ..record
        };
        assert!(Record::from_bytes(&no_limit.to_bytes()).unwrap().limit.is_nan());

        assert!(Record::from_bytes(&[0u8; RECORD_LEN]).is_none());
    }

    #[test]
    fn the_last_record_is_found_past_a_place_left_empty() {
        let record = |time| Record {
            time,
            limit: f64::NAN,
            value: time as f64,
        };

        let mut body = vec![0u8; RECORD_LEN * 5];
        assert!(last_record(&body).is_none());

        body[..RECORD_LEN].copy_from_slice(&record(10).to_bytes());
        // The place of the second record stays empty
        body[RECORD_LEN * 2..RECORD_LEN * 3].copy_from_slice(&record(30).to_bytes());

        let (taken, last) = last_record(&body).unwrap();
        assert_eq!((taken, last.time), (3, 30));
    }

    #[test]
    fn index_entries_are_read_up_to_the_zero_length() {
        let mut body = index_entry("api", 2);
        body.extend(index_entry("сервис", 7));
        let used = body.len();
        body.resize(used + 64, 0);

        let (entries, read) = parse_index_entries(&body);

        assert_eq!(
            entries,
            [("api".to_string(), 2), ("сервис".to_string(), 7)]
        );
        assert_eq!(read, used);
    }

    #[test]
    fn what_a_cut_write_left_is_not_an_entry() {
        let whole = index_entry("api", 2);

        // The name is there, its frame is not: the number reads as zero
        let mut cut = whole.clone();
        cut.extend_from_slice(&index_entry("postgres", 5)[..9]);
        cut.resize(cut.len() + 16, 0);

        let (entries, read) = parse_index_entries(&cut);
        assert_eq!(entries, [("api".to_string(), 2)]);
        assert_eq!(read, whole.len());

        // An entry which would reach past the frame
        let (entries, read) = parse_index_entries(&[200, b'a', b'b']);
        assert!(entries.is_empty());
        assert_eq!(read, 0);
    }

    #[test]
    fn the_header_of_a_file_tells_ours_from_an_empty_and_a_foreign_one() {
        assert_eq!(check_file_header(&file_header()), FileHeader::Ours);
        assert_eq!(check_file_header(&[0u8; FILE_HEADER_LEN]), FileHeader::Missing);
        assert_eq!(check_file_header(b"prod: "), FileHeader::Foreign);

        let mut next_version = file_header();
        next_version[4] += 1;
        assert_eq!(check_file_header(&next_version), FileHeader::Foreign);
    }

    #[test]
    fn a_frame_header_keeps_its_kind_and_its_next() {
        let header = frame_header(KIND_DATA, Some(9));
        assert_eq!(frame_kind(&header), KIND_DATA);
        assert_eq!(frame_next(&header), Some(9));

        assert_eq!(frame_next(&frame_header(KIND_INDEX, None)), None);
        // Nothing is chained to the first index frame
        assert_eq!(frame_next(&frame_header(KIND_DATA, Some(FIRST_INDEX_FRAME))), None);
    }
}

use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::ErrorKind,
    os::unix::fs::FileExt,
    path::{Path, PathBuf},
};

use super::{
    check_file_header, file_header, frame_header, frame_kind, frame_next, frame_offset, frames_in,
    index_entry, last_record, parse_index_entries, FileHeader, Record, FILE_HEADER_LEN,
    FIRST_INDEX_FRAME, FRAME_BODY_LEN, FRAME_HEADER_LEN, FRAME_LEN, INITIAL_FRAMES, KIND_DATA,
    KIND_INDEX, MAX_SERVICE_NAME_LEN, RECORDS_PER_FRAME, RECORD_LEN,
};

/// The frames of a service in the file, and where its next record goes
#[derive(Debug, Clone, Copy)]
struct ServiceChain {
    first: u32,
    /// The last frame of the chain - the one which is written into
    tail: u32,
    /// The places of the tail which are taken: the next record goes after them
    tail_records: usize,
    /// The time of the last record written, 0 when there is none
    last_time: u32,
}

/// One metric of the services of one VM for one day. Every service has a chain of data
/// frames in the file, found through the index frames; a full frame gets a new one after
/// it at the end of the file. There is no index of time: the records of an interval are
/// found by reading the chain.
///
/// Nothing of the file is kept in memory but where each chain is. A record goes straight
/// to its place - one write of [`RECORD_LEN`] bytes - so there is nothing to flush.
pub struct DayFile {
    path: PathBuf,
    file: File,
    /// The frames the file has: the next one takes this number
    frames_count: u32,
    /// The header of the file is written
    has_header: bool,
    /// Why nothing is written into the file and nothing is read from it
    refused: Option<String>,
    services: HashMap<String, ServiceChain>,
    /// The last index frame, and the bytes of entries in it
    index_tail: u32,
    index_tail_used: usize,
}

impl DayFile {
    /// The file at `path`. Without `create` a file which is not there is `None`. A new
    /// file stays empty until its first record.
    pub fn open(path: &Path, create: bool) -> std::io::Result<Option<Self>> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(create)
            .open(path);

        let file = match file {
            Ok(file) => file,
            Err(err) if !create && err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err),
        };

        let mut result = Self {
            path: path.to_path_buf(),
            frames_count: frames_in(file.metadata()?.len()),
            file,
            has_header: false,
            refused: None,
            services: HashMap::new(),
            index_tail: FIRST_INDEX_FRAME,
            index_tail_used: 0,
        };

        let mut header = [0u8; FILE_HEADER_LEN];
        result.read_at(&mut header, 0)?;

        match check_file_header(&header) {
            FileHeader::Ours => {
                result.has_header = true;
                result.load()?;
            }
            // A new file - or one whose first write did not complete: what it has is read
            FileHeader::Missing => result.load()?,
            FileHeader::Foreign => {
                let reason = format!(
                    "{} is not a metric file of this version: it is left as it is",
                    path.display()
                );
                println!("MetricsStorage: {}", reason);
                result.refused = Some(reason);
            }
        }

        Ok(Some(result))
    }

    /// Reads the index, and walks the chain of every service to its last record
    fn load(&mut self) -> std::io::Result<()> {
        // A chain has no more frames than the file: a walk longer than that goes in circles
        let max_walk = self.frames_count as usize + 1;

        let mut frame = vec![0u8; FRAME_LEN];
        let mut entries = Vec::new();
        let mut index_frame = FIRST_INDEX_FRAME;

        for _ in 0..max_walk {
            self.read_at(&mut frame, frame_offset(index_frame))?;

            if frame_kind(&frame) != KIND_INDEX {
                break;
            }

            let (found, used) = parse_index_entries(&frame[FRAME_HEADER_LEN..]);
            entries.extend(found);
            self.index_tail = index_frame;
            self.index_tail_used = used;

            match frame_next(&frame) {
                Some(next) => index_frame = next,
                None => break,
            }
        }

        for (name, first) in entries {
            if self.services.contains_key(&name) {
                continue;
            }

            if let Some(chain) = self.load_chain(first, max_walk, &mut frame)? {
                self.services.insert(name, chain);
            }
        }

        Ok(())
    }

    /// Walks a chain from its first frame to its last one, and finds its last record.
    /// `None` when the first frame is not a frame of records.
    fn load_chain(
        &mut self,
        first: u32,
        max_walk: usize,
        frame: &mut [u8],
    ) -> std::io::Result<Option<ServiceChain>> {
        let mut header = [0u8; FRAME_HEADER_LEN];
        self.read_at(&mut header, frame_offset(first))?;

        if !is_data_frame(&header) {
            return Ok(None);
        }

        let mut tail = first;
        let mut before_tail = None;

        for _ in 0..max_walk {
            // A frame a chain goes through is taken, even when the file ends before it:
            // the write which made the file longer may be the one that was lost
            self.frames_count = self.frames_count.max(tail.saturating_add(1));

            let Some(next) = frame_next(&header) else {
                break;
            };

            let mut next_header = [0u8; FRAME_HEADER_LEN];
            self.read_at(&mut next_header, frame_offset(next))?;

            if !is_data_frame(&next_header) {
                break;
            }

            before_tail = Some(tail);
            tail = next;
            header = next_header;
        }

        self.frames_count = self.frames_count.max(tail.saturating_add(1));

        self.read_at(frame, frame_offset(tail))?;

        let (tail_records, mut last_time) = match last_record(&frame[FRAME_HEADER_LEN..]) {
            Some((taken, record)) => (taken, record.time),
            None => (0, 0),
        };

        // The tail is a frame just added: the last record is in the frame before it
        if tail_records == 0 {
            if let Some(before_tail) = before_tail {
                self.read_at(frame, frame_offset(before_tail))?;

                if let Some((_, record)) = last_record(&frame[FRAME_HEADER_LEN..]) {
                    last_time = record.time;
                }
            }
        }

        Ok(Some(ServiceChain {
            first,
            tail,
            tail_records,
            last_time,
        }))
    }

    /// Writes the record after the ones the service has. `false` when it is not written:
    /// the last record of the service is of this very second, or the file is not one to
    /// write into.
    pub fn append(&mut self, service: &str, record: &Record) -> std::io::Result<bool> {
        if self.refused.is_some() {
            return Ok(false);
        }

        if service.is_empty() || service.len() > MAX_SERVICE_NAME_LEN || service.contains('\0') {
            return Err(invalid_input(format!(
                "'{}' can not name a service in the index",
                service
            )));
        }

        if record.time == 0 {
            return Err(invalid_input("a record with no time".to_string()));
        }

        if !self.has_header {
            self.write_header()?;
        }

        let mut chain = match self.services.get(service) {
            Some(chain) => *chain,
            None => self.add_service(service)?,
        };

        if chain.last_time == record.time {
            return Ok(false);
        }

        if chain.tail_records >= RECORDS_PER_FRAME {
            let next = self.add_frame(KIND_DATA)?;

            self.file.write_all_at(
                &frame_header(KIND_DATA, Some(next)),
                frame_offset(chain.tail),
            )?;

            chain.tail = next;
            chain.tail_records = 0;
            // The chain is longer now, whatever comes of the record
            self.set_chain(service, chain);
        }

        let offset = frame_offset(chain.tail)
            + (FRAME_HEADER_LEN + chain.tail_records * RECORD_LEN) as u64;

        self.file.write_all_at(&record.to_bytes(), offset)?;

        chain.tail_records += 1;
        chain.last_time = record.time;
        self.set_chain(service, chain);

        Ok(true)
    }

    fn set_chain(&mut self, service: &str, chain: ServiceChain) {
        if let Some(stored) = self.services.get_mut(service) {
            *stored = chain;
        }
    }

    /// The first index frame, then the header of the file - written last, as it is what
    /// tells that the file is whole.
    fn write_header(&mut self) -> std::io::Result<()> {
        if self.frames_count < INITIAL_FRAMES {
            self.file.set_len(frame_offset(INITIAL_FRAMES))?;
            self.frames_count = INITIAL_FRAMES;
        }

        // The kind alone: an index frame which was there keeps its next
        self.file
            .write_all_at(&[KIND_INDEX], frame_offset(FIRST_INDEX_FRAME))?;
        self.file.write_all_at(&file_header(), 0)?;
        self.has_header = true;

        Ok(())
    }

    /// A first data frame for the service, then its entry in the index
    fn add_service(&mut self, service: &str) -> std::io::Result<ServiceChain> {
        let first = self.add_frame(KIND_DATA)?;

        let mut entry = index_entry(service, first);
        let entry_len = entry.len();

        if self.index_tail_used + entry_len > FRAME_BODY_LEN {
            let next = self.add_frame(KIND_INDEX)?;

            self.file.write_all_at(
                &frame_header(KIND_INDEX, Some(next)),
                frame_offset(self.index_tail),
            )?;

            self.index_tail = next;
            self.index_tail_used = 0;
        }

        // The zero length goes with it where there is room, so that the entries end here
        // whatever a cut write has left at this place before
        if self.index_tail_used + entry_len < FRAME_BODY_LEN {
            entry.push(0);
        }

        let offset =
            frame_offset(self.index_tail) + (FRAME_HEADER_LEN + self.index_tail_used) as u64;

        self.file.write_all_at(&entry, offset)?;
        self.index_tail_used += entry_len;

        let chain = ServiceChain {
            first,
            tail: first,
            tail_records: 0,
            last_time: 0,
        };

        self.services.insert(service.to_string(), chain);

        Ok(chain)
    }

    /// A new frame at the end of the file. Nothing points at it yet: a frame is written
    /// before the pointer to it, so what a write cut short leaves is a frame nobody uses.
    fn add_frame(&mut self, kind: u8) -> std::io::Result<u32> {
        let frame = self.frames_count;

        if frame == u32::MAX {
            return Err(std::io::Error::other(format!(
                "{} has no room for more frames",
                self.path.display()
            )));
        }

        self.file.set_len(frame_offset(frame + 1))?;
        self.file
            .write_all_at(&frame_header(kind, None), frame_offset(frame))?;
        self.frames_count = frame + 1;

        Ok(frame)
    }

    /// Adds the records the service has in `from .. to` to `out`, in the order they are
    /// in the file - the order they were written in.
    pub fn read(
        &self,
        service: &str,
        from: u32,
        to: u32,
        out: &mut Vec<Record>,
    ) -> std::io::Result<()> {
        let Some(chain) = self.services.get(service) else {
            return Ok(());
        };

        let mut frame = vec![0u8; FRAME_LEN];
        let mut number = chain.first;

        // No chain is longer than the file
        for _ in 0..self.frames_count {
            self.read_at(&mut frame, frame_offset(number))?;

            for bytes in frame[FRAME_HEADER_LEN..].chunks_exact(RECORD_LEN) {
                if let Some(record) = Record::from_bytes(bytes) {
                    if record.time >= from && record.time < to {
                        out.push(record);
                    }
                }
            }

            if number == chain.tail {
                break;
            }

            match frame_next(&frame) {
                Some(next) => number = next,
                None => break,
            }
        }

        Ok(())
    }

    /// Fills `buf` from `offset`; what the file does not have reads as zeroes - which is
    /// what a frame nothing was written into is.
    fn read_at(&self, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
        let mut done = 0;

        while done < buf.len() {
            match self.file.read_at(&mut buf[done..], offset + done as u64) {
                Ok(0) => break,
                Ok(read) => done += read,
                Err(err) if err.kind() == ErrorKind::Interrupted => {}
                Err(err) => return Err(err),
            }
        }

        buf[done..].fill(0);

        Ok(())
    }

    #[cfg(test)]
    fn frames_count(&self) -> u32 {
        self.frames_count
    }

    #[cfg(test)]
    fn services_count(&self) -> usize {
        self.services.len()
    }
}

/// A frame of records - or one which was given to a chain and did not get its header
fn is_data_frame(header: &[u8]) -> bool {
    matches!(frame_kind(header), KIND_DATA | 0)
}

fn invalid_input(message: String) -> std::io::Error {
    std::io::Error::new(ErrorKind::InvalidInput, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file of its own per test, under the system temp dir, not there yet.
    fn temp_file(test: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "docker-statistics-hub-day-file-{}-{}.cpu",
            test,
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        path
    }

    fn record(time: u32) -> Record {
        Record {
            time,
            limit: 8.0,
            value: time as f64 / 10.0,
        }
    }

    fn open(path: &Path) -> DayFile {
        DayFile::open(path, true).unwrap().unwrap()
    }

    fn times(file: &DayFile, service: &str, from: u32, to: u32) -> Vec<u32> {
        let mut out = Vec::new();
        file.read(service, from, to, &mut out).unwrap();
        out.iter().map(|record| record.time).collect()
    }

    fn all_times(file: &DayFile, service: &str) -> Vec<u32> {
        times(file, service, 0, u32::MAX)
    }

    #[test]
    fn a_file_which_is_not_there_is_not_made_by_a_read() {
        let path = temp_file("missing");

        assert!(DayFile::open(&path, false).unwrap().is_none());
        assert!(!path.exists());
    }

    #[test]
    fn records_come_back_by_service_and_by_interval() {
        let path = temp_file("read-back");
        let mut file = open(&path);

        for time in [100, 103, 106] {
            assert!(file.append("api", &record(time)).unwrap());
            assert!(file.append("postgres", &record(time + 1)).unwrap());
        }

        assert_eq!(all_times(&file, "api"), [100, 103, 106]);
        assert_eq!(all_times(&file, "postgres"), [101, 104, 107]);
        assert!(all_times(&file, "nobody").is_empty());

        // `from` is in, `to` is not
        assert_eq!(times(&file, "api", 103, 106), [103]);
        assert_eq!(times(&file, "api", 101, 200), [103, 106]);

        let mut out = Vec::new();
        file.read("api", 100, 101, &mut out).unwrap();
        assert_eq!((out[0].limit, out[0].value), (8.0, 10.0));
    }

    #[test]
    fn a_limit_which_is_not_there_is_kept_as_such() {
        let path = temp_file("no-limit");
        let mut file = open(&path);

        file.append(
            "api",
            &Record {
                time: 100,
                limit: f64::NAN,
                value: 1.5,
            },
        )
        .unwrap();

        let mut out = Vec::new();
        open(&path).read("api", 0, u32::MAX, &mut out).unwrap();

        assert!(out[0].limit.is_nan());
        assert_eq!(out[0].value, 1.5);
    }

    #[test]
    fn one_second_of_a_service_is_written_once() {
        let path = temp_file("same-second");
        let mut file = open(&path);

        assert!(file.append("api", &record(100)).unwrap());
        assert!(!file.append("api", &record(100)).unwrap());
        // The same second of another service is its own
        assert!(file.append("postgres", &record(100)).unwrap());

        // And it is known after the file is opened anew
        let mut file = open(&path);
        assert!(!file.append("api", &record(100)).unwrap());
        assert!(file.append("api", &record(101)).unwrap());

        assert_eq!(all_times(&file, "api"), [100, 101]);
    }

    #[test]
    fn a_file_opened_anew_goes_on_where_it_stopped() {
        let path = temp_file("reopen");

        {
            let mut file = open(&path);
            file.append("api", &record(100)).unwrap();
            file.append("postgres", &record(100)).unwrap();
            file.append("api", &record(103)).unwrap();
        }

        // A second instance over the same file is what a restart is.
        let mut file = open(&path);
        let frames = file.frames_count();

        file.append("api", &record(106)).unwrap();
        file.append("postgres", &record(106)).unwrap();

        assert_eq!(all_times(&file, "api"), [100, 103, 106]);
        assert_eq!(all_times(&file, "postgres"), [100, 106]);
        // Nothing was added to the file but the records
        assert_eq!(file.frames_count(), frames);
        assert_eq!(file.services_count(), 2);
    }

    #[test]
    fn a_full_frame_gets_the_next_one_chained_after_it() {
        let path = temp_file("next-frame");
        let mut file = open(&path);

        let count = RECORDS_PER_FRAME as u32 * 2 + 10;

        for time in 1..=count {
            file.append("api", &record(time)).unwrap();
            // Another service takes frames in between: the chains are woven together
            if time % 500 == 0 {
                file.append("postgres", &record(time)).unwrap();
            }
        }

        let expected: Vec<u32> = (1..=count).collect();
        assert_eq!(all_times(&file, "api"), expected);

        // The header, the index, three frames of one service and one of the other
        assert_eq!(file.frames_count(), 6);
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            frame_offset(6)
        );

        let mut file = open(&path);
        assert_eq!(all_times(&file, "api"), expected);
        assert_eq!(all_times(&file, "postgres").len(), 6);

        file.append("api", &record(count + 1)).unwrap();
        assert_eq!(all_times(&file, "api").len(), count as usize + 1);
        assert_eq!(file.frames_count(), 6);
    }

    #[test]
    fn a_frame_filled_to_its_last_place_is_followed_on_the_next_record() {
        let path = temp_file("exactly-full");

        {
            let mut file = open(&path);
            for time in 1..=RECORDS_PER_FRAME as u32 {
                file.append("api", &record(time)).unwrap();
            }
            assert_eq!(file.frames_count(), 3);
        }

        let mut file = open(&path);
        let next = RECORDS_PER_FRAME as u32 + 1;

        // The last second of the full frame is still known
        assert!(!file.append("api", &record(next - 1)).unwrap());
        assert!(file.append("api", &record(next)).unwrap());
        assert_eq!(file.frames_count(), 4);

        // The new frame is chained and its record did not get to the disk: the last
        // second is the one of the frame before
        {
            let file = OpenOptions::new().write(true).open(&path).unwrap();
            let offset = frame_offset(3) + FRAME_HEADER_LEN as u64;
            file.write_all_at(&[0u8; RECORD_LEN], offset).unwrap();
        }

        let mut file = open(&path);
        assert!(!file.append("api", &record(next - 1)).unwrap());
        assert!(file.append("api", &record(next)).unwrap());

        assert_eq!(all_times(&file, "api").len(), next as usize);
        assert_eq!(file.frames_count(), 4);
    }

    #[test]
    fn the_index_goes_over_to_a_next_frame_when_it_is_full() {
        let path = temp_file("index-frames");

        // 200 bytes of a name: some 160 entries fill an index frame
        let name = |number: usize| format!("{:0>200}", number);
        let services = 400;

        {
            let mut file = open(&path);
            for number in 0..services {
                file.append(&name(number), &record(100 + number as u32))
                    .unwrap();
            }
        }

        let mut file = open(&path);
        assert_eq!(file.services_count(), services);

        for number in 0..services {
            assert_eq!(all_times(&file, &name(number)), [100 + number as u32]);
        }

        // One more service lands in the last index frame, and is there after a restart
        file.append("one-more", &record(7)).unwrap();
        assert_eq!(all_times(&open(&path), "one-more"), [7]);
    }

    #[test]
    fn a_frame_nobody_points_at_does_no_harm() {
        let path = temp_file("orphan");

        {
            let mut file = open(&path);
            file.append("api", &record(100)).unwrap();
        }

        // A frame was added and the process stopped before the pointer to it was written
        {
            let file = OpenOptions::new().write(true).open(&path).unwrap();
            let len = file.metadata().unwrap().len();
            file.set_len(len + FRAME_LEN as u64).unwrap();
            file.write_all_at(&frame_header(KIND_DATA, None), len).unwrap();
        }

        let mut file = open(&path);
        file.append("api", &record(103)).unwrap();
        file.append("postgres", &record(103)).unwrap();

        let file = open(&path);
        assert_eq!(all_times(&file, "api"), [100, 103]);
        assert_eq!(all_times(&file, "postgres"), [103]);
    }

    #[test]
    fn a_place_a_write_did_not_reach_is_skipped_not_written_over() {
        let path = temp_file("hole");

        {
            let mut file = open(&path);
            for time in [100, 103, 106] {
                file.append("api", &record(time)).unwrap();
            }
        }

        // The second record did not get to the disk, the third one did
        {
            let file = OpenOptions::new().write(true).open(&path).unwrap();
            let offset = frame_offset(INITIAL_FRAMES) + (FRAME_HEADER_LEN + RECORD_LEN) as u64;
            file.write_all_at(&[0u8; RECORD_LEN], offset).unwrap();
        }

        let mut file = open(&path);
        file.append("api", &record(109)).unwrap();

        assert_eq!(all_times(&open(&path), "api"), [100, 106, 109]);
    }

    #[test]
    fn a_chain_which_goes_past_the_end_of_the_file_keeps_its_frame() {
        let path = temp_file("past-the-end");

        {
            let mut file = open(&path);
            for time in 1..=RECORDS_PER_FRAME as u32 + 1 {
                file.append("api", &record(time)).unwrap();
            }
        }

        // The pointer to the second frame got to the disk, the frame itself did not
        {
            let file = OpenOptions::new().write(true).open(&path).unwrap();
            let len = file.metadata().unwrap().len();
            file.set_len(len - FRAME_LEN as u64).unwrap();
        }

        let mut file = open(&path);

        // Another service must not be given the frame the chain goes through
        file.append("postgres", &record(5)).unwrap();
        file.append("api", &record(9_000)).unwrap();

        let file = open(&path);
        assert_eq!(all_times(&file, "postgres"), [5]);

        let api = all_times(&file, "api");
        assert_eq!(api.len(), RECORDS_PER_FRAME + 1);
        assert_eq!(api.last(), Some(&9_000));
    }

    #[test]
    fn a_file_which_is_something_else_is_left_as_it_is() {
        let path = temp_file("foreign");
        std::fs::write(&path, "prod: [not, a, metric, file]\n").unwrap();

        let mut file = open(&path);

        assert!(!file.append("api", &record(100)).unwrap());
        assert!(all_times(&file, "api").is_empty());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "prod: [not, a, metric, file]\n"
        );
    }

    #[test]
    fn a_name_the_index_can_not_hold_is_refused() {
        let path = temp_file("bad-name");
        let mut file = open(&path);

        assert!(file.append("", &record(100)).is_err());
        assert!(file
            .append(&"x".repeat(MAX_SERVICE_NAME_LEN + 1), &record(100))
            .is_err());
        assert!(file
            .append(&"x".repeat(MAX_SERVICE_NAME_LEN), &record(100))
            .is_ok());
    }
}

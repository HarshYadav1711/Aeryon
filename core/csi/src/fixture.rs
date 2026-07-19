//! Aeryon CSI Fixture Format v1 — deterministic development replay source.
//!
//! This is an explicitly versioned **development** fixture format for tests,
//! CI, and offline architecture validation. It is **not** the production
//! recording format and does not represent live RF capture.

use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use aeryon_domain::{
    FrameId, FrameMetadata, Metadata, MetadataKey, MetadataValue, SensorId, Timestamp,
};
use num_complex::Complex32;
use serde::{Deserialize, Serialize};

use crate::error::FixtureError;
use crate::frame::{CsiFrame, CsiRadioMetadata, CsiSourceKind};

/// Canonical fixture schema identifier.
pub const FIXTURE_SCHEMA: &str = "aeryon-csi-fixture";
/// Supported fixture schema version.
pub const FIXTURE_VERSION: u32 = 1;
/// Canonical sample layout label for `[rx][tx][subcarrier]` storage.
pub const SAMPLE_LAYOUT_RX_TX_SUBCARRIER: &str = "rx-tx-subcarrier";

/// Sample layout enum for validated fixture headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalSampleLayout {
    /// Contiguous `[receive_antenna][transmit_antenna][subcarrier]` order.
    RxTxSubcarrier,
}

impl CanonicalSampleLayout {
    fn parse(value: &str) -> Option<Self> {
        match value {
            SAMPLE_LAYOUT_RX_TX_SUBCARRIER => Some(Self::RxTxSubcarrier),
            _ => None,
        }
    }

    /// Wire label used in fixture headers.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RxTxSubcarrier => SAMPLE_LAYOUT_RX_TX_SUBCARRIER,
        }
    }
}

/// Parsed fixture header (first NDJSON record).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureHeader {
    /// Schema identifier.
    pub schema: String,
    /// Schema version.
    pub version: u32,
    /// Sensor identity for emitted frames.
    pub sensor_id: SensorId,
    /// Human-readable description.
    pub description: String,
    /// Required sample layout.
    pub sample_layout: CanonicalSampleLayout,
}

/// Serde representation of a real/imaginary pair.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ComplexPair {
    /// Real component.
    pub re: f32,
    /// Imaginary component.
    pub im: f32,
}

impl From<ComplexPair> for Complex32 {
    fn from(value: ComplexPair) -> Self {
        Complex32::new(value.re, value.im)
    }
}

impl From<Complex32> for ComplexPair {
    fn from(value: Complex32) -> Self {
        Self {
            re: value.re,
            im: value.im,
        }
    }
}

/// Optional radio fields in a fixture frame record.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct FixtureRadioRecord {
    /// RSSI in dBm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rssi_dbm: Option<f32>,
    /// Noise floor in dBm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub noise_floor_dbm: Option<f32>,
    /// AGC value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agc: Option<f32>,
    /// Packet/frame flags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
}

/// Serializable fixture frame record (subsequent NDJSON lines).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixtureFrameRecord {
    /// Record discriminator (`"frame"`).
    pub record_type: String,
    /// Frame identifier.
    pub frame_id: u64,
    /// Monotonic sequence number.
    pub sequence: u64,
    /// Capture timestamp in nanoseconds since Unix epoch.
    pub capture_timestamp_nanos: u64,
    /// Optional center frequency in hertz.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center_frequency_hz: Option<f64>,
    /// Optional channel bandwidth in hertz.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandwidth_hz: Option<f64>,
    /// Receive antenna count.
    pub receive_antennas: u16,
    /// Transmit antenna count.
    pub transmit_antennas: u16,
    /// Strictly ascending subcarrier indices.
    pub subcarrier_indices: Vec<i16>,
    /// Contiguous complex samples in `[rx][tx][subcarrier]` order.
    pub samples: Vec<ComplexPair>,
    /// Optional radio metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radio: Option<FixtureRadioRecord>,
}

#[derive(Debug, Deserialize)]
struct RawHeader {
    record_type: String,
    schema: String,
    version: u32,
    sensor_id: String,
    description: String,
    sample_layout: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawRecord {
    Header(RawHeader),
    Frame(FixtureFrameRecord),
    Unknown { record_type: String },
}

/// Streaming reader for Aeryon CSI Fixture Format v1.
#[derive(Debug)]
pub struct FixtureReader<R> {
    reader: BufReader<R>,
    header: Option<FixtureHeader>,
    line: usize,
    last_sequence: Option<u64>,
    finished: bool,
}

impl FixtureReader<File> {
    /// Opens a fixture file for streaming reads.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, FixtureError> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|source| FixtureError::Io { line: 0, source })?;
        Self::new(file)
    }
}

impl<R: Read> FixtureReader<R> {
    /// Wraps any readable stream.
    pub fn new(reader: R) -> Result<Self, FixtureError> {
        let mut reader = Self {
            reader: BufReader::new(reader),
            header: None,
            line: 0,
            last_sequence: None,
            finished: false,
        };
        reader.read_header()?;
        Ok(reader)
    }

    /// Returns the validated fixture header.
    pub fn header(&self) -> &FixtureHeader {
        self.header
            .as_ref()
            .expect("header is loaded during construction")
    }

    /// Reads the next validated CSI frame, or `None` at end of stream.
    pub fn next_frame(&mut self) -> Result<Option<CsiFrame>, FixtureError> {
        if self.finished {
            return Ok(None);
        }

        loop {
            let Some(raw_line) = self.read_line()? else {
                self.finished = true;
                return Ok(None);
            };

            if raw_line.trim().is_empty() {
                continue;
            }

            let record: RawRecord =
                serde_json::from_str(&raw_line).map_err(|source| FixtureError::Json {
                    line: self.line,
                    source,
                })?;

            match record {
                RawRecord::Frame(frame) => {
                    let csi = self.frame_record_to_csi(frame)?;
                    return Ok(Some(csi));
                }
                RawRecord::Header(_) => {
                    return Err(FixtureError::UnexpectedRecordType {
                        line: self.line,
                        record_type: "header".to_owned(),
                    });
                }
                RawRecord::Unknown { record_type } => {
                    return Err(FixtureError::UnexpectedRecordType {
                        line: self.line,
                        record_type,
                    });
                }
            }
        }
    }

    fn read_header(&mut self) -> Result<(), FixtureError> {
        loop {
            let Some(raw_line) = self.read_line()? else {
                return Err(FixtureError::MissingHeader);
            };
            if raw_line.trim().is_empty() {
                continue;
            }

            let record: RawRecord =
                serde_json::from_str(&raw_line).map_err(|source| FixtureError::Json {
                    line: self.line,
                    source,
                })?;

            let RawRecord::Header(header) = record else {
                return Err(FixtureError::ExpectedHeader { line: self.line });
            };

            if header.record_type != "header" {
                return Err(FixtureError::ExpectedHeader { line: self.line });
            }
            if header.schema != FIXTURE_SCHEMA {
                return Err(FixtureError::UnsupportedSchema {
                    line: self.line,
                    schema: header.schema,
                });
            }
            if header.version != FIXTURE_VERSION {
                return Err(FixtureError::UnsupportedVersion {
                    line: self.line,
                    version: header.version,
                });
            }
            let sample_layout =
                CanonicalSampleLayout::parse(&header.sample_layout).ok_or_else(|| {
                    FixtureError::UnsupportedLayout {
                        line: self.line,
                        layout: header.sample_layout.clone(),
                    }
                })?;

            let sensor_value = header.sensor_id.parse::<u64>().map_err(|source| {
                FixtureError::InvalidSensorId {
                    line: self.line,
                    source,
                }
            })?;

            self.header = Some(FixtureHeader {
                schema: header.schema,
                version: header.version,
                sensor_id: SensorId::new(sensor_value),
                description: header.description,
                sample_layout,
            });
            return Ok(());
        }
    }

    fn frame_record_to_csi(
        &mut self,
        record: FixtureFrameRecord,
    ) -> Result<CsiFrame, FixtureError> {
        if record.record_type != "frame" {
            return Err(FixtureError::UnexpectedRecordType {
                line: self.line,
                record_type: record.record_type,
            });
        }

        if let Some(previous) = self.last_sequence {
            if record.sequence <= previous {
                return Err(FixtureError::NonMonotonicSequence {
                    line: self.line,
                    previous,
                    current: record.sequence,
                });
            }
        }

        let sensor_id = self.header().sensor_id;
        let samples: Vec<Complex32> = record.samples.into_iter().map(Complex32::from).collect();
        let radio = record.radio.unwrap_or_default();

        let mut metadata_fields = Metadata::new();
        metadata_fields.insert(
            MetadataKey::Source,
            MetadataValue::Text(CsiSourceKind::Replay.as_str().to_owned()),
        );

        let metadata = FrameMetadata {
            frame_id: FrameId::new(record.frame_id),
            sensor_id,
            timestamp: Timestamp::from_nanos(record.capture_timestamp_nanos),
            sequence: record.sequence,
            mission_id: None,
            metadata: metadata_fields,
        };

        let frame = CsiFrame::try_new(
            metadata,
            Timestamp::from_nanos(record.capture_timestamp_nanos),
            record.center_frequency_hz,
            record.bandwidth_hz,
            record.receive_antennas,
            record.transmit_antennas,
            record.subcarrier_indices,
            samples,
            CsiSourceKind::Replay,
            CsiRadioMetadata {
                rssi_dbm: radio.rssi_dbm,
                noise_floor_dbm: radio.noise_floor_dbm,
                agc: radio.agc,
                flags: radio.flags,
            },
        )
        .map_err(|source| FixtureError::InvalidFrame {
            line: self.line,
            source,
        })?;

        self.last_sequence = Some(record.sequence);
        Ok(frame)
    }

    fn read_line(&mut self) -> Result<Option<String>, FixtureError> {
        let mut buffer = String::new();
        let bytes = self
            .reader
            .read_line(&mut buffer)
            .map_err(|source| FixtureError::Io {
                line: self.line.saturating_add(1),
                source,
            })?;
        if bytes == 0 {
            return Ok(None);
        }
        self.line = self.line.saturating_add(1);
        Ok(Some(buffer))
    }
}

impl<R: Read> Iterator for FixtureReader<R> {
    type Item = Result<CsiFrame, FixtureError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_frame() {
            Ok(Some(frame)) => Some(Ok(frame)),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        }
    }
}

/// Serializes a complex sample round-trip through the fixture pair representation.
pub fn complex_pair_round_trip(sample: Complex32) -> Complex32 {
    Complex32::from(ComplexPair::from(sample))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const HEADER: &str = r#"{"record_type":"header","schema":"aeryon-csi-fixture","version":1,"sensor_id":"2","description":"test","sample_layout":"rx-tx-subcarrier"}"#;

    fn frame_line(sequence: u64) -> String {
        format!(
            r#"{{"record_type":"frame","frame_id":{},"sequence":{},"capture_timestamp_nanos":{},"center_frequency_hz":5180000000.0,"bandwidth_hz":20000000.0,"receive_antennas":2,"transmit_antennas":1,"subcarrier_indices":[-1,0,1],"samples":[{{"re":1.0,"im":0.0}},{{"re":0.0,"im":1.0}},{{"re":-1.0,"im":0.0}},{{"re":2.0,"im":0.0}},{{"re":0.0,"im":2.0}},{{"re":-2.0,"im":0.0}}]}}"#,
            sequence + 1,
            sequence,
            1_000 + sequence
        )
    }

    #[test]
    fn supported_v1_header_and_frames() {
        let source = format!("{HEADER}\n{}\n{}", frame_line(0), frame_line(1));
        let mut reader = FixtureReader::new(Cursor::new(source)).expect("open");
        assert_eq!(reader.header().version, 1);
        let first = reader.next_frame().expect("f0").expect("some");
        let second = reader.next_frame().expect("f1").expect("some");
        assert_eq!(first.sequence(), 0);
        assert_eq!(second.sequence(), 1);
        assert!(reader.next_frame().expect("eof").is_none());
    }

    #[test]
    fn missing_header_is_rejected() {
        let error = FixtureReader::new(Cursor::new("")).expect_err("missing");
        assert!(matches!(error, FixtureError::MissingHeader));
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let source = r#"{"record_type":"header","schema":"aeryon-csi-fixture","version":99,"sensor_id":"2","description":"x","sample_layout":"rx-tx-subcarrier"}"#;
        let error = FixtureReader::new(Cursor::new(source)).expect_err("version");
        assert!(matches!(
            error,
            FixtureError::UnsupportedVersion { version: 99, .. }
        ));
    }

    #[test]
    fn invalid_layout_is_rejected() {
        let source = r#"{"record_type":"header","schema":"aeryon-csi-fixture","version":1,"sensor_id":"2","description":"x","sample_layout":"tx-rx"}"#;
        let error = FixtureReader::new(Cursor::new(source)).expect_err("layout");
        assert!(matches!(error, FixtureError::UnsupportedLayout { .. }));
    }

    #[test]
    fn malformed_json_reports_line() {
        let source = format!("{HEADER}\n{{not-json");
        let mut reader = FixtureReader::new(Cursor::new(source)).expect("header ok");
        let error = reader.next_frame().expect_err("json");
        assert_eq!(error.line(), Some(2));
        assert!(matches!(error, FixtureError::Json { .. }));
    }

    #[test]
    fn dimension_mismatch_reports_line() {
        let bad = r#"{"record_type":"frame","frame_id":1,"sequence":0,"capture_timestamp_nanos":1,"receive_antennas":2,"transmit_antennas":1,"subcarrier_indices":[0,1],"samples":[{"re":1.0,"im":0.0}]}"#;
        let source = format!("{HEADER}\n{bad}");
        let mut reader = FixtureReader::new(Cursor::new(source)).expect("header");
        let error = reader.next_frame().expect_err("dims");
        assert_eq!(error.line(), Some(2));
        assert!(matches!(error, FixtureError::InvalidFrame { .. }));
    }

    #[test]
    fn non_monotonic_sequence_is_rejected() {
        let source = format!("{HEADER}\n{}\n{}", frame_line(0), frame_line(0));
        let mut reader = FixtureReader::new(Cursor::new(source)).expect("header");
        reader.next_frame().expect("first").expect("some");
        let error = reader.next_frame().expect_err("mono");
        assert!(matches!(
            error,
            FixtureError::NonMonotonicSequence {
                previous: 0,
                current: 0,
                ..
            }
        ));
    }

    #[test]
    fn deterministic_output_order() {
        let source = format!(
            "{HEADER}\n{}\n{}\n{}",
            frame_line(0),
            frame_line(1),
            frame_line(2)
        );
        let sequences: Vec<_> = FixtureReader::new(Cursor::new(source))
            .expect("reader")
            .map(|frame| frame.expect("frame").sequence())
            .collect();
        assert_eq!(sequences, vec![0, 1, 2]);
    }

    #[test]
    fn complex_pair_serialization_round_trip() {
        let sample = Complex32::new(1.25, -0.5);
        let round = complex_pair_round_trip(sample);
        assert_eq!(round, sample);
        let json = serde_json::to_string(&ComplexPair::from(sample)).expect("ser");
        let parsed: ComplexPair = serde_json::from_str(&json).expect("de");
        assert_eq!(Complex32::from(parsed), sample);
    }

    #[test]
    fn first_non_header_record_is_rejected() {
        let error = FixtureReader::new(Cursor::new(frame_line(0))).expect_err("header");
        assert!(matches!(error, FixtureError::ExpectedHeader { line: 1 }));
    }
}

//! RustDDS backend.
//!
//! Implements the agnostic [`DdsBackend`] trait (used by the inspector)
//! and adds typed pub/sub helpers ([`TypedPublisher`], [`TypedSubscriber`])
//! used by the `pub-rustdds` / `sub-rustdds` example binaries.

use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use mio::{Events, Interest, Poll, Token};
use rustdds::no_key::{
    Decode as NoKeyDecode, DefaultDecoder as NoKeyDefaultDecoder,
    DeserializerAdapter as NoKeyDeserializerAdapter,
};
use rustdds::policy::{Durability, History, Reliability};
use rustdds::with_key::{
    DataReader as WithKeyDataReader, DataWriter as WithKeyDataWriter, Decode as WithKeyDecode,
    DefaultDecoder as WithKeyDefaultDecoder, DeserializerAdapter as WithKeyDeserializerAdapter,
    Sample,
};
use rustdds::{
    DataReaderStatus, DataWriterStatus, DomainParticipant, DomainParticipantBuilder,
    DomainParticipantStatusEvent, Key, Keyed, QosPolicies, QosPolicyBuilder,
    RepresentationIdentifier, StatusEvented, Topic, TopicKind,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::metrics::{MatchTable, PubMetrics, PubReport, SubMetrics};
use super::{
    BackendEvent, BackendSample, DdsBackend, EndpointDto, ParticipantDto, TopicDto,
};

// ============================================================================
// RustDdsBackend — the inspector-side impl
// ============================================================================

pub struct RustDdsBackend {
    dp: Arc<DomainParticipant>,
}

impl RustDdsBackend {
    /// Returns the underlying participant for backend-specific operations
    /// (typed publish/subscribe). Most callers should use the typed
    /// helpers below instead of touching this directly.
    pub fn domain_participant(&self) -> &Arc<DomainParticipant> {
        &self.dp
    }
}

impl DdsBackend for RustDdsBackend {
    fn name() -> &'static str {
        "rustdds"
    }

    fn version() -> &'static str {
        "0.11"
    }

    fn start(domain_id: u16, events_tx: Sender<BackendEvent>) -> Result<Self> {
        let dp = Arc::new(
            DomainParticipantBuilder::new(domain_id)
                .build()
                .context("DomainParticipant build failed")?,
        );
        spawn_discovery_thread(dp.clone(), events_tx);
        Ok(Self { dp })
    }

    fn spawn_raw_subscription(
        &self,
        topic_name: String,
        type_name: String,
        samples_tx: Sender<BackendSample>,
        stop: Arc<AtomicBool>,
    ) -> Result<()> {
        let dp = self.dp.clone();
        thread::spawn(move || {
            if let Err(e) = run_raw_subscription(&dp, &topic_name, &type_name, &samples_tx, &stop)
            {
                eprintln!("subscription '{topic_name}' ended: {e:?}");
            }
        });
        Ok(())
    }
}

// ============================================================================
// Typed publisher / subscriber — for the example binaries
// ============================================================================

/// Inspector-grade default QoS: Reliable, Volatile, KeepLast 100.
pub fn default_inspector_qos() -> QosPolicies {
    QosPolicyBuilder::new()
        .reliability(Reliability::Reliable {
            max_blocking_time: rustdds::Duration::from_millis(100),
        })
        .durability(Durability::Volatile)
        .history(History::KeepLast { depth: 100 })
        .build()
}

/// Thin wrapper around a RustDDS DataWriter that publishes typed
/// samples on a WITH_KEY topic. Tracks per-call timing in `metrics` and
/// per-reader match sessions in `matches`.
pub struct TypedPublisher<T>
where
    T: Keyed + Serialize + 'static,
    <T as Keyed>::K: Key,
{
    writer: WithKeyDataWriter<T>,
    _topic: Topic,
    metrics: Arc<PubMetrics>,
    matches: Arc<MatchTable>,
}

impl<T> TypedPublisher<T>
where
    T: Keyed + Serialize + 'static,
    <T as Keyed>::K: Key,
{
    pub fn write(&self, value: T) -> Result<()> {
        self.drain_status();
        let start = Instant::now();
        let res = self.writer.write(value, None);
        let elapsed_ns = start.elapsed().as_nanos() as u64;
        match res {
            Ok(()) => {
                self.metrics.record_write(elapsed_ns);
                Ok(())
            }
            Err(_) => {
                self.metrics.record_error();
                Err(anyhow::anyhow!("write failed"))
            }
        }
    }

    /// Pull any pending writer status events (PublicationMatched, etc.)
    /// and update the match table. Called automatically before each
    /// write — callers should invoke it once more at shutdown to catch
    /// trailing events.
    pub fn drain_status(&self) {
        while let Some(event) = self.writer.try_recv_status() {
            handle_writer_status(&self.matches, &self.metrics, event);
        }
    }

    /// Shared handle to the publisher's metrics — clone and hand off to
    /// a heartbeat / reporting thread.
    pub fn metrics(&self) -> &Arc<PubMetrics> {
        &self.metrics
    }

    /// Shared handle to the publisher's match table.
    pub fn matches(&self) -> &Arc<MatchTable> {
        &self.matches
    }

    /// Combined snapshot suitable for a final exit dump.
    pub fn report(&self) -> PubReport {
        PubReport {
            metrics: self.metrics.snapshot(),
            matches: self.matches.snapshot(),
        }
    }
}

fn handle_writer_status(matches: &MatchTable, metrics: &PubMetrics, event: DataWriterStatus) {
    if let DataWriterStatus::PublicationMatched { current, reader, .. } = event {
        let guid = guid_string(&reader);
        let sent_now = metrics.sent();
        if current.count_change() > 0 {
            matches.on_matched(guid, sent_now);
        } else if current.count_change() < 0 {
            matches.on_unmatched(&guid, sent_now);
        }
    }
}

/// Thin wrapper around a RustDDS DataReader exposing a sync
/// `take_next_sample` API for the example subscriber binary. Drains
/// `DataReaderStatus::SampleLost` events into `metrics` on every poll.
pub struct TypedSubscriber<T>
where
    T: Keyed + DeserializeOwned + 'static,
    <T as Keyed>::K: Key + DeserializeOwned,
{
    reader: WithKeyDataReader<T>,
    poll: Poll,
    events: Events,
    _topic: Topic,
    metrics: Arc<SubMetrics>,
}

impl<T> TypedSubscriber<T>
where
    T: Keyed + DeserializeOwned + 'static,
    <T as Keyed>::K: Key + DeserializeOwned,
{
    /// Wait up to `timeout` for the next sample. Returns `Ok(None)` on
    /// timeout. `Sample::Dispose` variants are surfaced as `Ok(None)` —
    /// callers that care about disposes should use the raw reader.
    pub fn take_next(&mut self, timeout: Duration) -> Result<Option<T>> {
        self.drain_status();
        let _ = self.poll.poll(&mut self.events, Some(timeout))?;
        for _event in self.events.iter() {
            loop {
                match self.reader.take_next_sample() {
                    Ok(Some(ds)) => match ds.into_value() {
                        Sample::Value(v) => return Ok(Some(v)),
                        Sample::Dispose(_) => continue,
                    },
                    Ok(None) => break,
                    Err(e) => return Err(anyhow::anyhow!("reader error: {e:?}")),
                }
            }
        }
        Ok(None)
    }

    /// Drain reader status events. Currently records
    /// `DataReaderStatus::SampleLost` into `metrics.lost_dds_local`.
    pub fn drain_status(&self) {
        while let Some(event) = self.reader.try_recv_status() {
            if let DataReaderStatus::SampleLost { count } = event {
                let n = count.count_change();
                if n > 0 {
                    self.metrics.record_sample_lost_dds(n as u64);
                }
            }
        }
    }

    pub fn metrics(&self) -> &Arc<SubMetrics> {
        &self.metrics
    }
}

impl RustDdsBackend {
    /// Create a typed publisher on a WITH_KEY topic. The topic is
    /// (re-)declared locally with the given type name and the inspector
    /// default QoS.
    pub fn create_typed_publisher<T>(
        &self,
        topic_name: &str,
        type_name: &str,
    ) -> Result<TypedPublisher<T>>
    where
        T: Keyed + Serialize + 'static,
        <T as Keyed>::K: Key,
    {
        let qos = default_inspector_qos();
        let topic = self
            .dp
            .create_topic(
                topic_name.to_string(),
                type_name.to_string(),
                &qos,
                TopicKind::WithKey,
            )
            .context("create_topic failed")?;
        let publisher = self.dp.create_publisher(&qos)?;
        let writer = publisher
            .create_datawriter_cdr::<T>(&topic, None)
            .context("create_datawriter failed")?;
        Ok(TypedPublisher {
            writer,
            _topic: topic,
            metrics: PubMetrics::new(),
            matches: MatchTable::new(),
        })
    }

    /// Create a typed subscriber on a WITH_KEY topic.
    pub fn create_typed_subscriber<T>(
        &self,
        topic_name: &str,
        type_name: &str,
    ) -> Result<TypedSubscriber<T>>
    where
        T: Keyed + DeserializeOwned + 'static,
        <T as Keyed>::K: Key + DeserializeOwned,
    {
        let qos = default_inspector_qos();
        let topic = self
            .dp
            .create_topic(
                topic_name.to_string(),
                type_name.to_string(),
                &qos,
                TopicKind::WithKey,
            )
            .context("create_topic failed")?;
        let subscriber = self.dp.create_subscriber(&qos)?;
        let mut reader = subscriber
            .create_datareader_cdr::<T>(&topic, Some(qos))
            .context("create_datareader failed")?;
        let poll = Poll::new()?;
        poll.registry()
            .register(&mut reader, Token(1), Interest::READABLE)?;
        Ok(TypedSubscriber {
            reader,
            poll,
            events: Events::with_capacity(8),
            _topic: topic,
            metrics: SubMetrics::new(),
        })
    }
}

// ============================================================================
// BytesAdapter — DeserializerAdapter that doesn't deserialize
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RawSample {
    bytes: Vec<u8>,
}

impl Keyed for RawSample {
    type K = ();
    fn key(&self) -> Self::K {}
}

#[derive(Clone, Copy)]
struct BytesDecoder;

#[derive(Clone, Copy)]
struct BytesAdapter;

const SUPPORTED_ENCODINGS: &[RepresentationIdentifier] = &[
    RepresentationIdentifier::CDR_LE,
    RepresentationIdentifier::CDR_BE,
    RepresentationIdentifier::PL_CDR_LE,
    RepresentationIdentifier::PL_CDR_BE,
];

impl NoKeyDeserializerAdapter<RawSample> for BytesAdapter {
    type Error = Infallible;
    type Decoded = RawSample;
    fn supported_encodings() -> &'static [RepresentationIdentifier] {
        SUPPORTED_ENCODINGS
    }
    fn transform_decoded(decoded: Self::Decoded) -> RawSample {
        decoded
    }
}

impl NoKeyDefaultDecoder<RawSample> for BytesAdapter {
    type Decoder = BytesDecoder;
    const DECODER: Self::Decoder = BytesDecoder;
}

impl NoKeyDecode<RawSample> for BytesDecoder {
    type Error = Infallible;
    fn decode_bytes(
        self,
        input_bytes: &[u8],
        _encoding: RepresentationIdentifier,
    ) -> Result<RawSample, Self::Error> {
        Ok(RawSample {
            bytes: input_bytes.to_vec(),
        })
    }
}

impl WithKeyDeserializerAdapter<RawSample> for BytesAdapter {
    type DecodedKey = ();
    fn transform_decoded_key(_decoded_key: Self::DecodedKey) {}
}

impl WithKeyDefaultDecoder<RawSample> for BytesAdapter {
    type Decoder = BytesDecoder;
    const DECODER: Self::Decoder = BytesDecoder;
}

impl WithKeyDecode<RawSample, ()> for BytesDecoder {
    fn decode_key_bytes(
        self,
        _input_key_bytes: &[u8],
        _encoding: RepresentationIdentifier,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

// ============================================================================
// Internal helpers — discovery + raw subscription
// ============================================================================

fn spawn_discovery_thread(dp: Arc<DomainParticipant>, tx: Sender<BackendEvent>) {
    thread::spawn(move || {
        let listener = dp.status_listener();
        loop {
            while let Some(event) = listener.try_recv_status() {
                if let Some(be) = convert_status_event(event) {
                    if tx.send(be).is_err() {
                        return;
                    }
                }
            }
            thread::sleep(Duration::from_millis(50));
        }
    });
}

fn convert_status_event(event: DomainParticipantStatusEvent) -> Option<BackendEvent> {
    use DomainParticipantStatusEvent as E;
    Some(match event {
        E::ParticipantDiscovered { dpd } => BackendEvent::ParticipantAdded(ParticipantDto {
            guid: hex_bytes(dpd.guid.prefix.as_ref()),
            entity_name: dpd.entity_name.clone(),
            vendor_id: pretty_vendor(dpd.vendor_id.as_bytes()),
        }),
        E::ParticipantLost { id, .. } => BackendEvent::ParticipantRemoved(hex_bytes(id.as_ref())),
        E::TopicDetected { name, type_name } => BackendEvent::TopicAdded(TopicDto {
            name,
            type_name,
        }),
        E::TopicLost { name } => BackendEvent::TopicRemoved(name),
        E::WriterDetected { writer } => BackendEvent::WriterAdded(endpoint_dto(&writer)),
        E::WriterLost { guid, .. } => BackendEvent::WriterRemoved(guid_string(&guid)),
        E::ReaderDetected { reader } => BackendEvent::ReaderAdded(endpoint_dto(&reader)),
        E::ReaderLost { guid, .. } => BackendEvent::ReaderRemoved(guid_string(&guid)),
        _ => return None,
    })
}

fn endpoint_dto(d: &rustdds::EndpointDescription) -> EndpointDto {
    EndpointDto {
        guid: guid_string(&d.guid),
        participant_guid: hex_bytes(d.guid.prefix.as_ref()),
        topic_name: d.topic_name.clone(),
        type_name: d.type_name.clone(),
        qos: serde_json::to_value(&d.qos).unwrap_or(serde_json::Value::Null),
    }
}

fn guid_string(g: &rustdds::GUID) -> String {
    let prefix = hex_bytes(g.prefix.as_ref());
    let entity = hex_bytes(&g.entity_id.to_slice());
    format!("{prefix}.{entity}")
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Map an OMG-assigned 2-byte VendorId to a clean product name. Source:
/// <https://www.dds-foundation.org/dds-rtps-vendor-and-product-ids/>.
/// Unknown IDs fall back to a 4-char hex tag so they're still
/// identifiable.
fn pretty_vendor(bytes: [u8; 2]) -> String {
    match bytes {
        [0x00, 0x00] => "unknown".to_string(),
        [0x01, 0x01] => "RTI Connext".to_string(),
        [0x01, 0x02] => "OpenSplice".to_string(),
        [0x01, 0x03] => "OpenDDS".to_string(),
        [0x01, 0x04] => "Mil-DDS".to_string(),
        [0x01, 0x05] => "InterCOM".to_string(),
        [0x01, 0x06] => "CoreDX".to_string(),
        [0x01, 0x09] => "Diamond DDS".to_string(),
        [0x01, 0x0A] => "RTI Connext Micro".to_string(),
        [0x01, 0x0B] => "Vortex Cafe".to_string(),
        [0x01, 0x0D] => "Vortex Lite".to_string(),
        [0x01, 0x0F] => "FastDDS".to_string(),
        [0x01, 0x10] => "Cyclone DDS".to_string(),
        [0x01, 0x11] => "GurumDDS".to_string(),
        [0x01, 0x12] => "RustDDS".to_string(),
        [0x01, 0x13] => "ZRDDS".to_string(),
        [0x01, 0x14] => "Dust DDS".to_string(),
        _ => format!("0x{:02x}{:02x}", bytes[0], bytes[1]),
    }
}

const READER_TOKEN: Token = Token(1);

fn run_raw_subscription(
    dp: &Arc<DomainParticipant>,
    topic_name: &str,
    type_name: &str,
    samples_tx: &Sender<BackendSample>,
    stop: &Arc<AtomicBool>,
) -> Result<()> {
    let qos = default_inspector_qos();
    eprintln!("subscribe '{topic_name}' (type='{type_name}') — trying WITH_KEY");
    match dp.create_topic(
        topic_name.to_string(),
        type_name.to_string(),
        &qos,
        TopicKind::WithKey,
    ) {
        Ok(topic) => {
            let subscriber = dp.create_subscriber(&qos)?;
            let mut reader = subscriber
                .create_datareader::<RawSample, BytesAdapter>(&topic, Some(qos))
                .context("create_datareader (with_key) failed")?;
            return run_with_key_loop(&mut reader, topic_name, samples_tx, stop);
        }
        Err(e) => eprintln!("  ✗ WITH_KEY create_topic failed: {e:?} — trying NO_KEY"),
    }

    let topic = dp
        .create_topic(
            topic_name.to_string(),
            type_name.to_string(),
            &qos,
            TopicKind::NoKey,
        )
        .context("create_topic (no_key) failed")?;
    let subscriber = dp.create_subscriber(&qos)?;
    let mut reader = subscriber
        .create_datareader_no_key::<RawSample, BytesAdapter>(&topic, Some(qos))
        .context("create_datareader_no_key failed")?;
    run_no_key_loop(&mut reader, topic_name, samples_tx, stop)
}

fn run_with_key_loop(
    reader: &mut rustdds::with_key::DataReader<RawSample, BytesAdapter>,
    topic_name: &str,
    samples_tx: &Sender<BackendSample>,
    stop: &Arc<AtomicBool>,
) -> Result<()> {
    let mut poll = Poll::new()?;
    poll.registry()
        .register(reader, READER_TOKEN, Interest::READABLE)?;
    let mut events = Events::with_capacity(8);
    let poll_timeout = Duration::from_millis(100);

    while !stop.load(Ordering::Relaxed) {
        let _ = poll.poll(&mut events, Some(poll_timeout));
        for event in &events {
            if event.token() == READER_TOKEN {
                loop {
                    match reader.take_next_sample() {
                        Ok(Some(ds)) => match ds.into_value() {
                            Sample::Value(raw) => {
                                if send_sample(samples_tx, topic_name, raw.bytes).is_err() {
                                    return Ok(());
                                }
                            }
                            Sample::Dispose(_) => {}
                        },
                        Ok(None) => break,
                        Err(e) => {
                            eprintln!("reader '{topic_name}' error: {e:?}");
                            break;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn run_no_key_loop(
    reader: &mut rustdds::no_key::DataReader<RawSample, BytesAdapter>,
    topic_name: &str,
    samples_tx: &Sender<BackendSample>,
    stop: &Arc<AtomicBool>,
) -> Result<()> {
    let mut poll = Poll::new()?;
    poll.registry()
        .register(reader, READER_TOKEN, Interest::READABLE)?;
    let mut events = Events::with_capacity(8);
    let poll_timeout = Duration::from_millis(100);

    while !stop.load(Ordering::Relaxed) {
        let _ = poll.poll(&mut events, Some(poll_timeout));
        for event in &events {
            if event.token() == READER_TOKEN {
                loop {
                    match reader.take_next_sample() {
                        Ok(Some(ds)) => {
                            let raw = ds.into_value();
                            if send_sample(samples_tx, topic_name, raw.bytes).is_err() {
                                return Ok(());
                            }
                        }
                        Ok(None) => break,
                        Err(e) => {
                            eprintln!("reader '{topic_name}' error: {e:?}");
                            break;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn send_sample(
    tx: &Sender<BackendSample>,
    topic_name: &str,
    bytes: Vec<u8>,
) -> std::result::Result<(), std::sync::mpsc::SendError<BackendSample>> {
    tx.send(BackendSample {
        topic: topic_name.to_string(),
        bytes,
        recv_ns: now_ns(),
    })
}

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

use bitcoin::OutPoint;
use bitcoin::Transaction;
use bitcoin::XOnlyPublicKey;
use event_log::db_handle::accessors::occurrence::ApplicationTypeID;
use event_log::db_handle::accessors::occurrence::Occurrence;
use event_log::db_handle::accessors::occurrence::OccurrenceConversionError;
use event_log::db_handle::accessors::occurrence::ToOccurrence;
use game_player_messages::PsbtString;
use ruma_serde::CanonicalJsonValue;
use sapio_base::serialization_helpers::SArc;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use simps::EventKey;

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    ModuleBytes(Vec<u8>),
    TransactionFinalized(String, Transaction),
    Rebind(OutPoint),
    SyntheticPeriodicActions(i64),
    NewRecompileTriggeringObservation(Value, SArc<EventKey>),
    // strictly speaking we don't need this to be an event with any information.
    EmittedPSBTVia(PsbtString, XOnlyPublicKey),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Tag {
    InitModule,
    EvLoopCounter(u64),
    ScopedCounter(String, u64),
    ScopedValue(String, String),
}

impl ToString for Tag {
    fn to_string(&self) -> String {
        ruma_serde::to_canonical_value(self)
            .expect("Tag Type Should not have Fallible Serialization")
            .to_string()
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TaggedEvent(pub Event, pub Option<Tag>);

impl ToOccurrence for TaggedEvent {
    fn to_data(&self) -> CanonicalJsonValue {
        ruma_serde::to_canonical_value(&self.0).unwrap()
    }
    fn stable_typeid() -> ApplicationTypeID {
        ApplicationTypeID::from_inner("LitigatorEvent")
    }
    fn unique_tag(&self) -> Option<String> {
        self.1.as_ref().map(ToString::to_string)
    }
    fn from_occurrence(occurrence: Occurrence) -> Result<TaggedEvent, OccurrenceConversionError>
    where
        Self: Sized + for<'de> Deserialize<'de>,
    {
        let v: Event = serde_json::from_value(occurrence.data.into())
            .map_err(OccurrenceConversionError::DeserializationError)?;
        if occurrence.typeid != Self::stable_typeid() {
            return Err(OccurrenceConversionError::TypeidMismatch {
                expected: occurrence.typeid,
                got: Self::stable_typeid(),
            });
        }
        let tag = occurrence
            .unique_tag
            .map(|t| serde_json::from_str(&t))
            .transpose()
            .map_err(OccurrenceConversionError::DeserializationError)?;
        Ok(TaggedEvent(v, tag))
    }
}

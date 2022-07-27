mod owned;
pub use owned::*;

use crossbeam_channel::Sender;
use slog::{Record, KV};
use std::fmt;

pub struct Drain {
    tx: Sender<OwnedRecord>,
}

impl Drain {
    #[inline]
    pub fn new(data: DrainData) -> Self {
        Drain { tx: data.0 }
    }
}

#[derive(Debug)]
pub enum Error {
    Serialization(slog::Error),
    Send,
}

impl slog::Drain for Drain {
    type Ok = ();
    type Err = Error;

    #[inline]
    fn log(
        &self,
        record: &Record,
        logger_values: &slog::OwnedKVList,
    ) -> Result<Self::Ok, Self::Err> {
        let mut ser = ToOwnedSerializer(OwnedKVList(Vec::new()));
        record
            .kv()
            .serialize(record, &mut ser)
            .map_err(Error::Serialization)?;
        self.tx
            .send(OwnedRecord {
                msg: fmt::format(*record.msg()),
                location: *record.location(),
                tag: record.tag().to_string(),
                level: record.level(),
                kv: ser.0,
                logger_values: logger_values.clone(),
            })
            .map_err(|_| Error::Send)
    }
}

#[derive(Clone)]
pub struct DrainData(Sender<OwnedRecord>);

pub struct Receiver(crossbeam_channel::Receiver<OwnedRecord>);

impl Receiver {
    #[inline]
    pub fn try_iter(&self) -> impl IntoIterator<Item = OwnedRecord> + '_ {
        self.0.try_iter()
    }
}

#[inline]
pub fn init() -> (DrainData, Receiver) {
    let (tx, rx) = crossbeam_channel::unbounded();
    (DrainData(tx), Receiver(rx))
}

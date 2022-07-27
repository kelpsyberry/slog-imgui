use slog::{Key, Level, Record, RecordLocation, Serializer, KV};
use std::fmt;

pub enum OwnedValue {
    None,
    Unit,
    Bool(bool),
    Char(char),
    String(String),
    U64(u64),
    I64(i64),
    F32(f32),
    F64(f64),
}

pub struct OwnedKVList(pub Vec<(Key, OwnedValue)>);

impl KV for OwnedKVList {
    #[inline]
    fn serialize(&self, _record: &Record, serializer: &mut dyn Serializer) -> slog::Result {
        for (key, val) in &self.0 {
            match val {
                OwnedValue::None => serializer.emit_none(key)?,
                OwnedValue::Unit => serializer.emit_unit(key)?,
                &OwnedValue::Bool(val) => serializer.emit_bool(key, val)?,
                &OwnedValue::Char(val) => serializer.emit_char(key, val)?,
                OwnedValue::String(val) => serializer.emit_str(key, val)?,
                &OwnedValue::U64(val) => serializer.emit_u64(key, val)?,
                &OwnedValue::I64(val) => serializer.emit_i64(key, val)?,
                &OwnedValue::F32(val) => serializer.emit_f32(key, val)?,
                &OwnedValue::F64(val) => serializer.emit_f64(key, val)?,
            }
        }
        Ok(())
    }
}

pub struct OwnedRecord {
    pub msg: String,
    pub location: RecordLocation,
    pub tag: String,
    pub level: Level,
    pub kv: OwnedKVList,
    pub logger_values: slog::OwnedKVList,
}

pub(super) struct ToOwnedSerializer(pub OwnedKVList);

impl Serializer for ToOwnedSerializer {
    #[inline]
    fn emit_none(&mut self, key: Key) -> slog::Result {
        self.0 .0.push((key, OwnedValue::None));
        Ok(())
    }
    #[inline]
    fn emit_unit(&mut self, key: Key) -> slog::Result {
        self.0 .0.push((key, OwnedValue::Unit));
        Ok(())
    }
    #[inline]
    fn emit_bool(&mut self, key: Key, val: bool) -> slog::Result {
        self.0 .0.push((key, OwnedValue::Bool(val)));
        Ok(())
    }
    #[inline]
    fn emit_char(&mut self, key: Key, val: char) -> slog::Result {
        self.0 .0.push((key, OwnedValue::Char(val)));
        Ok(())
    }
    #[inline]
    fn emit_str(&mut self, key: Key, val: &str) -> slog::Result {
        self.0 .0.push((key, OwnedValue::String(val.to_string())));
        Ok(())
    }
    #[inline]
    fn emit_usize(&mut self, key: Key, val: usize) -> slog::Result {
        self.0 .0.push((key, OwnedValue::U64(val as u64)));
        Ok(())
    }
    #[inline]
    fn emit_isize(&mut self, key: Key, val: isize) -> slog::Result {
        self.0 .0.push((key, OwnedValue::I64(val as i64)));
        Ok(())
    }
    #[inline]
    fn emit_u8(&mut self, key: Key, val: u8) -> slog::Result {
        self.0 .0.push((key, OwnedValue::U64(val as u64)));
        Ok(())
    }
    #[inline]
    fn emit_i8(&mut self, key: Key, val: i8) -> slog::Result {
        self.0 .0.push((key, OwnedValue::I64(val as i64)));
        Ok(())
    }
    #[inline]
    fn emit_u16(&mut self, key: Key, val: u16) -> slog::Result {
        self.0 .0.push((key, OwnedValue::U64(val as u64)));
        Ok(())
    }
    #[inline]
    fn emit_i16(&mut self, key: Key, val: i16) -> slog::Result {
        self.0 .0.push((key, OwnedValue::I64(val as i64)));
        Ok(())
    }
    #[inline]
    fn emit_u32(&mut self, key: Key, val: u32) -> slog::Result {
        self.0 .0.push((key, OwnedValue::U64(val as u64)));
        Ok(())
    }
    #[inline]
    fn emit_i32(&mut self, key: Key, val: i32) -> slog::Result {
        self.0 .0.push((key, OwnedValue::I64(val as i64)));
        Ok(())
    }
    #[inline]
    fn emit_u64(&mut self, key: Key, val: u64) -> slog::Result {
        self.0 .0.push((key, OwnedValue::U64(val)));
        Ok(())
    }
    #[inline]
    fn emit_i64(&mut self, key: Key, val: i64) -> slog::Result {
        self.0 .0.push((key, OwnedValue::I64(val)));
        Ok(())
    }
    #[inline]
    fn emit_f32(&mut self, key: Key, val: f32) -> slog::Result {
        self.0 .0.push((key, OwnedValue::F32(val)));
        Ok(())
    }
    #[inline]
    fn emit_f64(&mut self, key: Key, val: f64) -> slog::Result {
        self.0 .0.push((key, OwnedValue::F64(val)));
        Ok(())
    }
    #[inline]
    fn emit_arguments(&mut self, key: Key, val: &fmt::Arguments) -> slog::Result {
        self.0 .0.push((key, OwnedValue::String(fmt::format(*val))));
        Ok(())
    }
}

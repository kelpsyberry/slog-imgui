use slog::{Key, Serializer};
use std::fmt::{self, Write as _};

pub struct StringSerializer<'a> {
    pub buffer: &'a mut String,
    pub comma_needed: bool,
}

macro_rules! emit(
	($self: expr, $key: expr, $value: expr) => {{
        if $self.comma_needed {
            $self.buffer.push_str(", ");
        }
        $self.buffer.push_str($key);
        $self.buffer.push_str(": ");
        write!($self.buffer, "{}", $value)?;
        Ok(())
	}};
);

impl<'a> Serializer for StringSerializer<'a> {
    #[inline]
    fn emit_none(&mut self, key: Key) -> slog::Result {
        emit!(self, key, "None")
    }
    #[inline]
    fn emit_unit(&mut self, key: Key) -> slog::Result {
        emit!(self, key, "()")
    }
    #[inline]
    fn emit_bool(&mut self, key: Key, val: bool) -> slog::Result {
        emit!(self, key, val)
    }
    #[inline]
    fn emit_char(&mut self, key: Key, val: char) -> slog::Result {
        emit!(self, key, val)
    }
    #[inline]
    fn emit_usize(&mut self, key: Key, val: usize) -> slog::Result {
        emit!(self, key, val)
    }
    #[inline]
    fn emit_isize(&mut self, key: Key, val: isize) -> slog::Result {
        emit!(self, key, val)
    }
    #[inline]
    fn emit_u8(&mut self, key: Key, val: u8) -> slog::Result {
        emit!(self, key, val)
    }
    #[inline]
    fn emit_i8(&mut self, key: Key, val: i8) -> slog::Result {
        emit!(self, key, val)
    }
    #[inline]
    fn emit_u16(&mut self, key: Key, val: u16) -> slog::Result {
        emit!(self, key, val)
    }
    #[inline]
    fn emit_i16(&mut self, key: Key, val: i16) -> slog::Result {
        emit!(self, key, val)
    }
    #[inline]
    fn emit_u32(&mut self, key: Key, val: u32) -> slog::Result {
        emit!(self, key, val)
    }
    #[inline]
    fn emit_i32(&mut self, key: Key, val: i32) -> slog::Result {
        emit!(self, key, val)
    }
    #[inline]
    fn emit_f32(&mut self, key: Key, val: f32) -> slog::Result {
        emit!(self, key, val)
    }
    #[inline]
    fn emit_u64(&mut self, key: Key, val: u64) -> slog::Result {
        emit!(self, key, val)
    }
    #[inline]
    fn emit_i64(&mut self, key: Key, val: i64) -> slog::Result {
        emit!(self, key, val)
    }
    #[inline]
    fn emit_f64(&mut self, key: Key, val: f64) -> slog::Result {
        emit!(self, key, val)
    }
    #[inline]
    fn emit_str(&mut self, key: Key, val: &str) -> slog::Result {
        emit!(self, key, val)
    }
    #[inline]
    fn emit_arguments(&mut self, key: Key, val: &fmt::Arguments) -> slog::Result {
        emit!(self, key, val)
    }
}

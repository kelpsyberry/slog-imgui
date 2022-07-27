use super::{history, History};
use slog::{ser::Serializer, Key};
use std::fmt;

type StringKV = (Key, String);

#[derive(Default)]
pub struct LoggerKVGroupsSerializer {
    cur_kv_groups: Vec<(history::NodeId, StringKV)>,
    kv_buf: Vec<StringKV>,
}

impl LoggerKVGroupsSerializer {
    pub fn clear(&mut self) {
        self.cur_kv_groups.clear();
    }

    pub fn finish(&mut self, history: &mut History) -> (u16, history::NodeId) {
        let mut parent = history::NodeId::MAX;

        if self.kv_buf.is_empty() {
            self.cur_kv_groups.clear();
            return (0, parent);
        }

        for (i, kv) in self.kv_buf.drain(..).rev().enumerate() {
            if let Some((id, prev_kv)) = self.cur_kv_groups.get(i) {
                if *prev_kv == kv {
                    parent = *id;
                    continue;
                }
            }

            if parent != history::NodeId::MAX {
                history.groups.get_mut(&parent).unwrap().ref_count += 1;
            }

            let id = history.next_group_id;
            history.groups.insert(
                id,
                history::Group {
                    parent,
                    filtered_parent: parent,
                    ref_count: 0,
                    filtered_ref_count: 0,
                    kv_str: format!("{}: {}", kv.0, kv.1),
                },
            );
            history.next_group_id += 1;

            history.all.push(history::Node {
                indent: i as u16,
                kind: history::NodeKind::Group,
                id,
            });

            self.cur_kv_groups.truncate(i);
            self.cur_kv_groups.push((id, kv));
            parent = id;
        }
        (self.cur_kv_groups.len() as u16, parent)
    }
}

macro_rules! emit(
	($self: expr, $key: expr, $value: expr) => {{
        $self.kv_buf.push(($key, format!("{}", $value)));
        Ok(())
	}};
);

impl Serializer for LoggerKVGroupsSerializer {
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

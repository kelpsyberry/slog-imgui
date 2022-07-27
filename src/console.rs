mod string_ser;
use string_ser::StringSerializer;
mod logger_kv_group_ser;
use logger_kv_group_ser::LoggerKVGroupsSerializer;
mod history;
use history::History;
mod filter_data;
use filter_data::FilterData;

#[cfg(any(feature = "async"))]
use crate::async_drain::OwnedRecord;
use imgui::{FontId, StyleColor, Ui};
#[cfg(feature = "async")]
use slog::RecordStatic;
use slog::{Level, Record, KV};
use std::fmt;

#[derive(Clone, Copy, Debug)]
pub struct LevelColors {
    pub critical: [f32; 4],
    pub error: [f32; 4],
    pub warning: [f32; 4],
    pub info: [f32; 4],
    pub debug: [f32; 4],
    pub trace: [f32; 4],
}

impl LevelColors {
    pub const fn new() -> Self {
        LevelColors {
            critical: [0.75, 0., 0., 1.],
            error: [1., 0., 0., 1.],
            warning: [0.9, 0.9, 0., 1.],
            info: [1., 1., 1., 1.],
            debug: [0., 0.87, 1., 1.],
            trace: [0.75, 0.75, 0.75, 1.],
        }
    }
}

impl Default for LevelColors {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Builder {
    pub show_options: bool,
    pub msg_filter: String,
    pub kv_filter: Vec<String>,
    pub locked_to_bottom: bool,
    pub history_capacity: usize,
    pub level_colors: LevelColors,
}

impl Builder {
    pub const fn new() -> Self {
        Builder {
            show_options: true,
            msg_filter: String::new(),
            kv_filter: Vec::new(),
            locked_to_bottom: true,
            history_capacity: 1024 * 1024,
            level_colors: LevelColors::new(),
        }
    }

    pub fn build(self) -> Console {
        Console {
            history: History::default(),
            logger_kv_groups_ser: LoggerKVGroupsSerializer::default(),

            locked_to_bottom: self.locked_to_bottom,
            history_capacity: self.history_capacity,
            level_colors: self.level_colors,
            options_vis: if self.show_options {
                OptionsVisibility::Shown {
                    msg_filter_buf: self.msg_filter.clone(),
                    kv_filter_buf: self.kv_filter.join(", "),
                }
            } else {
                OptionsVisibility::Hidden
            },

            filter_data: FilterData::new(self.msg_filter, self.kv_filter),
        }
    }
}

enum OptionsVisibility {
    Shown {
        msg_filter_buf: String,
        kv_filter_buf: String,
    },
    Hidden,
}

pub struct Console {
    history: History,
    logger_kv_groups_ser: LoggerKVGroupsSerializer,

    pub locked_to_bottom: bool,
    pub history_capacity: usize,
    pub level_colors: LevelColors,
    options_vis: OptionsVisibility,

    filter_data: FilterData,
}

impl Console {
    pub fn render_window(
        &mut self,
        ui: &Ui,
        font: Option<FontId>,
        text_spacing: f32,
        text_padding: f32,
        opened: &mut bool,
    ) {
        ui.window("Log").opened(opened).build(|| {
            self.render_options(ui);
            ui.child_window("log_contents").build(|| {
                let _font_token = font.map(|font| ui.push_font(font));
                let _item_spacing =
                    ui.push_style_var(imgui::StyleVar::ItemSpacing([0.0, text_spacing]));
                let _frame_padding =
                    ui.push_style_var(imgui::StyleVar::FramePadding([text_padding; 2]));
                self.render(ui);
            });
        });
    }

    pub fn render_options(&mut self, ui: &Ui) {
        if let OptionsVisibility::Shown {
            msg_filter_buf,
            kv_filter_buf,
        } = &mut self.options_vis
        {
            let (frame_padding, item_spacing) = unsafe {
                let style = ui.style();
                (style.frame_padding, style.item_spacing)
            };

            ui.checkbox("Lock", &mut self.locked_to_bottom);

            let clear_button_width = ui.calc_text_size("Clear")[0] + frame_padding[0] * 2.0;

            ui.same_line();

            let filter_field_width =
                (ui.content_region_avail()[0] - clear_button_width - item_spacing[0] * 2.0) * 0.5;

            ui.set_next_item_width(filter_field_width);
            if ui
                .input_text("##msg_filter", msg_filter_buf)
                .hint("Message filter")
                .build()
            {
                Self::update_msg_filter(
                    &mut self.history,
                    &mut self.filter_data,
                    msg_filter_buf.clone(),
                );
            }

            ui.same_line();
            ui.set_next_item_width(filter_field_width);
            if ui
                .input_text("##kv_filter", kv_filter_buf)
                .hint("Group filter")
                .build()
            {
                Self::update_kv_filter(
                    &mut self.history,
                    &mut self.filter_data,
                    if kv_filter_buf.is_empty() {
                        Vec::new()
                    } else {
                        kv_filter_buf
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .collect()
                    },
                );
            }

            ui.same_line();
            if ui.button_with_size("Clear", [clear_button_width, 0.0]) {
                self.clear();
            }

            ui.dummy([0.0, 6.0]);
            ui.separator();
            ui.dummy([0.0, 6.0]);
        }
    }

    pub fn render(&mut self, ui: &Ui) {
        let history = if self.filter_data.filtering_enabled() {
            &self.history.filtered
        } else {
            &self.history.all
        };

        let line_height = ui.frame_height_with_spacing() as f64;
        let history_height = history.len() as f64 * line_height;
        let window_height = ui.window_size()[1] as f64;

        if self.locked_to_bottom {
            ui.set_scroll_y((history_height - window_height) as f32);
        }

        let top_y = ui.scroll_y() as f64;
        let bot_y = top_y + window_height;

        let y_offset = if self.locked_to_bottom {
            (history_height - bot_y).max(0.0)
        } else {
            0.0
        };

        let start_i = (((top_y + y_offset) / line_height).floor() as usize).min(history.len());
        let end_i = (((bot_y + y_offset) / line_height).ceil() as usize).min(history.len());

        let (indent_spacing, frame_padding) = unsafe {
            let style = ui.style();
            (style.indent_spacing, style.frame_padding)
        };

        ui.dummy([0.0, (start_i as f64 * line_height - y_offset) as f32]);

        for (i, node) in history
            .iter()
            .enumerate()
            .skip(start_i)
            .take(end_i - start_i)
        {
            let cursor_pos = [0.0, (i as f64 * line_height - y_offset) as f32];
            ui.set_cursor_pos(cursor_pos);

            let indent = node.indent as f32 * indent_spacing;

            let (text, text_color) = unsafe {
                match node.kind {
                    history::NodeKind::Group => (
                        &self.history.groups.get(&node.id).unwrap_unchecked().kv_str,
                        ui.style_color(StyleColor::Text),
                    ),

                    history::NodeKind::Leaf => {
                        let leaf = self
                            .history
                            .leaves
                            .get_unchecked((node.id - self.history.cur_leaf_base_id) as usize);
                        (
                            &leaf.msg,
                            match leaf.level {
                                Level::Critical => self.level_colors.critical,
                                Level::Error => self.level_colors.error,
                                Level::Warning => self.level_colors.warning,
                                Level::Info => self.level_colors.info,
                                Level::Debug => self.level_colors.debug,
                                Level::Trace => self.level_colors.trace,
                            },
                        )
                    }
                }
            };

            let _id = ui.push_id_usize(i);

            let text_size = ui.calc_text_size(text);
            let frame_size = [0, 1].map(|i| text_size[i] + frame_padding[i] * 2.0);

            if ui.invisible_button("", [frame_size[0] + indent, frame_size[1]]) {
                ui.set_clipboard_text(text);
            }

            let color = if ui.is_item_active() {
                Some(ui.style_color(StyleColor::ButtonActive))
            } else if ui.is_item_hovered() {
                Some(ui.style_color(StyleColor::ButtonHovered))
            } else {
                None
            };

            if let Some(mut color) = color {
                color[3] *= 0.5;

                let window_pos = ui.window_pos();
                let start = [
                    window_pos[0] - ui.scroll_x() + cursor_pos[0] + indent,
                    window_pos[1] - ui.scroll_y() + cursor_pos[1],
                ];

                ui.get_window_draw_list()
                    .add_rect(
                        start,
                        [start[0] + frame_size[0], start[1] + frame_size[1]],
                        color,
                    )
                    .filled(true)
                    .rounding(unsafe { ui.style() }.frame_rounding)
                    .build();
            }

            ui.set_cursor_pos([
                cursor_pos[0] + frame_padding[0] + indent,
                cursor_pos[1] + frame_padding[1],
            ]);
            ui.text_colored(text_color, text);
        }

        ui.set_cursor_pos([0.0, (end_i as f64 * line_height - y_offset) as f32]);
        ui.dummy([
            0.0,
            ((history.len() - end_i) as f64 * line_height + y_offset) as f32,
        ]);
    }
}

impl Console {
    fn process_record(
        &mut self,
        record: &Record,
        logger_values: &slog::OwnedKVList,
    ) -> Result<(), slog::Error> {
        let (indent, group_id) = {
            logger_values.serialize(record, &mut self.logger_kv_groups_ser)?;
            self.logger_kv_groups_ser.finish(&mut self.history)
        };

        if group_id != history::NodeId::MAX {
            self.history.groups.get_mut(&group_id).unwrap().ref_count += 1;
        }

        let mut msg = fmt::format(*record.msg());
        record.kv().serialize(
            record,
            &mut StringSerializer {
                comma_needed: !msg.is_empty(),
                buffer: &mut msg,
            },
        )?;

        let id = self.history.next_leaf_id();
        let leaf = history::Leaf {
            parent: group_id,
            filtered_parent: group_id,
            level: record.level(),
            msg,
        };
        if self.filter_data.filtering_enabled() {
            self.filter_data.filter_new_message()(self, indent, id, &leaf);
        }
        self.history.leaves.push(leaf);
        self.history.all.push(history::Node {
            indent,
            kind: history::NodeKind::Leaf,
            id,
        });

        Ok(())
    }

    fn finish_processing_records(&mut self) {
        if self.history.leaves.len() > self.history_capacity {
            self.history
                .remove_leaves_before(self.history.leaves.len() - self.history_capacity);
        }
    }

    #[cfg(any(feature = "async"))]
    #[cfg_attr(feature = "nightly", doc(cfg(feature = "async")))]
    pub fn process_async(
        &mut self,
        records: impl IntoIterator<Item = OwnedRecord>,
    ) -> Result<(), slog::Error> {
        for record in records.into_iter() {
            self.process_record(
                &Record::new(
                    &RecordStatic {
                        location: &record.location,
                        tag: &record.tag,
                        level: record.level,
                    },
                    &format_args!("{}", record.msg),
                    slog::BorrowedKV(&record.kv),
                ),
                &record.logger_values,
            )?;
        }
        self.finish_processing_records();
        Ok(())
    }

    pub fn process_sync<'a>(
        &mut self,
        records: impl IntoIterator<Item = (&'a Record<'a>, &'a slog::OwnedKVList)>,
    ) -> Result<(), slog::Error> {
        for (record, logger_values) in records.into_iter() {
            self.process_record(record, logger_values)?;
        }
        self.finish_processing_records();
        Ok(())
    }
}

impl Console {
    #[inline]
    pub fn clear(&mut self) {
        self.logger_kv_groups_ser.clear();
        self.history.clear();
    }

    fn update_msg_filter(history: &mut History, filter_data: &mut FilterData, new: String) {
        let filtering_was_enabled = filter_data.filtering_enabled();
        let prev = filter_data.set_msg_filter(new);
        let new = filter_data.msg_filter();

        if !filter_data.filtering_enabled() {
            history.clear_filtered();
            return;
        }

        if filtering_was_enabled && new.contains(&prev) {
            history.apply_msg_filter_restriction(new);
        } else {
            filter_data.filter_all()(history, filter_data);
        }

        history.clean_filtered_groups();
    }

    fn update_kv_filter(history: &mut History, filter_data: &mut FilterData, new: Vec<String>) {
        let filtering_was_enabled = filter_data.filtering_enabled();
        let prev = filter_data.set_kv_filter(new);
        let new = filter_data.kv_filter();

        if !filter_data.filtering_enabled() {
            history.clear_filtered();
            return;
        }

        if filtering_was_enabled && prev.iter().all(|elem| new.contains(elem)) {
            history.apply_kv_filter_restriction(new);
        } else {
            filter_data.filter_all()(history, filter_data);
        }

        history.clean_filtered_groups();
    }

    fn filter_all<const MSG_ENABLED: bool, const KV_ENABLED: bool>(
        history: &mut History,
        filter_data: &mut FilterData,
    ) {
        history.filter_all::<MSG_ENABLED, KV_ENABLED>(
            filter_data.msg_filter(),
            filter_data.kv_filter(),
        );
    }

    fn filter_new_message<const MSG_ENABLED: bool, const KV_ENABLED: bool>(
        &mut self,
        indent: u16,
        id: u64,
        leaf: &history::Leaf,
    ) {
        self.history.filter_new_message::<MSG_ENABLED, KV_ENABLED>(
            indent,
            id,
            leaf,
            self.filter_data.msg_filter(),
            self.filter_data.kv_filter(),
        );
    }
}

impl Console {
    #[inline]
    pub fn show_options(&self) -> bool {
        matches!(&self.options_vis, OptionsVisibility::Shown { .. })
    }

    #[inline]
    pub fn set_show_options(&mut self, value: bool) {
        self.options_vis = if value {
            OptionsVisibility::Shown {
                msg_filter_buf: String::new(),
                kv_filter_buf: String::new(),
            }
        } else {
            OptionsVisibility::Hidden
        }
    }

    #[inline]
    pub fn msg_filter(&self) -> &str {
        self.filter_data.msg_filter()
    }

    #[inline]
    pub fn set_msg_filter(&mut self, value: String) {
        if let OptionsVisibility::Shown { msg_filter_buf, .. } = &mut self.options_vis {
            msg_filter_buf.clear();
            msg_filter_buf.push_str(&value);
        }
        Self::update_msg_filter(&mut self.history, &mut self.filter_data, value);
    }

    #[inline]
    pub fn kv_filter(&self) -> &[String] {
        self.filter_data.kv_filter()
    }

    #[inline]
    pub fn set_kv_filter(&mut self, value: Vec<String>) {
        if let OptionsVisibility::Shown { kv_filter_buf, .. } = &mut self.options_vis {
            kv_filter_buf.clear();
            if let Some((last, elems)) = value.split_last() {
                for elem in elems {
                    kv_filter_buf.push_str(elem);
                    kv_filter_buf.push_str(", ");
                }
                kv_filter_buf.push_str(last);
            }
        }
        Self::update_kv_filter(&mut self.history, &mut self.filter_data, value);
    }
}

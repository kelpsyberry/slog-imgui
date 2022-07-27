use super::{history, Console, History};
use std::mem::replace;

type FilterAllFn = fn(&mut History, &mut FilterData);

static FILTER_ALL_FNS: [FilterAllFn; 4] = [
    Console::filter_all::<true, true>,
    Console::filter_all::<true, false>,
    Console::filter_all::<false, true>,
    Console::filter_all::<false, false>,
];

type FilterNewMessageFn = fn(&mut Console, u16, history::NodeId, &history::Leaf);
static FILTER_NEW_MESSAGE_FNS: [FilterNewMessageFn; 4] = [
    Console::filter_new_message::<true, true>,
    Console::filter_new_message::<true, false>,
    Console::filter_new_message::<false, true>,
    Console::filter_new_message::<false, false>,
];

pub struct FilterData {
    filtering_enabled: bool,
    msg_filter: String,
    kv_filter: Vec<String>,
    filter_all: FilterAllFn,
    filter_new_message: FilterNewMessageFn,
}

impl FilterData {
    pub fn new(msg_filter: String, kv_filter: Vec<String>) -> Self {
        let fn_key = (msg_filter.is_empty() as usize) << 1 | kv_filter.is_empty() as usize;
        FilterData {
            filtering_enabled: !(msg_filter.is_empty() && kv_filter.is_empty()),
            msg_filter,
            kv_filter,
            filter_all: FILTER_ALL_FNS[fn_key],
            filter_new_message: FILTER_NEW_MESSAGE_FNS[fn_key],
        }
    }

    pub fn filtering_enabled(&self) -> bool {
        self.filtering_enabled
    }

    fn update_filters(&mut self) {
        self.filtering_enabled = !(self.msg_filter.is_empty() && self.kv_filter.is_empty());
        let fn_key =
            (self.msg_filter.is_empty() as usize) << 1 | self.kv_filter.is_empty() as usize;
        self.filter_all = FILTER_ALL_FNS[fn_key];
        self.filter_new_message = FILTER_NEW_MESSAGE_FNS[fn_key];
    }

    pub fn msg_filter(&self) -> &str {
        &self.msg_filter
    }

    pub fn set_msg_filter(&mut self, value: String) -> String {
        let prev = replace(&mut self.msg_filter, value);
        self.update_filters();
        prev
    }

    pub fn kv_filter(&self) -> &[String] {
        &self.kv_filter
    }

    pub fn set_kv_filter(&mut self, value: Vec<String>) -> Vec<String> {
        let prev = replace(&mut self.kv_filter, value);
        self.update_filters();
        prev
    }

    pub fn filter_all(&self) -> FilterAllFn {
        self.filter_all
    }

    pub fn filter_new_message(&self) -> FilterNewMessageFn {
        self.filter_new_message
    }
}

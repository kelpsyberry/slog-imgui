use ahash::AHashMap as HashMap;
use slog::Level;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Leaf,
    Group,
}

pub type NodeId = u64;

#[derive(Clone, Copy)]
pub struct Node {
    pub indent: u16,
    pub kind: NodeKind,
    pub id: NodeId,
}

#[derive(Clone)]
pub struct Group {
    pub parent: NodeId,
    pub filtered_parent: NodeId,
    pub ref_count: u32,
    pub filtered_ref_count: u32,
    pub kv_str: String,
}

#[derive(Clone)]
pub struct Leaf {
    pub parent: NodeId,
    pub filtered_parent: NodeId,
    pub level: Level,
    pub msg: String,
}

#[derive(Default)]
pub struct History {
    pub next_group_id: NodeId,
    pub groups: HashMap<NodeId, Group>,
    pub cur_leaf_base_id: NodeId,
    pub leaves: Vec<Leaf>,
    pub all: Vec<Node>,
    pub filtered: Vec<Node>,
}

enum RetainUntilResult {
    Remove,
    Retain,
    RetainAndStop,
}

fn retain_until<T>(values: &mut Vec<T>, mut f: impl FnMut(&T) -> RetainUntilResult) {
    unsafe {
        let mut dst_i = 0;
        for src_i in 0..values.len() {
            let elem = values.as_ptr().add(src_i);

            match f(&*elem) {
                RetainUntilResult::Remove => drop(elem.read()),

                RetainUntilResult::Retain => {
                    values.as_mut_ptr().add(dst_i).write(elem.read());
                    dst_i += 1;
                }

                RetainUntilResult::RetainAndStop => {
                    let copy_len = values.len() - src_i;
                    values.as_mut_ptr().add(dst_i).copy_from(elem, copy_len);
                    values.set_len(dst_i + copy_len);
                    return;
                }
            }
        }
    }
}

macro_rules! increase_ref_count {
    ($init_parent_id: expr, $groups: expr, $ref_count: ident, $parent: ident) => {{
        let mut parent_id = $init_parent_id;
        while parent_id != NodeId::MAX {
            let parent = $groups.get_mut(&parent_id).unwrap_unchecked();
            parent.$ref_count += 1;
            if parent.$ref_count != 1 {
                break;
            }
            parent_id = parent.$parent;
        }
    }};
}

macro_rules! decrease_ref_count {
    ($init_parent_id: expr, $groups: expr, $ref_count: ident, $parent: ident) => {{
        let mut parent_id = $init_parent_id;
        while parent_id != NodeId::MAX {
            let parent = $groups.get_mut(&parent_id).unwrap_unchecked();
            parent.$ref_count -= 1;
            if parent.$ref_count != 0 {
                break;
            }
            parent_id = parent.$parent;
        }
    }};
}

impl History {
    pub fn clear(&mut self) {
        self.next_group_id = 0;
        self.groups.clear();
        self.cur_leaf_base_id = 0;
        self.leaves.clear();
        self.all.clear();
        self.filtered.clear();
    }

    pub fn clear_filtered(&mut self) {
        self.filtered.clear();

        for group in self.groups.values_mut() {
            group.filtered_ref_count = 0;
            group.filtered_parent = group.parent;
        }

        for leaf in &mut self.leaves {
            leaf.filtered_parent = leaf.parent;
        }
    }

    pub fn next_leaf_id(&self) -> NodeId {
        self.cur_leaf_base_id + self.leaves.len() as NodeId
    }

    pub fn remove_leaves_before(&mut self, start_pos: usize) {
        let prev_leaf_base_id = self.cur_leaf_base_id;
        self.cur_leaf_base_id += start_pos as NodeId;

        macro_rules! retain_nodes {
            ($history: ident, $ref_count: ident, $parent: ident) => {
                retain_until(&mut self.$history, |node| match node.kind {
                    NodeKind::Leaf => {
                        if node.id >= self.cur_leaf_base_id {
                            return RetainUntilResult::RetainAndStop;
                        }

                        unsafe {
                            decrease_ref_count!(
                                self.leaves
                                    .get_unchecked((node.id - prev_leaf_base_id) as usize)
                                    .$parent,
                                self.groups,
                                $ref_count,
                                $parent
                            );
                        }
                        RetainUntilResult::Remove
                    }

                    NodeKind::Group => RetainUntilResult::Retain,
                });

                self.$history.retain(|node| match node.kind {
                    NodeKind::Leaf => true,
                    NodeKind::Group => unsafe {
                        let group = self.groups.get(&node.id).unwrap_unchecked();
                        if group.$ref_count == 0 {
                            decrease_ref_count!(group.$parent, self.groups, $ref_count, $parent);
                            false
                        } else {
                            true
                        }
                    },
                });
            };
        }

        retain_nodes!(all, ref_count, parent);
        retain_nodes!(filtered, filtered_ref_count, filtered_parent);

        self.leaves.drain(..start_pos);
        self.groups.retain(|_, group| group.ref_count != 0);
    }

    fn filter_node<
        'a,
        const MSG_ENABLED: bool,
        const KV_ENABLED: bool,
        const INCREASE_REF_COUNT: bool,
    >(
        leaves: &'a [Leaf],
        leaf_base_id: NodeId,
        groups: &'a mut HashMap<NodeId, Group>,
        msg_filter: &'a str,
        kv_filter: &'a [String],
    ) -> impl FnMut(&Node) -> bool + 'a {
        let mut kv_filter_satisfied = vec![(false, 0); kv_filter.len()];

        move |node| {
            if KV_ENABLED {
                for (satisfied, satisfied_indent) in &mut kv_filter_satisfied {
                    if node.indent <= *satisfied_indent {
                        *satisfied = false;
                    }
                }
            }

            unsafe {
                match node.kind {
                    NodeKind::Leaf => {
                        let leaf = leaves.get_unchecked((node.id - leaf_base_id) as usize);
                        let filter_satisfied = (!KV_ENABLED
                            || kv_filter_satisfied.iter().all(|(satisfied, _)| *satisfied))
                            && (!MSG_ENABLED || leaf.msg.contains(msg_filter));
                        if filter_satisfied {
                            if INCREASE_REF_COUNT {
                                increase_ref_count!(
                                    leaf.filtered_parent,
                                    groups,
                                    filtered_ref_count,
                                    filtered_parent
                                );
                            }
                        } else if !INCREASE_REF_COUNT {
                            decrease_ref_count!(
                                leaf.filtered_parent,
                                groups,
                                filtered_ref_count,
                                filtered_parent
                            );
                        }
                        filter_satisfied
                    }

                    NodeKind::Group => {
                        if KV_ENABLED {
                            let group = groups.get(&node.id).unwrap_unchecked();
                            for (i, (satisfied, satisfied_indent)) in
                                kv_filter_satisfied.iter_mut().enumerate()
                            {
                                if !*satisfied && group.kv_str.contains(kv_filter.get_unchecked(i))
                                {
                                    *satisfied = true;
                                    *satisfied_indent = node.indent;
                                }
                            }
                        }
                        true
                    }
                }
            }
        }
    }

    pub fn apply_msg_filter_restriction(&mut self, new: &str) {
        self.filtered
            .retain(Self::filter_node::<true, false, false>(
                &self.leaves,
                self.cur_leaf_base_id,
                &mut self.groups,
                new,
                &[],
            ));
    }

    pub fn apply_kv_filter_restriction(&mut self, new: &[String]) {
        self.filtered
            .retain(Self::filter_node::<false, true, false>(
                &self.leaves,
                self.cur_leaf_base_id,
                &mut self.groups,
                "",
                new,
            ));
    }

    pub fn filter_all<const MSG_ENABLED: bool, const KV_ENABLED: bool>(
        &mut self,
        msg_filter: &str,
        kv_filter: &[String],
    ) {
        if !(MSG_ENABLED || KV_ENABLED) {
            return;
        }

        self.clear_filtered();

        let mut filter_node = Self::filter_node::<MSG_ENABLED, KV_ENABLED, true>(
            &self.leaves,
            self.cur_leaf_base_id,
            &mut self.groups,
            msg_filter,
            kv_filter,
        );
        self.filtered
            .extend(self.all.iter().filter(move |node| filter_node(*node)));
    }

    fn remove_unreferenced_filtered_groups(&mut self) {
        self.filtered.retain(|node| match node.kind {
            NodeKind::Leaf => true,
            NodeKind::Group => unsafe {
                self.groups
                    .get(&node.id)
                    .unwrap_unchecked()
                    .filtered_ref_count
                    != 0
            },
        });
    }

    fn collapse_filtered_groups(&mut self) {
        unsafe {
            let mut min_indent = 0;
            loop {
                let mut found_groups = false;
                let mut cur_indent = u16::MAX;
                let mut cur_kv_str = None;
                let mut cur_id = 0;

                let mut i = 0;
                let mut copy_src_i = 0;
                let mut copy_dst_i = 0;

                while i < self.filtered.len() {
                    let node = &self.filtered[i];
                    if node.indent < min_indent {
                        cur_kv_str = None;
                    } else if node.kind == NodeKind::Group && node.indent <= cur_indent {
                        let group = self.groups.get_mut(&node.id).unwrap_unchecked();
                        found_groups = true;

                        if node.indent < cur_indent || cur_kv_str.as_ref() != Some(&group.kv_str) {
                            cur_kv_str = Some(group.kv_str.clone());
                            cur_indent = node.indent;
                            cur_id = node.id;
                        } else {
                            group.filtered_ref_count = 0;

                            {
                                let copy_len = i - copy_src_i;
                                if copy_src_i != copy_dst_i {
                                    self.filtered.as_mut_ptr().add(copy_dst_i).copy_from(
                                        self.filtered.as_ptr().add(copy_src_i),
                                        copy_len,
                                    );
                                }
                                copy_src_i = i + 1;
                                copy_dst_i += copy_len;
                            }

                            let children_indent = cur_indent + 1;
                            for node in &mut self.filtered[i..] {
                                if node.indent < children_indent {
                                    break;
                                }
                                if node.indent == children_indent {
                                    match node.kind {
                                        NodeKind::Group => {
                                            self.groups
                                                .get_mut(&node.id)
                                                .unwrap_unchecked()
                                                .filtered_parent = cur_id;
                                        }

                                        NodeKind::Leaf => {
                                            self.leaves
                                                .get_unchecked_mut(
                                                    (node.id - self.cur_leaf_base_id) as usize,
                                                )
                                                .filtered_parent = cur_id;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    i += 1;
                }

                if copy_src_i != copy_dst_i {
                    let copy_len = self.filtered.len() - copy_src_i;
                    self.filtered
                        .as_mut_ptr()
                        .add(copy_dst_i)
                        .copy_from(self.filtered.as_ptr().add(copy_src_i), copy_len);
                    self.filtered.set_len(copy_dst_i + copy_len);
                }

                if !found_groups {
                    break;
                }

                min_indent = cur_indent + 1;
            }
        }
    }

    pub fn clean_filtered_groups(&mut self) {
        self.remove_unreferenced_filtered_groups();
        self.collapse_filtered_groups();
    }

    pub fn filter_new_message<const MSG_ENABLED: bool, const KV_ENABLED: bool>(
        &mut self,
        mut indent: u16,
        id: NodeId,
        leaf: &Leaf,
        msg_filter: &str,
        kv_filter: &[String],
    ) {
        let filter_satisfied = (!KV_ENABLED
            || unsafe {
                let mut kv_filter_satisfied = vec![false; kv_filter.len()];

                let mut parent_id = leaf.parent;
                while parent_id != NodeId::MAX {
                    let parent = self.groups.get_mut(&parent_id).unwrap_unchecked();
                    for (i, filter) in kv_filter.iter().enumerate() {
                        if parent.kv_str.contains(filter) {
                            *kv_filter_satisfied.get_unchecked_mut(i) = true;
                        }
                    }
                    parent_id = parent.parent;
                }

                kv_filter_satisfied.iter().all(|v| *v)
            })
            && (!MSG_ENABLED || leaf.msg.contains(msg_filter));
        if !filter_satisfied {
            return;
        }

        let pos = self.filtered.len();
        self.filtered.push(Node {
            indent,
            kind: NodeKind::Leaf,
            id,
        });

        unsafe {
            let mut parent_id = leaf.filtered_parent;
            while parent_id != NodeId::MAX {
                let parent = self.groups.get_mut(&parent_id).unwrap_unchecked();
                parent.filtered_ref_count += 1;
                if parent.filtered_ref_count != 1 {
                    break;
                }
                indent -= 1;
                self.filtered.insert(
                    pos,
                    Node {
                        indent,
                        kind: NodeKind::Group,
                        id: parent_id,
                    },
                );
                parent_id = parent.filtered_parent;
            }
        }

        self.collapse_filtered_groups();
    }
}

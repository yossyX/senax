use compact_str::CompactString;
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Ref,
    Include,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EdgeTarget {
    pub group: Option<String>,
    pub node: String,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub edges: Vec<EdgeTarget>,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub name: String,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct MarkResult {
    pub node_marks: BTreeMap<(CompactString, CompactString), Mark>,
}

#[derive(Debug, Clone)]
pub struct UnifiedGroup {
    pub nodes: BTreeMap<(CompactString, CompactString), Mark>,
    pub start_node: (CompactString, CompactString),
    pub ref_unified_groups: BTreeSet<(CompactString, CompactString)>,
}

#[derive(Debug)]
pub struct GraphAnalyzer {
    groups: BTreeMap<String, Group>,
}

impl GraphAnalyzer {
    pub fn new() -> Self {
        Self {
            groups: BTreeMap::new(),
        }
    }

    pub fn add_group(&mut self, group: Group) {
        self.groups.insert(group.name.clone(), group);
    }

    fn mark_from_node(&self, start_group: &str, start_node: &str) -> Option<MarkResult> {
        if !self.groups.contains_key(start_group) {
            return None;
        }

        let start_key = (start_group.into(), start_node.into());
        let mut node_marks = BTreeMap::new();
        let mut in_stack = HashSet::new();
        let mut visited = HashSet::new();
        let mut rec_stack = Vec::new();
        let mut node_paths: BTreeMap<
            (CompactString, CompactString),
            Vec<(CompactString, CompactString)>,
        > = BTreeMap::new();
        let mut node_backrefs: BTreeMap<
            (CompactString, CompactString),
            Vec<Vec<(CompactString, CompactString)>>,
        > = BTreeMap::new();

        node_marks.insert(start_key.clone(), Mark::Include);

        let _ = self.dfs_mark(
            start_group,
            start_node,
            &start_key,
            &mut visited,
            &mut in_stack,
            &mut rec_stack,
            &mut node_marks,
            &mut node_paths,
            &mut node_backrefs,
        );

        Some(MarkResult { node_marks })
    }

    fn dfs_mark(
        &self,
        current_group: &str,
        current_node: &str,
        start_key: &(CompactString, CompactString),
        visited: &mut HashSet<(CompactString, CompactString)>,
        in_stack: &mut HashSet<(CompactString, CompactString)>,
        rec_stack: &mut Vec<(CompactString, CompactString)>,
        node_marks: &mut BTreeMap<(CompactString, CompactString), Mark>,
        node_paths: &mut BTreeMap<
            (CompactString, CompactString),
            Vec<(CompactString, CompactString)>,
        >,
        node_backrefs: &mut BTreeMap<
            (CompactString, CompactString),
            Vec<Vec<(CompactString, CompactString)>>,
        >,
    ) -> bool {
        let key = (current_group.into(), current_node.into());

        if in_stack.contains(&key) {
            if let Some(mark) = node_marks.get(&key) {
                if *mark == Mark::Include {
                    let mut found = false;
                    for stack_key in rec_stack.iter() {
                        if stack_key == &key {
                            found = true;
                        } else if found {
                            node_marks.insert(stack_key.clone(), Mark::Include);
                            self.promote_to_include(
                                stack_key,
                                node_marks,
                                node_paths,
                                node_backrefs,
                            );
                        }
                    }
                    return true;
                } else {
                    node_backrefs
                        .entry(key.clone())
                        .or_insert_with(Vec::new)
                        .push(rec_stack.clone());
                }
            }
            return false;
        }

        if visited.contains(&key) {
            return node_marks
                .get(&key)
                .map(|m| *m == Mark::Include)
                .unwrap_or(false);
        }

        visited.insert(key.clone());
        in_stack.insert(key.clone());
        rec_stack.push(key.clone());

        if &key != start_key {
            node_marks.insert(key.clone(), Mark::Ref);
            node_paths.insert(key.clone(), rec_stack.clone());
        }

        let mut is_in_start_cycle = false;

        if let Some(group) = self.groups.get(current_group) {
            if let Some(node) = group.nodes.iter().find(|n| n.name == current_node) {
                for edge in &node.edges {
                    let target_group = edge.group.as_deref().unwrap_or(current_group);
                    let target_node = &edge.node;

                    let has_cycle_to_start = self.dfs_mark(
                        target_group,
                        target_node,
                        start_key,
                        visited,
                        in_stack,
                        rec_stack,
                        node_marks,
                        node_paths,
                        node_backrefs,
                    );

                    if has_cycle_to_start {
                        is_in_start_cycle = true;
                    }
                }
            }
        }

        if is_in_start_cycle && in_stack.contains(&key) && &key != start_key {
            node_marks.insert(key.clone(), Mark::Include);
            self.promote_to_include(&key, node_marks, node_paths, node_backrefs);
        }

        rec_stack.pop();
        in_stack.remove(&key);

        is_in_start_cycle
    }

    fn promote_to_include(
        &self,
        node_key: &(CompactString, CompactString),
        node_marks: &mut BTreeMap<(CompactString, CompactString), Mark>,
        node_paths: &mut BTreeMap<
            (CompactString, CompactString),
            Vec<(CompactString, CompactString)>,
        >,
        node_backrefs: &mut BTreeMap<
            (CompactString, CompactString),
            Vec<Vec<(CompactString, CompactString)>>,
        >,
    ) {
        if let Some(path) = node_paths.get(node_key).cloned() {
            for path_node in path.iter() {
                if node_marks.get(path_node) == Some(&Mark::Ref) {
                    node_marks.insert(path_node.clone(), Mark::Include);
                    self.promote_to_include(path_node, node_marks, node_paths, node_backrefs);
                }
            }
        }

        if let Some(backrefs) = node_backrefs.get(node_key).cloned() {
            for rec_stack in backrefs {
                let mut found = false;
                for stack_key in rec_stack.iter() {
                    if stack_key == node_key {
                        found = true;
                    } else if found {
                        if node_marks.get(stack_key) == Some(&Mark::Ref) {
                            node_marks.insert(stack_key.clone(), Mark::Include);
                            self.promote_to_include(
                                stack_key,
                                node_marks,
                                node_paths,
                                node_backrefs,
                            );
                        }
                    }
                }
            }
            node_backrefs.remove(node_key);
        }
    }

    pub fn unify_all_nodes(&self) -> Vec<UnifiedGroup> {
        let mut unified_groups = Vec::new();
        let mut unified_nodes = HashSet::new();

        let mut all_nodes: Vec<(CompactString, CompactString)> = Vec::new();
        for (group_name, group) in &self.groups {
            for node in &group.nodes {
                all_nodes.push((group_name.into(), (&node.name).into()));
            }
        }

        while unified_nodes.len() < all_nodes.len() {
            let start_node = all_nodes
                .iter()
                .find(|node| !unified_nodes.contains(*node))
                .cloned();

            if let Some((group_name, node_name)) = start_node {
                if let Some(mark_result) = self.mark_from_node(&group_name, &node_name) {
                    let mut nodes = BTreeMap::new();

                    for ((g, n), mark) in &mark_result.node_marks {
                        if *mark == Mark::Include {
                            unified_nodes.insert((g.clone(), n.clone()));
                            nodes.insert((g.clone(), n.clone()), *mark);
                        }
                    }

                    for ((g, n), mark) in &mark_result.node_marks {
                        if *mark == Mark::Include {
                            if let Some(group) = self.groups.get(g.as_str()) {
                                if let Some(node) = group.nodes.iter().find(|node| &node.name == n)
                                {
                                    for edge in &node.edges {
                                        let target_group =
                                            edge.group.as_deref().unwrap_or(g.as_str());
                                        let target_key = (target_group.into(), (&edge.node).into());

                                        if let Some(target_mark) =
                                            mark_result.node_marks.get(&target_key)
                                        {
                                            if *target_mark == Mark::Ref {
                                                nodes.insert(target_key, *target_mark);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if nodes.is_empty() {
                        unified_nodes.insert((group_name.clone(), node_name.clone()));
                        nodes.insert((group_name.clone(), node_name.clone()), Mark::Include);
                    }

                    unified_groups.push(UnifiedGroup {
                        nodes,
                        start_node: (group_name, node_name),
                        ref_unified_groups: BTreeSet::new(),
                    });
                } else {
                    let mut nodes = BTreeMap::new();
                    nodes.insert((group_name.clone(), node_name.clone()), Mark::Include);
                    unified_nodes.insert((group_name.clone(), node_name.clone()));

                    unified_groups.push(UnifiedGroup {
                        nodes,
                        start_node: (group_name.clone(), node_name.clone()),
                        ref_unified_groups: BTreeSet::new(),
                    });
                }
            } else {
                break;
            }
        }

        unified_groups
    }

    pub fn merge_unified_groups(
        &self,
        unified_groups: Vec<UnifiedGroup>,
        max_include_count: usize,
    ) -> Vec<UnifiedGroup> {
        let mut remaining: Vec<UnifiedGroup> = unified_groups;
        let mut check_overlap = true;

        self.calculate_ref_unified_groups(&mut remaining);

        loop {
            let (best_pair, has_overlap) =
                self.find_best_merge_pair(&remaining, max_include_count, check_overlap);

            if check_overlap && !has_overlap {
                check_overlap = false;
            }

            if let Some((idx1, idx2)) = best_pair {
                let (remove_first, remove_second) = if idx1 > idx2 {
                    (idx1, idx2)
                } else {
                    (idx2, idx1)
                };

                let mut group1 = remaining.remove(remove_first);
                let group2 = remaining.remove(remove_second);

                let keep_start = if group2.nodes.contains_key(&group2.start_node) {
                    group2.start_node.clone()
                } else {
                    group1.start_node.clone()
                };

                for ((g, n), mark) in &group2.nodes {
                    let existing_mark = group1.nodes.get(&(g.clone(), n.clone()));
                    let new_mark = self.merge_marks(*mark, existing_mark.copied());
                    group1.nodes.insert((g.clone(), n.clone()), new_mark);
                }

                group1.start_node = keep_start.clone();

                remaining.push(group1);

                self.calculate_ref_unified_groups(&mut remaining);
            } else {
                break;
            }
        }

        remaining
    }

    fn calculate_ref_unified_groups(&self, unified_groups: &mut [UnifiedGroup]) {
        let mut node_to_include_groups: BTreeMap<(CompactString, CompactString), usize> =
            BTreeMap::new();
        for (idx, group) in unified_groups.iter().enumerate() {
            for (node_key, mark) in &group.nodes {
                if *mark == Mark::Include {
                    node_to_include_groups
                        .entry(node_key.clone())
                        .or_insert(idx);
                }
            }
        }

        for group_idx in 0..unified_groups.len() {
            let mut ref_groups_set: BTreeSet<(CompactString, CompactString)> = BTreeSet::new();

            let ref_nodes: Vec<_> = unified_groups[group_idx]
                .nodes
                .iter()
                .filter(|(_, mark)| **mark == Mark::Ref)
                .map(|(key, _)| key.clone())
                .collect();

            for ref_node_key in ref_nodes {
                if let Some(target_group_idx) = node_to_include_groups.get(&ref_node_key) {
                    assert_ne!(*target_group_idx, group_idx);
                    let target_start_node = &unified_groups[*target_group_idx].start_node;
                    ref_groups_set.insert(target_start_node.clone());
                } else {
                    panic!("error");
                }
            }

            unified_groups[group_idx].ref_unified_groups = ref_groups_set;
        }
    }

    fn find_ref_loop_pair(&self, groups: &[UnifiedGroup]) -> Option<(usize, usize)> {
        let mut start_node_to_idx = BTreeMap::new();
        for (idx, group) in groups.iter().enumerate() {
            start_node_to_idx.insert(group.start_node.clone(), idx);
        }

        for start_idx in 0..groups.len() {
            let mut visited = HashSet::new();
            let mut rec_stack = Vec::new();

            if let Some(loop_indices) = self.find_ref_loop_with_path(
                start_idx,
                groups,
                &start_node_to_idx,
                &mut visited,
                &mut rec_stack,
            ) {
                if loop_indices.len() >= 2 {
                    return Some((loop_indices[0], loop_indices[1]));
                }
            }
        }

        None
    }

    fn find_ref_loop_with_path(
        &self,
        group_idx: usize,
        unified_groups: &[UnifiedGroup],
        start_node_to_idx: &BTreeMap<(CompactString, CompactString), usize>,
        visited: &mut HashSet<usize>,
        rec_stack: &mut Vec<usize>,
    ) -> Option<Vec<usize>> {
        if let Some(pos) = rec_stack.iter().position(|&idx| idx == group_idx) {
            return Some(rec_stack[pos..].to_vec());
        }

        if visited.contains(&group_idx) {
            return None;
        }

        visited.insert(group_idx);
        rec_stack.push(group_idx);

        let group = &unified_groups[group_idx];
        for ref_start_node in &group.ref_unified_groups {
            if let Some(&target_idx) = start_node_to_idx.get(ref_start_node) {
                if let Some(loop_indices) = self.find_ref_loop_with_path(
                    target_idx,
                    unified_groups,
                    start_node_to_idx,
                    visited,
                    rec_stack,
                ) {
                    return Some(loop_indices);
                }
            }
        }

        rec_stack.pop();
        None
    }

    fn find_best_merge_pair(
        &self,
        groups: &[UnifiedGroup],
        max_include_count: usize,
        check_overlap: bool,
    ) -> (Option<(usize, usize)>, bool) {
        let mut best_pair = None;

        if let Some((i, j)) = self.find_ref_loop_pair(groups) {
            return (Some((i, j)), true);
        }

        if check_overlap {
            for i in 0..groups.len() {
                for j in (i + 1)..groups.len() {
                    let overlap_score =
                        self.count_overlapping_include_nodes(&groups[i].nodes, &groups[j].nodes);

                    if overlap_score > 0 {
                        return (Some((i, j)), true);
                    }
                }
            }
        }
        let mut candidate_groups: Vec<(usize, usize, usize)> = groups
            .iter()
            .enumerate()
            .map(|(idx, g)| {
                let size = g.nodes.values().filter(|m| **m == Mark::Include).count();
                let ref_incoming_counts = groups
                    .iter()
                    .filter(|group_j| group_j.ref_unified_groups.contains(&g.start_node))
                    .count();
                (idx, size, ref_incoming_counts)
            })
            .filter(|(_, size, incoming)| {
                *size
                    < if *incoming > 0 {
                        (max_include_count + 1) / 2
                    } else {
                        max_include_count
                    }
            })
            .collect();

        candidate_groups.sort_by(|a, b| b.2.cmp(&a.2).then(b.1.cmp(&a.1)));

        for (i_pos, &(i, _, ref_i)) in candidate_groups.iter().enumerate() {
            let mut best_ref_reduction = 0;
            for &(j, _, ref_j) in candidate_groups.iter().skip(i_pos + 1) {
                let merged_include_count =
                    self.calculate_merged_include_count(&groups[i].nodes, &groups[j].nodes);

                if merged_include_count
                    <= if ref_i > 0 || ref_j > 0 {
                        (max_include_count + 1) / 2
                    } else {
                        max_include_count
                    }
                {
                    if self.would_create_ref_loop(groups, i, j) {
                        continue;
                    }

                    let ref_count_i = groups[i]
                        .nodes
                        .values()
                        .filter(|m| **m == Mark::Ref)
                        .count();
                    let ref_count_j = groups[j]
                        .nodes
                        .values()
                        .filter(|m| **m == Mark::Ref)
                        .count();
                    let total_ref_before = ref_count_i + ref_count_j;

                    let mut merged_nodes = groups[i].nodes.clone();
                    for (node_key, mark_j) in &groups[j].nodes {
                        let mark_i = merged_nodes.get(node_key);
                        merged_nodes
                            .insert(node_key.clone(), self.merge_marks(*mark_j, mark_i.copied()));
                    }
                    let ref_count_after =
                        merged_nodes.values().filter(|m| **m == Mark::Ref).count();

                    let ref_reduction = total_ref_before.saturating_sub(ref_count_after);

                    if best_pair.is_none() || ref_reduction > best_ref_reduction {
                        best_ref_reduction = ref_reduction;
                        best_pair = Some((i, j));
                    }
                }
            }
            if best_pair.is_some() {
                break;
            }
        }
        (best_pair, false)
    }

    fn count_overlapping_include_nodes(
        &self,
        nodes1: &BTreeMap<(CompactString, CompactString), Mark>,
        nodes2: &BTreeMap<(CompactString, CompactString), Mark>,
    ) -> usize {
        let mut overlap_count = 0;

        for (key, mark1) in nodes1 {
            if *mark1 == Mark::Include {
                if let Some(mark2) = nodes2.get(key) {
                    if *mark2 == Mark::Include {
                        overlap_count += 1;
                    }
                }
            }
        }

        overlap_count
    }

    fn would_create_ref_loop(
        &self,
        groups: &[UnifiedGroup],
        merge_idx1: usize,
        merge_idx2: usize,
    ) -> bool {
        let has_ref1 = groups[merge_idx1]
            .nodes
            .values()
            .any(|mark| *mark == Mark::Ref);
        let has_ref2 = groups[merge_idx2]
            .nodes
            .values()
            .any(|mark| *mark == Mark::Ref);
        let has_ref_groups1 = !groups[merge_idx1].ref_unified_groups.is_empty();
        let has_ref_groups2 = !groups[merge_idx2].ref_unified_groups.is_empty();

        if !has_ref1 && !has_ref2 && !has_ref_groups1 && !has_ref_groups2 {
            return false;
        }

        let mut virtual_groups = Vec::new();

        let mut merged_refs: HashSet<(CompactString, CompactString)> = HashSet::new();
        merged_refs.extend(groups[merge_idx1].ref_unified_groups.iter().cloned());
        merged_refs.extend(groups[merge_idx2].ref_unified_groups.iter().cloned());

        merged_refs.remove(&groups[merge_idx1].start_node);
        merged_refs.remove(&groups[merge_idx2].start_node);

        let merged_start_node = groups[merge_idx1].start_node.clone();

        for (idx, group) in groups.iter().enumerate() {
            if idx == merge_idx1 {
                let mut merged_group = group.clone();
                merged_group.ref_unified_groups = merged_refs.iter().cloned().collect();
                merged_group.start_node = merged_start_node.clone();
                virtual_groups.push(merged_group);
            } else if idx == merge_idx2 {
                continue;
            } else {
                let mut updated_group = group.clone();
                updated_group.ref_unified_groups = updated_group
                    .ref_unified_groups
                    .iter()
                    .map(|ref_start| {
                        if ref_start == &groups[merge_idx2].start_node {
                            merged_start_node.clone()
                        } else {
                            ref_start.clone()
                        }
                    })
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect();
                virtual_groups.push(updated_group);
            }
        }

        let validation_errors = self.validate_no_ref_loops(&virtual_groups);
        !validation_errors.is_empty()
    }

    fn calculate_merged_include_count(
        &self,
        nodes1: &BTreeMap<(CompactString, CompactString), Mark>,
        nodes2: &BTreeMap<(CompactString, CompactString), Mark>,
    ) -> usize {
        let mut merged = nodes2.clone();
        for (key, mark1) in nodes1 {
            let mark2 = merged.get(key).copied();
            let new_mark = self.merge_marks(*mark1, mark2);
            merged.insert(key.clone(), new_mark);
        }
        merged
            .values()
            .filter(|mark| **mark == Mark::Include)
            .count()
    }

    fn merge_marks(&self, mark1: Mark, mark2: Option<Mark>) -> Mark {
        match (mark1, mark2) {
            (m1, Some(m2)) => match (m1, m2) {
                (Mark::Include, _) | (_, Mark::Include) => Mark::Include,
                _ => Mark::Ref,
            },
            (m, None) => m,
        }
    }

    fn validate_no_ref_loops(&self, unified_groups: &[UnifiedGroup]) -> Vec<String> {
        let mut errors = Vec::new();

        let mut start_node_to_idx = BTreeMap::new();
        for (idx, group) in unified_groups.iter().enumerate() {
            start_node_to_idx.insert(group.start_node.clone(), idx);
        }

        for (group_idx, _) in unified_groups.iter().enumerate() {
            let mut visited = HashSet::new();
            let mut rec_stack = HashSet::new();

            if self.detect_ref_loop(
                group_idx,
                unified_groups,
                &start_node_to_idx,
                &mut visited,
                &mut rec_stack,
            ) {
                errors.push(format!(
                    "Group #{} ({}/{}): Loop reference detected in ref_unified_groups",
                    group_idx,
                    unified_groups[group_idx].start_node.0,
                    unified_groups[group_idx].start_node.1
                ));
            }
        }

        errors
    }

    fn detect_ref_loop(
        &self,
        group_idx: usize,
        unified_groups: &[UnifiedGroup],
        start_node_to_idx: &BTreeMap<(CompactString, CompactString), usize>,
        visited: &mut HashSet<usize>,
        rec_stack: &mut HashSet<usize>,
    ) -> bool {
        if rec_stack.contains(&group_idx) {
            return true;
        }

        if visited.contains(&group_idx) {
            return false;
        }

        visited.insert(group_idx);
        rec_stack.insert(group_idx);

        let group = &unified_groups[group_idx];
        for ref_start_node in &group.ref_unified_groups {
            if let Some(&target_idx) = start_node_to_idx.get(ref_start_node) {
                if self.detect_ref_loop(
                    target_idx,
                    unified_groups,
                    start_node_to_idx,
                    visited,
                    rec_stack,
                ) {
                    return true;
                }
            }
        }

        rec_stack.remove(&group_idx);
        false
    }
}

impl Default for GraphAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl UnifiedGroup {
    pub fn include_groups(&self) -> BTreeSet<CompactString> {
        let mut include_groups = BTreeSet::new();

        for ((group_name, _), mark) in &self.nodes {
            if *mark == Mark::Include {
                include_groups.insert(group_name.clone());
            }
        }

        include_groups
    }
    pub fn unified_name(&self) -> String {
        use crate::common::ToCase;
        format!(
            "{}__{}",
            self.start_node.0.to_snake(),
            self.start_node.1.to_snake()
        )
    }
    pub fn is_ref(&self, rel: &[String]) -> bool {
        self.nodes.get(&((&rel[0]).into(), (&rel[1]).into())) == Some(&Mark::Ref)
    }
    pub fn unified_name_from_rel(unified_groups: &[UnifiedGroup], rel: &[String]) -> String {
        let unified = unified_groups
            .iter()
            .find(|v| v.nodes.contains_key(&((&rel[0]).into(), (&rel[1]).into())));
        unified.unwrap().unified_name()
    }
}

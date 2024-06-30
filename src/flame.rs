pub type StackIdentifier = usize;
pub static ROOT: &str = "all";
pub static ROOT_ID: usize = 0;

#[derive(Debug, Clone, PartialEq)]
pub struct StackInfo {
    pub id: StackIdentifier,
    pub line_index: usize,
    pub start_index: usize,
    pub end_index: usize,
    pub total_count: u64,
    pub self_count: u64,
    pub parent: Option<StackIdentifier>,
    pub children: Vec<StackIdentifier>,
    pub level: usize,
    pub width_factor: f64,
    pub hit: bool,
}

#[derive(Debug, Clone)]
pub struct SearchPattern {
    pub pattern: String,
    pub is_regex: bool,
    pub re: regex::Regex,
    pub is_manual: bool,
}

impl SearchPattern {
    pub fn new(pattern: &str, is_regex: bool, is_manual: bool) -> Result<Self, regex::Error> {
        let _pattern = if is_regex {
            pattern.to_string()
        } else {
            format!("^{}$", regex::escape(pattern))
        };
        let re = regex::Regex::new(&_pattern)?;
        Ok(Self {
            pattern: pattern.to_string(),
            is_regex,
            re,
            is_manual,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Hits {
    coverage_count: u64,
    ids: Vec<StackIdentifier>,
}

#[derive(Debug, Clone)]
pub struct FlameGraph {
    data: String,
    stacks: Vec<StackInfo>,
    levels: Vec<Vec<StackIdentifier>>,
    hits: Option<Hits>,
    sorted: bool,
}

impl FlameGraph {
    pub fn from_string(mut content: String, sorted: bool) -> Self {
        // Make sure content ends with newline to simplify parsing
        if !content.ends_with('\n') {
            content.push('\n');
        }
        let mut stacks = Vec::<StackInfo>::new();
        stacks.push(StackInfo {
            id: ROOT_ID,
            line_index: 0,
            start_index: 0,
            end_index: 0,
            total_count: 0,
            self_count: 0,
            width_factor: 0.0,
            parent: None,
            children: Vec::<StackIdentifier>::new(),
            level: 0,
            hit: false,
        });
        let mut last_line_index = 0;
        for line_index in content
            .char_indices()
            .filter(|(_, c)| *c == '\n')
            .map(|(i, _)| i)
        {
            let line = &content[last_line_index..line_index];
            #[allow(clippy::unnecessary_unwrap)]
            let line_and_count = match line.rsplit_once(' ') {
                Some((line, count)) => {
                    let parsed_count = count.parse::<u64>();
                    if line.is_empty() || parsed_count.is_err() {
                        None
                    } else {
                        Some((line, parsed_count.unwrap()))
                    }
                }
                _ => None,
            };
            if line_and_count.is_none() || line.starts_with('#') {
                last_line_index = line_index + 1;
                continue;
            }
            let (line, count) = line_and_count.unwrap();

            stacks[ROOT_ID].total_count += count;
            let mut parent_id = ROOT_ID;
            let mut level = 1;
            let mut last_delim_index = 0;
            for delim_index in line
                .char_indices()
                .filter(|(_, c)| *c == ';')
                .map(|(i, _)| i)
            {
                let stack_id = FlameGraph::update_one(
                    &mut stacks,
                    &content,
                    count,
                    last_line_index,
                    last_line_index + last_delim_index,
                    last_line_index + delim_index,
                    parent_id,
                    level,
                    false,
                );
                parent_id = stack_id;
                level += 1;
                last_delim_index = delim_index + 1;
            }
            FlameGraph::update_one(
                &mut stacks,
                &content,
                count,
                last_line_index,
                last_line_index + last_delim_index,
                last_line_index + line.len(),
                parent_id,
                level,
                true,
            );
            last_line_index = line_index + 1;
        }

        let mut out = Self {
            data: content,
            stacks,
            levels: vec![],
            hits: None,
            sorted,
        };
        out.populate_levels(&ROOT_ID, 0, None);
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn update_one(
        stacks: &mut Vec<StackInfo>,
        content: &str,
        count: u64,
        line_index: usize,
        start_index: usize,
        end_index: usize,
        parent_id: StackIdentifier,
        level: usize,
        is_self: bool,
    ) -> StackIdentifier {
        let short_name = &content[start_index..end_index];
        // Invariant: parent always exists. We can just check the short name to
        // check if the parent already contains the child, since the prior
        // prefixes should always match (definition of a parent).
        let parent_stack = stacks.get(parent_id).unwrap();
        let current_stack_id_if_exists = parent_stack
            .children
            .iter()
            .find(|child_id| {
                let child = stacks.get(**child_id).unwrap();
                &content[child.start_index..child.end_index] == short_name
            })
            .cloned();
        let stack_id = if let Some(stack_id) = current_stack_id_if_exists {
            stack_id
        } else {
            stacks.push(StackInfo {
                id: stacks.len(),
                line_index,
                start_index,
                end_index,
                total_count: 0,
                self_count: 0,
                width_factor: 0.0,
                parent: Some(parent_id),
                children: Vec::<StackIdentifier>::new(),
                level,
                hit: false,
            });
            let stack_id = stacks.len() - 1;
            stacks.get_mut(parent_id).unwrap().children.push(stack_id);
            stack_id
        };
        let info = stacks.get_mut(stack_id).unwrap();
        info.total_count += count;
        if is_self {
            info.self_count += count;
        }
        stack_id
    }

    fn populate_levels(
        &mut self,
        stack_id: &StackIdentifier,
        level: usize,
        parent_total_count_and_width_factor: Option<(u64, f64)>,
    ) {
        // Update levels
        if self.levels.len() <= level {
            self.levels.push(vec![]);
        }
        self.levels[level].push(*stack_id);

        // Calculate width_factor of the current stack
        let stack = self.stacks.get(*stack_id).unwrap();
        let total_count = stack.total_count;
        let width_factor = if let Some((parent_total_count, parent_width_factor)) =
            parent_total_count_and_width_factor
        {
            parent_width_factor * (total_count as f64 / parent_total_count as f64)
        } else {
            1.0
        };

        // Sort children
        let sorted_children = if self.sorted {
            let mut sorted_children = stack.children.clone();
            sorted_children.sort_by_key(|child_id| {
                self.stacks
                    .get(*child_id)
                    .map(|child| child.total_count)
                    .unwrap_or(0)
            });
            sorted_children.reverse();
            Some(sorted_children)
        } else {
            None
        };

        // Make the updates to the current stack
        let stack = self.stacks.get_mut(*stack_id).unwrap();
        stack.width_factor = width_factor;
        if let Some(sorted_children) = sorted_children {
            stack.children = sorted_children;
        }

        // Move on to children
        for child_id in stack.children.clone().iter() {
            self.populate_levels(child_id, level + 1, Some((total_count, width_factor)));
        }
    }

    pub fn get_stack(&self, stack_id: &StackIdentifier) -> Option<&StackInfo> {
        self.stacks.get(*stack_id)
    }

    pub fn get_stack_short_name(&self, stack_id: &StackIdentifier) -> Option<&str> {
        self.get_stack(stack_id)
            .map(|stack| self.get_stack_short_name_from_info(stack))
    }

    pub fn get_stack_full_name(&self, stack_id: &StackIdentifier) -> Option<&str> {
        self.get_stack(stack_id)
            .map(|stack| self.get_stack_full_name_from_info(stack))
    }

    pub fn get_stack_short_name_from_info(&self, stack: &StackInfo) -> &str {
        if stack.id == ROOT_ID {
            ROOT
        } else {
            &self.data[stack.start_index..stack.end_index]
        }
    }

    pub fn get_stack_full_name_from_info(&self, stack: &StackInfo) -> &str {
        if stack.id == ROOT_ID {
            ROOT
        } else {
            &self.data[stack.line_index..stack.end_index]
        }
    }

    pub fn get_stack_by_full_name(&self, full_name: &str) -> Option<&StackInfo> {
        self.stacks
            .iter()
            .find(|stack| self.get_stack_full_name_from_info(stack) == full_name)
    }

    pub fn get_stack_id_by_full_name(&self, full_name: &str) -> Option<StackIdentifier> {
        self.get_stack_by_full_name(full_name).map(|stack| stack.id)
    }

    pub fn get_stacks_at_level(&self, level: usize) -> Option<&Vec<StackIdentifier>> {
        self.levels.get(level)
    }

    pub fn root(&self) -> &StackInfo {
        self.get_stack(&ROOT_ID).unwrap()
    }

    pub fn total_count(&self) -> u64 {
        self.root().total_count
    }

    pub fn get_num_levels(&self) -> usize {
        self.levels.len()
    }

    pub fn get_ancestors(&self, stack_id: &StackIdentifier) -> Vec<StackIdentifier> {
        let mut ancestors = vec![];
        let mut current_id = *stack_id;
        while let Some(stack) = self.get_stack(&current_id) {
            ancestors.push(current_id);
            if let Some(parent_id) = stack.parent {
                current_id = parent_id;
            } else {
                break;
            }
        }
        ancestors
    }

    pub fn get_descendants(&self, stack_id: &StackIdentifier) -> Vec<StackIdentifier> {
        let mut descendants = vec![];
        let mut stack_ids = vec![*stack_id];
        while let Some(stack_id) = stack_ids.pop() {
            descendants.push(stack_id);
            if let Some(stack) = self.get_stack(&stack_id) {
                stack_ids.extend(stack.children.iter().copied());
            }
        }
        descendants
    }

    pub fn set_hits(&mut self, p: &SearchPattern) {
        self.stacks.iter_mut().for_each(|stack| {
            stack.hit =
                p.re.is_match(&self.data[stack.start_index..stack.end_index]);
        });
        self.hits = Some(Hits {
            coverage_count: self._count_hit_coverage(ROOT_ID),
            ids: self._collect_hit_ids(),
        });
    }

    pub fn clear_hits(&mut self) {
        self.stacks.iter_mut().for_each(|stack| stack.hit = false);
        self.hits = None;
    }

    pub fn hit_coverage_count(&self) -> Option<u64> {
        self.hits.as_ref().map(|h| h.coverage_count)
    }

    pub fn hit_ids(&self) -> Option<&Vec<StackIdentifier>> {
        self.hits.as_ref().map(|h| &h.ids)
    }

    fn _count_hit_coverage(&self, stack_id: StackIdentifier) -> u64 {
        let stack = self.get_stack(&stack_id).unwrap();
        if stack.hit {
            return stack.total_count;
        }
        let mut count = 0;
        for child_id in stack.children.iter() {
            count += self._count_hit_coverage(*child_id);
        }
        count
    }

    fn _collect_hit_ids(&self) -> Vec<StackIdentifier> {
        let mut hits = vec![];
        for level in self.levels.iter() {
            for stacks in level.iter() {
                if let Some(stack) = self.get_stack(stacks) {
                    if stack.hit {
                        hits.push(*stacks);
                    }
                }
            }
        }
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct StackInfoReadable<'a> {
        pub id: StackIdentifier,
        pub short_name: &'a str,
        pub full_name: &'a str,
        pub total_count: u64,
        pub self_count: u64,
        pub parent: Option<StackIdentifier>,
        pub children: Vec<StackIdentifier>,
        pub level: usize,
        pub width_factor: f64,
        pub hit: bool,
    }

    impl<'a> StackInfoReadable<'a> {
        pub fn new(fg: &'a FlameGraph, stack_id: &'a StackIdentifier) -> StackInfoReadable<'a> {
            let stack = fg.get_stack(&stack_id).unwrap();
            StackInfoReadable {
                id: stack.id,
                short_name: fg.get_stack_short_name(stack_id).unwrap(),
                full_name: fg.get_stack_full_name(stack_id).unwrap(),
                total_count: stack.total_count,
                self_count: stack.self_count,
                parent: stack.parent,
                children: stack.children.clone(),
                level: stack.level,
                width_factor: stack.width_factor,
                hit: stack.hit,
            }
        }
    }

    fn get_readable_stacks<'a>(fg: &'a FlameGraph) -> Vec<StackInfoReadable<'a>> {
        fg.stacks
            .iter()
            .map(|s| StackInfoReadable::new(fg, &s.id))
            .collect::<Vec<_>>()
    }

    fn _print_stacks(fg: &FlameGraph) {
        let mut sorted_stacks = get_readable_stacks(fg);
        sorted_stacks.sort_by_key(|x| x.id);
        println!("{:?}", sorted_stacks);
    }

    #[test]
    fn test_simple() {
        let content = std::fs::read_to_string("tests/data/py-spy-simple.txt").unwrap();
        let fg = FlameGraph::from_string(content, true);
        let stacks = get_readable_stacks(&fg);
        // _print_stacks(&fg);
        let items: Vec<StackInfoReadable> = vec![
            StackInfoReadable {
                id: 0,
                short_name: "all",
                full_name: "all",
                total_count: 657,
                self_count: 0,
                parent: None,
                children: vec![3, 1, 5],
                level: 0,
                width_factor: 1.0,
                hit: false,
            },
            StackInfoReadable {
                id: 1,
                short_name: "<module> (long_running.py:24)",
                full_name: "<module> (long_running.py:24)",
                total_count: 17,
                self_count: 0,
                parent: Some(0),
                children: vec![7, 2],
                level: 1,
                width_factor: 0.0258751902587519,
                hit: false,
            },
            StackInfoReadable {
                id: 2,
                short_name: "quick_work (long_running.py:16)",
                full_name: "<module> (long_running.py:24);quick_work (long_running.py:16)",
                total_count: 7,
                self_count: 7,
                parent: Some(1),
                children: vec![],
                level: 2,
                width_factor: 0.0106544901065449,
                hit: false,
            },
            StackInfoReadable {
                id: 3,
                short_name: "<module> (long_running.py:25)",
                full_name: "<module> (long_running.py:25)",
                total_count: 639,
                self_count: 0,
                parent: Some(0),
                children: vec![4, 6],
                level: 1,
                width_factor: 0.9726027397260274,
                hit: false,
            },
            StackInfoReadable {
                id: 4,
                short_name: "work (long_running.py:8)",
                full_name: "<module> (long_running.py:25);work (long_running.py:8)",
                total_count: 421,
                self_count: 421,
                parent: Some(3),
                children: vec![],
                level: 2,
                width_factor: 0.6407914764079147,
                hit: false,
            },
            StackInfoReadable {
                id: 5,
                short_name: "<module> (long_running.py:26)",
                full_name: "<module> (long_running.py:26)",
                total_count: 1,
                self_count: 1,
                parent: Some(0),
                children: vec![],
                level: 1,
                width_factor: 0.0015220700152207,
                hit: false,
            },
            StackInfoReadable {
                id: 6,
                short_name: "work (long_running.py:7)",
                full_name: "<module> (long_running.py:25);work (long_running.py:7)",
                total_count: 218,
                self_count: 218,
                parent: Some(3),
                children: vec![],
                level: 2,
                width_factor: 0.3318112633181126,
                hit: false,
            },
            StackInfoReadable {
                id: 7,
                short_name: "quick_work (long_running.py:17)",
                full_name: "<module> (long_running.py:24);quick_work (long_running.py:17)",
                total_count: 10,
                self_count: 10,
                parent: Some(1),
                children: vec![],
                level: 2,
                width_factor: 0.015220700152207,
                hit: false,
            },
        ];
        let expected = items.into_iter().collect::<Vec<StackInfoReadable>>();
        assert_eq!(stacks, expected);
        assert_eq!(fg.total_count(), 657);
        assert_eq!(
            *fg.root(),
            StackInfo {
                id: ROOT_ID,
                line_index: 0,
                start_index: 0,
                end_index: 0,
                total_count: 657,
                self_count: 0,
                width_factor: 1.0,
                parent: None,
                children: vec![3, 1, 5],
                level: 0,
                hit: false,
            }
        );
    }

    #[test]
    fn test_no_name_count() {
        let content = std::fs::read_to_string("tests/data/invalid-lines.txt").unwrap();
        let fg = FlameGraph::from_string(content, true);
        let stacks = get_readable_stacks(&fg);
        // _print_stacks(&fg);
        let items: Vec<StackInfoReadable> = vec![
            StackInfoReadable {
                id: 0,
                short_name: "all",
                full_name: "all",
                total_count: 428,
                self_count: 0,
                parent: None,
                children: vec![2, 1],
                level: 0,
                width_factor: 1.0,
                hit: false,
            },
            StackInfoReadable {
                id: 1,
                short_name: "<module> (long_running.py:24)",
                full_name: "<module> (long_running.py:24)",
                total_count: 7,
                self_count: 7,
                parent: Some(0),
                children: vec![],
                level: 1,
                width_factor: 0.016355140186915886,
                hit: false,
            },
            StackInfoReadable {
                id: 2,
                short_name: "<module> (long_running.py:25)",
                full_name: "<module> (long_running.py:25)",
                total_count: 421,
                self_count: 421,
                parent: Some(0),
                children: vec![],
                level: 1,
                width_factor: 0.9836448598130841,
                hit: false,
            },
        ];
        let expected = items.into_iter().collect::<Vec<StackInfoReadable>>();
        assert_eq!(stacks, expected);
        assert_eq!(fg.total_count(), 428);
    }

    #[test]
    fn test_ignore_lines_starting_with_hash() {
        let content = std::fs::read_to_string("tests/data/ignore-metadata-lines.txt").unwrap();
        let fg = FlameGraph::from_string(content, true);
        let stacks = get_readable_stacks(&fg);
        // _print_stacks(&fg);
        let expected = vec![
            StackInfoReadable {
                id: 0,
                short_name: "all",
                full_name: "all",
                total_count: 7,
                self_count: 0,
                parent: None,
                children: vec![1],
                level: 0,
                width_factor: 1.0,
                hit: false,
            },
            StackInfoReadable {
                id: 1,
                short_name: "<module> (long_running.py:24)",
                full_name: "<module> (long_running.py:24)",
                total_count: 7,
                self_count: 7,
                parent: Some(0),
                children: vec![],
                level: 1,
                width_factor: 1.0,
                hit: false,
            },
        ];
        assert_eq!(stacks, expected);
    }
}

pub type StackIdentifier = usize;
pub static ROOT: &str = "root";
pub static ROOT_ID: usize = 0;

#[derive(Debug, Clone, PartialEq)]
pub struct StackInfo {
    pub id: StackIdentifier,
    pub short_name: String,
    pub full_name: String,
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
pub struct FlameGraph {
    stacks: Vec<StackInfo>,
    levels: Vec<Vec<StackIdentifier>>,
    pub hit_coverage_count: Option<u64>,
}

impl FlameGraph {
    pub fn from_string(content: &str) -> Self {
        let mut stacks = Vec::<StackInfo>::new();
        stacks.push(StackInfo {
            id: ROOT_ID,
            short_name: ROOT.to_string(),
            full_name: ROOT.to_string(),
            total_count: 0,
            self_count: 0,
            width_factor: 0.0,
            parent: None,
            children: Vec::<StackIdentifier>::new(),
            level: 0,
            hit: false,
        });
        for line in content.lines() {
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
            if line_and_count.is_none() {
                continue;
            }
            let (line, count) = line_and_count.unwrap();

            stacks[ROOT_ID].total_count += count;
            let mut leading = "".to_string();
            let mut parent_id = ROOT_ID;
            let mut level = 1;
            for part in line.split(';') {
                let full_name = if leading.is_empty() {
                    part.to_string()
                } else {
                    format!("{};{}", leading, part)
                };
                // Invariant: parent always exists
                let parent_stack = stacks.get(parent_id).unwrap();
                let current_stack_id_if_exists = parent_stack
                    .children
                    .iter()
                    .find(|child_id| {
                        let child = stacks.get(**child_id).unwrap();
                        child.full_name == full_name
                    })
                    .cloned();
                let stack_id = if let Some(stack_id) = current_stack_id_if_exists {
                    stack_id
                } else {
                    stacks.push(StackInfo {
                        id: stacks.len(),
                        short_name: part.to_string(),
                        full_name: full_name.clone(),
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
                if full_name == line {
                    info.self_count += count;
                }
                leading = full_name;
                parent_id = stack_id;
                level += 1;
            }
        }

        let mut out = Self {
            stacks,
            levels: vec![],
            hit_coverage_count: None,
        };
        out.populate_levels(&ROOT_ID, 0, None);
        out
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
        let mut sorted_children = stack.children.clone();
        sorted_children.sort_by_key(|child_id| {
            self.stacks
                .get(*child_id)
                .map(|child| child.total_count)
                .unwrap_or(0)
        });
        sorted_children.reverse();

        // Make the updates to the current stack
        let stack = self.stacks.get_mut(*stack_id).unwrap();
        stack.width_factor = width_factor;
        stack.children = sorted_children;

        // Move on to children
        for child_id in stack.children.clone().iter() {
            self.populate_levels(child_id, level + 1, Some((total_count, width_factor)));
        }
    }

    pub fn get_stack(&self, stack_id: &StackIdentifier) -> Option<&StackInfo> {
        self.stacks.get(*stack_id)
    }

    pub fn get_stack_by_full_name(&self, full_name: &str) -> Option<&StackInfo> {
        self.stacks
            .iter()
            .find(|stack| stack.full_name == full_name)
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

    pub fn is_ancenstor_or_descendant(
        &self,
        stack_id: &StackIdentifier,
        other_id: &StackIdentifier,
    ) -> bool {
        self.get_ancestors(stack_id).contains(other_id)
            || self.get_descendants(stack_id).contains(other_id)
    }

    pub fn set_hits(&mut self, p: &SearchPattern) {
        self.stacks.iter_mut().for_each(|stack| {
            stack.hit = p.re.is_match(&stack.short_name);
        });
        self.hit_coverage_count = Some(self._count_hit_coverage(ROOT_ID));
    }

    pub fn clear_hits(&mut self) {
        self.stacks.iter_mut().for_each(|stack| stack.hit = false);
        self.hit_coverage_count = None;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn _print_stacks(fg: &FlameGraph) {
        let mut sorted_stacks = fg.stacks.iter().collect::<Vec<_>>();
        sorted_stacks.sort_by_key(|x| x.id);
        for v in sorted_stacks.iter() {
            println!(
                "full_name: {} width_factor: {}",
                v.full_name, v.width_factor,
            );
        }
        println!("{:?}", sorted_stacks);
    }

    #[test]
    fn test_simple() {
        let content = std::fs::read_to_string("tests/data/py-spy-simple.txt").unwrap();
        let fg = FlameGraph::from_string(&content);
        // _print_stacks(&fg);
        let items: Vec<StackInfo> = vec![
            StackInfo {
                id: 0,
                short_name: "root".into(),
                full_name: "root".into(),
                total_count: 657,
                self_count: 0,
                parent: None,
                children: vec![1, 3, 5],
                level: 0,
                width_factor: 1.0,
                hit: false,
            },
            StackInfo {
                id: 1,
                short_name: "<module> (long_running.py:24)".into(),
                full_name: "<module> (long_running.py:24)".into(),
                total_count: 17,
                self_count: 0,
                parent: Some(0),
                children: vec![2, 7],
                level: 1,
                width_factor: 0.0258751902587519,
                hit: false,
            },
            StackInfo {
                id: 2,
                short_name: "quick_work (long_running.py:16)".into(),
                full_name: "<module> (long_running.py:24);quick_work (long_running.py:16)".into(),
                total_count: 7,
                self_count: 7,
                parent: Some(1),
                children: vec![],
                level: 2,
                width_factor: 0.0106544901065449,
                hit: false,
            },
            StackInfo {
                id: 3,
                short_name: "<module> (long_running.py:25)".into(),
                full_name: "<module> (long_running.py:25)".into(),
                total_count: 639,
                self_count: 0,
                parent: Some(0),
                children: vec![4, 6],
                level: 1,
                width_factor: 0.9726027397260274,
                hit: false,
            },
            StackInfo {
                id: 4,
                short_name: "work (long_running.py:8)".into(),
                full_name: "<module> (long_running.py:25);work (long_running.py:8)".into(),
                total_count: 421,
                self_count: 421,
                parent: Some(3),
                children: vec![],
                level: 2,
                width_factor: 0.6407914764079147,
                hit: false,
            },
            StackInfo {
                id: 5,
                short_name: "<module> (long_running.py:26)".into(),
                full_name: "<module> (long_running.py:26)".into(),
                total_count: 1,
                self_count: 1,
                parent: Some(0),
                children: vec![],
                level: 1,
                width_factor: 0.0015220700152207,
                hit: false,
            },
            StackInfo {
                id: 6,
                short_name: "work (long_running.py:7)".into(),
                full_name: "<module> (long_running.py:25);work (long_running.py:7)".into(),
                total_count: 218,
                self_count: 218,
                parent: Some(3),
                children: vec![],
                level: 2,
                width_factor: 0.3318112633181126,
                hit: false,
            },
            StackInfo {
                id: 7,
                short_name: "quick_work (long_running.py:17)".into(),
                full_name: "<module> (long_running.py:24);quick_work (long_running.py:17)".into(),
                total_count: 10,
                self_count: 10,
                parent: Some(1),
                children: vec![],
                level: 2,
                width_factor: 0.015220700152207,
                hit: false,
            },
        ];
        let expected = items.into_iter().collect::<Vec<StackInfo>>();
        assert_eq!(fg.stacks, expected);
        assert_eq!(fg.total_count(), 657);
        assert_eq!(
            *fg.root(),
            StackInfo {
                id: ROOT_ID,
                short_name: "root".into(),
                full_name: "root".into(),
                total_count: 657,
                self_count: 0,
                width_factor: 1.0,
                parent: None,
                children: vec![1, 3, 5],
                level: 0,
                hit: false,
            }
        );
    }
}

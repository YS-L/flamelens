use std::collections::HashMap;

pub type StackIdentifier = String;
pub static ROOT: &str = "root";

#[derive(Debug, Clone, PartialEq)]
pub struct StackInfo {
    pub short_name: String,
    pub full_name: StackIdentifier,
    pub total_count: u64,
    pub self_count: u64,
    pub parent: Option<StackIdentifier>,
    pub children: Vec<StackIdentifier>,
    pub level: usize,
    pub width_factor: f64,
}

#[derive(Debug, Clone)]
pub struct FlameGraph {
    stacks: HashMap<StackIdentifier, StackInfo>,
    levels: Vec<Vec<StackIdentifier>>,
}

impl FlameGraph {
    pub fn from_file(filename: &str) -> Self {
        let mut stacks = HashMap::<StackIdentifier, StackInfo>::new();
        stacks.insert(
            ROOT.to_string(),
            StackInfo {
                short_name: ROOT.to_string(),
                full_name: ROOT.to_string(),
                total_count: 0,
                self_count: 0,
                width_factor: 0.0,
                parent: None,
                children: Vec::<StackIdentifier>::new(),
                level: 0,
            },
        );
        for line in std::fs::read_to_string(filename)
            .expect("Could not read file")
            .lines()
        {
            let (line, count) = line.rsplit_once(' ').unwrap();
            let count = count.parse::<u64>().unwrap();

            stacks.get_mut(ROOT).unwrap().total_count += count;
            let mut leading = "".to_string();
            let mut level = 1;
            for part in line.split(';') {
                let full_name = if leading.is_empty() {
                    part.to_string()
                } else {
                    format!("{};{}", leading, part)
                };
                if !stacks.contains_key(&full_name) {
                    stacks.insert(
                        full_name.clone(),
                        StackInfo {
                            short_name: part.to_string(),
                            full_name: full_name.clone(),
                            total_count: 0,
                            self_count: 0,
                            width_factor: 0.0,
                            parent: if leading.is_empty() {
                                Some(ROOT.to_string())
                            } else {
                                Some(leading.clone())
                            },
                            children: Vec::<StackIdentifier>::new(),
                            level,
                        },
                    );
                }
                let info = stacks.get_mut(&full_name).unwrap();
                info.total_count += count;
                if full_name == line {
                    info.self_count += count;
                }
                let parent_id = info.parent.clone();
                if let Some(parent_id) = parent_id {
                    let parent = stacks.get_mut(&parent_id).unwrap();
                    if !parent.children.contains(&full_name) {
                        parent.children.push(full_name.clone());
                    }
                }
                leading = full_name;
                level += 1;
            }
        }

        let mut out = Self {
            stacks,
            levels: vec![],
        };
        out.populate_levels(&ROOT.to_string(), 0, None);
        out
    }

    fn populate_levels(
        &mut self,
        stack_id: &StackIdentifier,
        level: usize,
        parent_total_count_and_width_factor: Option<(u64, f64)>,
    ) {
        if self.levels.len() <= level {
            self.levels.push(vec![]);
        }
        self.levels[level].push(stack_id.clone());
        let stack = self.stacks.get_mut(stack_id).unwrap();
        let total_count = stack.total_count;
        let width_factor = if let Some((parent_total_count, parent_width_factor)) =
            parent_total_count_and_width_factor
        {
            parent_width_factor * (total_count as f64 / parent_total_count as f64)
        } else {
            1.0
        };
        stack.width_factor = width_factor;
        let children = stack.children.clone();
        for child_id in children.iter() {
            self.populate_levels(child_id, level + 1, Some((total_count, width_factor)));
        }
    }

    pub fn get_stack(&self, stack_id: &StackIdentifier) -> Option<&StackInfo> {
        self.stacks.get(stack_id)
    }

    pub fn get_stacks_at_level(&self, level: usize) -> Option<&Vec<StackIdentifier>> {
        self.levels.get(level)
    }

    pub fn root(&self) -> &StackInfo {
        // TODO: weird
        self.get_stack(&ROOT.to_string()).unwrap()
    }

    pub fn total_count(&self) -> u64 {
        self.root().total_count
    }

    pub fn get_stack_identifiers(&self) -> Vec<StackIdentifier> {
        self.stacks.keys().cloned().collect()
    }

    pub fn get_num_levels(&self) -> usize {
        self.levels.len()
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Reverse;

    use super::*;

    fn _print_stacks(fg: &FlameGraph) {
        let mut sorted_stacks = fg.stacks.iter().collect::<Vec<_>>();
        sorted_stacks.sort_by_key(|x| Reverse(&x.1.total_count));
        for (_k, v) in sorted_stacks.iter() {
            println!(
                "full_name: {} width_factor: {}",
                v.full_name, v.width_factor,
            );
        }
        println!("{:?}", sorted_stacks);
    }

    #[test]
    fn test_simple() {
        let fg = FlameGraph::from_file("tests/data/py-spy-simple.txt");
        // _print_stacks(&fg);
        let items: Vec<(StackIdentifier, StackInfo)> = vec![
            (
                "root".into(),
                StackInfo {
                    short_name: "root".into(),
                    full_name: "root".into(),
                    total_count: 657,
                    self_count: 0,
                    width_factor: 1.0,
                    parent: None,
                    children: vec![
                        "<module> (long_running.py:24)".into(),
                        "<module> (long_running.py:25)".into(),
                        "<module> (long_running.py:26)".into(),
                    ],
                    level: 0,
                },
            ),
            (
                "<module> (long_running.py:25)".into(),
                StackInfo {
                    short_name: "<module> (long_running.py:25)".into(),
                    full_name: "<module> (long_running.py:25)".into(),
                    total_count: 639,
                    self_count: 0,
                    width_factor: 0.9726027397260274,
                    parent: Some("root".into()),
                    children: vec![
                        "<module> (long_running.py:25);work (long_running.py:8)".into(),
                        "<module> (long_running.py:25);work (long_running.py:7)".into(),
                    ],
                    level: 1,
                },
            ),
            (
                "<module> (long_running.py:25);work (long_running.py:8)".into(),
                StackInfo {
                    short_name: "work (long_running.py:8)".into(),
                    full_name: "<module> (long_running.py:25);work (long_running.py:8)".into(),
                    total_count: 421,
                    self_count: 421,
                    width_factor: 0.6407914764079147,
                    parent: Some("<module> (long_running.py:25)".into()),
                    children: vec![],
                    level: 2,
                },
            ),
            (
                "<module> (long_running.py:25);work (long_running.py:7)".into(),
                StackInfo {
                    short_name: "work (long_running.py:7)".into(),
                    full_name: "<module> (long_running.py:25);work (long_running.py:7)".into(),
                    total_count: 218,
                    self_count: 218,
                    width_factor: 0.3318112633181126,
                    parent: Some("<module> (long_running.py:25)".into()),
                    children: vec![],
                    level: 2,
                },
            ),
            (
                "<module> (long_running.py:24)".into(),
                StackInfo {
                    short_name: "<module> (long_running.py:24)".into(),
                    full_name: "<module> (long_running.py:24)".into(),
                    total_count: 17,
                    self_count: 0,
                    width_factor: 0.0258751902587519,
                    parent: Some("root".into()),
                    children: vec![
                        "<module> (long_running.py:24);quick_work (long_running.py:16)".into(),
                        "<module> (long_running.py:24);quick_work (long_running.py:17)".into(),
                    ],
                    level: 1,
                },
            ),
            (
                "<module> (long_running.py:24);quick_work (long_running.py:17)".into(),
                StackInfo {
                    short_name: "quick_work (long_running.py:17)".into(),
                    full_name: "<module> (long_running.py:24);quick_work (long_running.py:17)"
                        .into(),
                    total_count: 10,
                    self_count: 10,
                    width_factor: 0.015220700152207,
                    parent: Some("<module> (long_running.py:24)".into()),
                    children: vec![],
                    level: 2,
                },
            ),
            (
                "<module> (long_running.py:24);quick_work (long_running.py:16)".into(),
                StackInfo {
                    short_name: "quick_work (long_running.py:16)".into(),
                    full_name: "<module> (long_running.py:24);quick_work (long_running.py:16)"
                        .into(),
                    total_count: 7,
                    self_count: 7,
                    width_factor: 0.0106544901065449,
                    parent: Some("<module> (long_running.py:24)".into()),
                    children: vec![],
                    level: 2,
                },
            ),
            (
                "<module> (long_running.py:26)".into(),
                StackInfo {
                    short_name: "<module> (long_running.py:26)".into(),
                    full_name: "<module> (long_running.py:26)".into(),
                    total_count: 1,
                    self_count: 1,
                    width_factor: 0.0015220700152207,
                    parent: Some("root".into()),
                    children: vec![],
                    level: 1,
                },
            ),
        ];
        let expected = items
            .into_iter()
            .collect::<HashMap<StackIdentifier, StackInfo>>();
        assert_eq!(fg.stacks, expected);
        assert_eq!(fg.total_count(), 657);
        assert_eq!(
            *fg.root(),
            StackInfo {
                short_name: "root".into(),
                full_name: "root".into(),
                total_count: 657,
                self_count: 0,
                width_factor: 1.0,
                parent: None,
                children: vec![
                    "<module> (long_running.py:24)".into(),
                    "<module> (long_running.py:25)".into(),
                    "<module> (long_running.py:26)".into()
                ],
                level: 0,
            }
        );
    }
}

pub fn run() {
    // Get first argument from command line
    let filename = std::env::args().nth(1).expect("No filename given");
    println!("Reading from file: {}", filename);

    let flamegraph = FlameGraph::from_file(&filename);

    for info in flamegraph.stacks.values() {
        println!(
            "short_name:{} total:{} self:{} num_child:{}",
            info.short_name,
            info.total_count,
            info.self_count,
            info.children.len()
        );
    }

    for info in flamegraph.stacks.values() {
        if info.parent.is_none() {
            println!(
                "[root] short_name:{} total:{} self:{} num_child:{}",
                info.short_name,
                info.total_count,
                info.self_count,
                info.children.len()
            );
        }
    }
}

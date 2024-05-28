use std::collections::HashMap;

pub type StackIdentifier = String;
pub static ROOT: &str = "root";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackInfo {
    pub short_name: String,
    pub full_name: StackIdentifier,
    pub total_count: u64,
    pub self_count: u64,
    pub parent: Option<StackIdentifier>,
    pub children: Vec<StackIdentifier>,
}

#[derive(Debug, Clone)]
pub struct FlameGraph {
    stacks: HashMap<StackIdentifier, StackInfo>,
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
                parent: None,
                children: Vec::<StackIdentifier>::new(),
            },
        );
        for line in std::fs::read_to_string(filename)
            .expect("Could not read file")
            .lines()
        {
            let (line, count) = line.rsplit_once(' ').unwrap();
            let count = count.parse::<u64>().unwrap();

            stacks.get_mut(&ROOT.to_string()).unwrap().total_count += count;
            let mut leading = "".to_string();
            for part in line.split(';') {
                let full_name = if leading.is_empty() {
                    part.to_string()
                } else {
                    format!("{};{}", leading, part)
                };
                if stacks.get(&full_name).is_none() {
                    stacks.insert(
                        full_name.clone(),
                        StackInfo {
                            short_name: part.to_string(),
                            full_name: full_name.clone(),
                            total_count: 0,
                            self_count: 0,
                            parent: if leading.is_empty() {
                                Some(ROOT.to_string())
                            } else {
                                Some(leading.clone())
                            },
                            children: Vec::<StackIdentifier>::new(),
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
            }
        }

        Self { stacks }
    }

    pub fn get_stack(&self, stack_id: &StackIdentifier) -> Option<&StackInfo> {
        self.stacks.get(stack_id)
    }

    pub fn root(&self) -> &StackInfo {
        // TODO: weird
        self.get_stack(&ROOT.to_string()).unwrap()
    }

    pub fn total_count(&self) -> u64 {
        self.root().total_count
    }

    fn get_parent_and_position_as_sibling(
        &self,
        stack_id: &StackIdentifier,
    ) -> Option<(&StackInfo, usize)> {
        let stack = self.get_stack(stack_id)?;
        let parent = self.get_stack(stack.parent.as_ref()?)?;
        let idx = parent.children.iter().position(|x| x == stack_id)?;
        Some((parent, idx))
    }

    pub fn get_next_sibling(&self, stack_id: &StackIdentifier) -> Option<&StackInfo> {
        let (parent, idx) = self.get_parent_and_position_as_sibling(stack_id)?;
        if idx + 1 < parent.children.len() {
            self.get_stack(&parent.children[idx + 1])
        } else {
            None
        }
    }

    pub fn get_previous_sibling(&self, stack_id: &StackIdentifier) -> Option<&StackInfo> {
        let (parent, idx) = self.get_parent_and_position_as_sibling(stack_id)?;
        if idx > 0 {
            self.get_stack(&parent.children[idx - 1])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Reverse;

    use super::*;

    fn _print_stacks(fg: &FlameGraph) {
        let mut sorted_stacks = fg.stacks.iter().collect::<Vec<_>>();
        sorted_stacks.sort_by_key(|x| Reverse(&x.1.total_count));
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
                    parent: None,
                    children: vec![
                        "<module> (long_running.py:24)".into(),
                        "<module> (long_running.py:25)".into(),
                        "<module> (long_running.py:26)".into(),
                    ],
                },
            ),
            (
                "<module> (long_running.py:25)".into(),
                StackInfo {
                    short_name: "<module> (long_running.py:25)".into(),
                    full_name: "<module> (long_running.py:25)".into(),
                    total_count: 639,
                    self_count: 0,
                    parent: Some("root".into()),
                    children: vec![
                        "<module> (long_running.py:25);work (long_running.py:8)".into(),
                        "<module> (long_running.py:25);work (long_running.py:7)".into(),
                    ],
                },
            ),
            (
                "<module> (long_running.py:25);work (long_running.py:8)".into(),
                StackInfo {
                    short_name: "work (long_running.py:8)".into(),
                    full_name: "<module> (long_running.py:25);work (long_running.py:8)".into(),
                    total_count: 421,
                    self_count: 421,
                    parent: Some("<module> (long_running.py:25)".into()),
                    children: vec![],
                },
            ),
            (
                "<module> (long_running.py:25);work (long_running.py:7)".into(),
                StackInfo {
                    short_name: "work (long_running.py:7)".into(),
                    full_name: "<module> (long_running.py:25);work (long_running.py:7)".into(),
                    total_count: 218,
                    self_count: 218,
                    parent: Some("<module> (long_running.py:25)".into()),
                    children: vec![],
                },
            ),
            (
                "<module> (long_running.py:24)".into(),
                StackInfo {
                    short_name: "<module> (long_running.py:24)".into(),
                    full_name: "<module> (long_running.py:24)".into(),
                    total_count: 17,
                    self_count: 0,
                    parent: Some("root".into()),
                    children: vec![
                        "<module> (long_running.py:24);quick_work (long_running.py:16)".into(),
                        "<module> (long_running.py:24);quick_work (long_running.py:17)".into(),
                    ],
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
                    parent: Some("<module> (long_running.py:24)".into()),
                    children: vec![],
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
                    parent: Some("<module> (long_running.py:24)".into()),
                    children: vec![],
                },
            ),
            (
                "<module> (long_running.py:26)".into(),
                StackInfo {
                    short_name: "<module> (long_running.py:26)".into(),
                    full_name: "<module> (long_running.py:26)".into(),
                    total_count: 1,
                    self_count: 1,
                    parent: Some("root".into()),
                    children: vec![],
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
                parent: None,
                children: vec![
                    "<module> (long_running.py:24)".into(),
                    "<module> (long_running.py:25)".into(),
                    "<module> (long_running.py:26)".into()
                ]
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

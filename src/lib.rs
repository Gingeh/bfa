use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    fmt::{Display, Write},
    num::NonZeroUsize,
};

use rustc_hash::FxBuildHasher;
use smallvec::{smallvec, SmallVec};

#[derive(Clone, Copy, Debug)]
pub enum Instruction {
    MoveLeft,
    MoveRight,
    Increment,
    Decrement,
    StartLoop,
    EndLoop,
    Read,
    Accept,
}

impl Instruction {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '<' => Some(Self::MoveLeft),
            '>' => Some(Self::MoveRight),
            '+' => Some(Self::Increment),
            '-' => Some(Self::Decrement),
            '[' => Some(Self::StartLoop),
            ']' => Some(Self::EndLoop),
            ',' => Some(Self::Read),
            '.' => Some(Self::Accept),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Program {
    pub cell_count: NonZeroUsize,
    pub instructions: Vec<Instruction>,
}

impl Program {
    pub fn new(program_text: &str, cell_count: NonZeroUsize) -> Self {
        let instructions = program_text
            .chars()
            .filter_map(Instruction::from_char)
            .collect();

        Self {
            cell_count,
            instructions,
        }
    }

    fn run_with_next_input(
        &self,
        mut state: InnerState,
        input: u8,
        seen_states: &mut HashMap<InnerState, (), FxBuildHasher>,
    ) -> State {
        state.cells.set(state.head_position, input);
        let mut accepting = false;

        'outer: while let Some(&intruction) = self.instructions.get(state.instruction_position) {
            match intruction {
                Instruction::MoveLeft => {
                    if state.head_position == 0 {
                        state.head_position = self.cell_count.get() - 1;
                    } else {
                        state.head_position -= 1;
                    }
                }
                Instruction::MoveRight => {
                    if state.head_position == self.cell_count.get() - 1 {
                        state.head_position = 0;
                    } else {
                        state.head_position += 1;
                    }
                }
                Instruction::Increment => {
                    state.cells.set(
                        state.head_position,
                        state.cells.get(state.head_position) + 1,
                    );
                }
                Instruction::Decrement => {
                    state.cells.set(
                        state.head_position,
                        state.cells.get(state.head_position).wrapping_sub(1),
                    );
                }
                Instruction::EndLoop => {
                    // unconditional!
                    // find and jump to matching StartLoop
                    let mut nesting = 0;
                    while let Some(&intruction) = self.instructions.get(state.instruction_position)
                    {
                        match intruction {
                            Instruction::StartLoop => {
                                nesting -= 1;
                                if nesting == 0 {
                                    break;
                                }
                            }
                            Instruction::EndLoop => nesting += 1,
                            _ => {}
                        }

                        if state.instruction_position == 0 {
                            break 'outer;
                        }
                        state.instruction_position -= 1;
                    }
                    continue; // <- to avoid incrementing instruction_position
                }
                Instruction::StartLoop => {
                    // if current cell is 0
                    if state.cells.get(state.head_position) == 0 {
                        // find and jump to matching EndLoop
                        let mut nesting = 0;
                        while let Some(&intruction) =
                            self.instructions.get(state.instruction_position)
                        {
                            match intruction {
                                Instruction::StartLoop => nesting += 1,
                                Instruction::EndLoop => {
                                    nesting -= 1;
                                    if nesting == 0 {
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            state.instruction_position += 1;
                            if state.instruction_position == self.instructions.len() {
                                break 'outer;
                            }
                        }
                    } else {
                        match seen_states.entry(state.clone()) {
                            Entry::Occupied(_) => break 'outer,
                            Entry::Vacant(slot) => slot.insert(()),
                        };
                    }
                }
                Instruction::Read => {
                    state.instruction_position += 1;
                    return State {
                        inner: Some(state),
                        accepting,
                    };
                }
                Instruction::Accept => accepting = true,
            }

            state.instruction_position += 1;
        }

        State {
            inner: None,
            accepting,
        }
    }
}

#[repr(transparent)]
#[derive(Eq, Hash, PartialEq, Clone, Debug)]
struct U4Vec(SmallVec<u8, { std::mem::size_of::<usize>() * 2 }>);

impl U4Vec {
    #[inline]
    fn get(&self, index: usize) -> u8 {
        (self.0[index / 2] >> (4 * (index & 1))) & 0x0F
    }

    #[inline]
    fn set(&mut self, index: usize, value: u8) {
        self.0[index / 2] &= 0xF0 >> (4 * (index & 1));
        self.0[index / 2] |= (value & 0x0F) << (4 * (index & 1))
    }
}

impl Display for U4Vec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('[')?;
        for pair in &self.0 {
            let first = pair & 0x0F;
            let second = (pair >> 4) & 0x0F;
            write!(f, "{first:X}{second:X}")?;
        }
        f.write_char(']')?;
        Ok(())
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
struct InnerState {
    cells: U4Vec,
    head_position: usize,
    instruction_position: usize,
}

#[derive(Eq, Hash, PartialEq, Clone, Debug)]
struct State {
    inner: Option<InnerState>,
    accepting: bool,
}

#[derive(Debug)]
pub struct Table {
    states: Vec<(bool, [usize; 16])>,
}

impl Table {
    pub fn build(program: &Program) -> Self {
        let mut state_ids = HashMap::with_hasher(FxBuildHasher);
        let mut table = Self { states: vec![] };
        let mut exploration_stack: Vec<State> = Vec::new();

        let mut seen_states = HashMap::with_hasher(FxBuildHasher);

        let start = program.run_with_next_input(
            InnerState {
                cells: U4Vec(smallvec![0; program.cell_count.get().div_ceil(2)]),
                head_position: 0,
                instruction_position: 0,
            },
            0,
            &mut seen_states,
        );
        seen_states.clear();

        exploration_stack.push(start.clone());
        table.states.push((start.accepting, [0; 16]));
        state_ids.insert(start, 0);

        while let Some(current) = exploration_stack.pop() {
            let current_id = *state_ids.get(&current).unwrap();
            if current.inner.is_none() {
                table.states[current_id] = (current.accepting, [current_id; 16]);
                continue;
            }
            for input in 0..16 {
                let next = program.run_with_next_input(
                    current.inner.as_ref().unwrap().clone(),
                    input,
                    &mut seen_states,
                );
                seen_states.clear();
                let next_id = state_ids.entry(next.clone()).or_insert_with(|| {
                    table.states.push((next.accepting, [0; 16]));
                    exploration_stack.push(next);
                    table.states.len() - 1
                });
                table.states[current_id].1[input as usize] = *next_id;
            }
        }

        table
    }

    pub fn minimize(&mut self) {
        let mut partition: Vec<usize> = vec![0; self.states.len()];
        let mut partition_reps = vec![0];

        let initial_accepting = self.states[0].0;
        let mut seen_different = false;
        for (id, (accepting, _)) in self.states.iter().enumerate() {
            if *accepting != initial_accepting {
                partition[id] = 1;
                if !seen_different {
                    seen_different = true;
                    partition_reps.push(id);
                }
            }
        }

        let mut queue: Vec<usize> = Vec::new();
        queue.push(0);
        if seen_different {
            queue.push(1);
        }

        while let Some(current) = queue.pop() {
            for input in 0..16 {
                let preimage: HashSet<usize, FxBuildHasher> = self
                    .states
                    .iter()
                    .enumerate()
                    .filter(|(_, (_, trans))| partition[trans[input]] == current)
                    .map(|(i, _)| i)
                    .collect();

                for part in 0..partition_reps.len() {
                    let (intersection, remainder): (Vec<usize>, Vec<usize>) = partition
                        .iter()
                        .enumerate()
                        .filter_map(|(state, &id)| if id == part { Some(state) } else { None })
                        .partition(|state| preimage.contains(state));

                    if intersection.is_empty() || remainder.is_empty() {
                        continue;
                    }

                    let lower;
                    let higher;
                    let inter_id;
                    let remain_id;

                    if intersection[0] < remainder[0] {
                        lower = &intersection;
                        higher = &remainder;
                        inter_id = part;
                        remain_id = partition_reps.len();
                    } else {
                        lower = &remainder;
                        higher = &intersection;
                        inter_id = partition_reps.len();
                        remain_id = part;
                    }

                    for &state in higher {
                        partition[state] = partition_reps.len();
                    }

                    partition_reps.push(higher[0]);
                    partition_reps[part] = lower[0];

                    if queue.contains(&inter_id) {
                        queue.push(remain_id);
                    } else if intersection.len() <= remainder.len() {
                        queue.push(inter_id);
                    } else {
                        queue.push(remain_id);
                    }
                }
            }
        }

        let mut new_states = Vec::with_capacity(partition_reps.len());
        for old_id in partition_reps {
            for edge in &mut self.states[old_id].1 {
                *edge = partition[*edge];
            }
            new_states.push(self.states[old_id]);
        }

        new_states.shrink_to_fit();
        self.states = new_states;
    }

    pub fn dot(&self) -> String {
        let mut output = "digraph G {\n".to_string();

        for (from, (_, edges)) in self.states.iter().enumerate() {
            for maybe_to in 0..self.states.len() {
                let mut empty = true;
                let mut run_start = None;
                let mut end_run = |run_start: &mut Option<usize>, input| {
                    if run_start.is_some() {
                        let start = run_start.take().unwrap();
                        if empty {
                            empty = false;
                            write!(&mut output, "    {from} -> {maybe_to} [label=\"").unwrap();
                        }
                        if input - start < 4 {
                            for n in start..input {
                                write!(&mut output, "{n:X}").unwrap();
                            }
                        } else {
                            write!(&mut output, "{start:X}-{:X}", input - 1).unwrap();
                        }
                    }
                };
                for (input, &to) in edges.iter().enumerate() {
                    if to == maybe_to && run_start.is_none() {
                        run_start = Some(input);
                    } else if to != maybe_to {
                        end_run(&mut run_start, input);
                    }
                }
                end_run(&mut run_start, 16);
                if !empty {
                    writeln!(&mut output, "\"];").unwrap();
                }
            }
        }

        for (id, (accepting, _)) in self.states.iter().enumerate() {
            if *accepting {
                writeln!(&mut output, "    {id}[peripheries=2];").unwrap();
            }
        }

        writeln!(&mut output, "}}").unwrap();
        output
    }
}

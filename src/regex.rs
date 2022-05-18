

#[derive(Clone, Copy, Default)]
struct Transition {
    on_character: Option<u8>,
    to_state_idx: usize
}

#[derive(Clone, Copy)]
struct State {
    transition_count: usize,
    transitions: [Transition; 16],
}

impl Default for State {
    fn default() -> Self {
        Self {
            transition_count: 0,
            transitions: Default::default()
        }
    }
}

impl State {
    const fn add_transition(mut self, on_character: Option<u8>, to_state_idx: usize) -> Self {
        self.transitions[self.transition_count].on_character = on_character;
        self.transitions[self.transition_count].to_state_idx = to_state_idx;
        self.transition_count += 1;
        self
    }
}


/// # NFA: Nondeterministic Finite Automaton
pub(crate) struct NFA<const N: usize> {
    states: [State; N],
    state_count: usize,
    accept_idx: usize,
    start_idx: usize,
}

impl<const N: usize> Default for NFA<N> {
    fn default() -> Self {
        Self {
            states: [Default::default(); N],
            state_count: 0,
            start_idx: 0,
            accept_idx: 0,
        }
    }
}

// Input: a regular expression r over an alphabet Σ
// Output: an NFA N accepting L(r)
impl<const N: usize> NFA<N> {

    pub(crate) fn debug_print(&self) {
        use crate::io::{ itoa, puts };

        puts("digraph {\n");
        puts("  S"); puts(itoa(self.start_idx as u32)); puts(" [shape=box];\n");
        puts("  S"); puts(itoa(self.accept_idx as u32)); puts(" [shape=doublecircle];\n");
        for (idx, state) in self.states[0..self.state_count].iter().enumerate() {
            for transition in &state.transitions[0..state.transition_count] {
                puts("  S"); puts(itoa(idx as u32)); puts(" -> "); puts("S"); puts(itoa(transition.to_state_idx as u32));

                puts("[label=\"");
                if let Some(byte) = transition.on_character {
                    puts([byte]);
                } else {
                    puts([0xce, 0xb5]); // epsilon
                }
                puts("\"];\n");
            }
        }
        puts("}\n");
    }

    pub(crate) const fn from_regex_bytes(input: &'static [u8]) -> Self {
        // I sure wish we could use Default::default() in const functions.
        let mut nfa = Self {
            states: [State {
                transition_count: 0,
                transitions: [Transition {
                    on_character: None,
                    to_state_idx: 0
                }; 16]
            }; N],
            state_count: 1,
            start_idx: 0,
            accept_idx: 0,
        };

        (nfa, _) = NFA::<N>::from_regex_bytes_inner(nfa, input, 0, false);
        nfa
    }

    const fn from_regex_bytes_inner(mut nfa: NFA<N>, input: &'static [u8], mut idx: usize, expect_rparen: bool) -> (Self, usize) {
        let mut escaped = false;

        /* expr ::= "(" expr ")"
         *        | expr "|" expr
         *        | expr "*"
         *        | expr "?"
         *        | expr "+"
         *        | term
         * term ::= <all characters except for "|?*()+^$\"> | "\" <any of "|?*()+^$\">
         */

        while idx < input.len() {
            match input[idx] {
                b'(' if !escaped => { // "(" s ")"; match until )
                    idx += 1;

                    (nfa, idx) = NFA::<N>::from_regex_bytes_inner(nfa, input, idx, true);
                }

                b')' if !escaped && expect_rparen => {
                    idx += 1;
                    return (nfa, idx)
                }

                b')' if !escaped && !expect_rparen => {
                    return (nfa, idx)
                }

                b'|' if !escaped => {
                    idx += 1;

                    // preserve the current start and end
                    let prev_start_idx = nfa.start_idx;
                    let prev_accept_idx = nfa.accept_idx;

                    // reset NFA to empty state
                    nfa.accept_idx = nfa.state_count;
                    nfa.start_idx = nfa.state_count;
                    nfa.state_count += 2;

                    (nfa, idx) = NFA::<N>::from_regex_bytes_inner(nfa, input, idx, false);

                    nfa = nfa.or(prev_start_idx, prev_accept_idx);
                }

                b'*' if !escaped => {
                    idx += 1;
                    nfa = nfa.kleene_star();
                }

                // not yet impl
                b'+' if !escaped => {
                    idx += 1;
                }

                b'\\' if !escaped => { // escape next char
                    idx += 1;
                    escaped = true;
                }

                0 => {
                    return (nfa, input.len())
                }

                c => {
                    escaped = false;
                    idx += 1;
                    nfa = nfa.product(c);
                }
            }
        }

        (nfa, input.len())
    }

    // Rule 1: For ε, construct an NFA where "i" is a new start state and "f" is a new accepting
    // state. This NFA recognizes the empty string, ε.
    // 
    //                 +---+  ε   +===+
    //     start ----> | i | ---> ‖ f ‖
    //                 +---+      +===+
    //
    // [Ed. Note: I've combined rules 2 and 3.b since they are... sort of the same rule.]
    //
    // Rule 2: For a in Σ, construct an NFA where "i" is a new start state and "f" is a new accepting
    // state. This NFA recognizes the character represented by "a".
    // 
    //                 +---+  a   +===+
    //     start ----> | i | ---> ‖ f ‖
    //                 +---+      +===+
    //
    // 3.b: for the regular expression "st", construct an NFA:
    //
    //                 +-------+------+
    //     start ----> Ⓘ  N(s) ○ N(t) Ⓕ
    //                 +-------+------+
    //
    const fn product(mut self, chara: u8) -> Self {
        // our start state is the same.
        // create a transition from our current accept node to a new accept node via "a"

        // TKTK: I think there's a bug here: the book sez we shouldn't _add_ a state here, we
        // should merge the current accept state into the incoming start state.
        self.states[self.accept_idx] = self.states[self.accept_idx].add_transition(Some(chara), self.state_count);
        self.accept_idx = self.state_count;
        self.state_count += 1;

        self
    }

    // 3.a: for the regular expression s|t, construct an NFA:
    //
    //                prev start      prev accept
    //                         ↓      ↓
    //                         +------+
    //                      ε  ○ N(s) ○  ε
    //                       ↗︎ +------+ ↘︎
    //                 +---+              +===+
    //     start ----> | i |              ‖ f ‖
    //                 +---+              +===+
    //                       ↘︎ +------+ ↗︎
    //                      ε  ○ N(t) ○  ε
    //                         +------+
    //                         ↑      ↑
    //            self.start_idx      self.accept_idx
    //
    const fn or(mut self, prev_start_idx: usize, prev_accept_idx: usize) -> Self {
        // 1. alloc two new states: i & f
        // 2. add two transitions from i on ε; one to prev_start_idx, and one to start_idx
        // 3. add transition from prev_accept to f
        // 4. add transition from nfa.accept_idx to f
        // 5. set nfa.start_idx to i
        // 6. set nfa.accept_idx to f

        let i_idx = self.state_count;
        let f_idx = self.state_count + 1;
        self.state_count += 2;

        self.states[i_idx] = self.states[i_idx].add_transition(None, prev_start_idx);
        self.states[i_idx] = self.states[i_idx].add_transition(None, self.start_idx);

        self.states[prev_accept_idx] = self.states[prev_accept_idx].add_transition(None, f_idx);
        self.states[self.accept_idx] = self.states[self.accept_idx].add_transition(None, f_idx);

        self.start_idx = i_idx;
        self.accept_idx = f_idx;

        self
    }

    // 3.c: for the regular expression s*, construct an NFA:
    //
    //                            +-------------+
    //                            |      ε      |
    //                 +---+  ε   |   ↙︎     ↖︎   |  ε   +===+
    //     start ----> | i | ---> | ○  N(s)   ○ | ---> ‖ f ‖
    //                 +---+      |             |      +===+
    //                       ↘︎    +-------------+   ↗︎
    //                         ↘︎                  ↗︎
    //                             --->  ε  --->
    const fn kleene_star(mut self) -> Self {
        // 1. alloc two new states: i & f
        // 2. add a transition from i to start_idx on ε
        // 3. add a transition from i to f on ε
        // 4. add a transition from accept_idx to start_idx on ε
        // 5. add a transition from accept_idx to f on ε
        // 6. set nfa.start_idx to i
        // 7. set nfa.accept_idx to f
        let i_idx = self.state_count;
        let f_idx = self.state_count + 1;
        self.state_count += 2;

        self.states[i_idx] = self.states[i_idx].add_transition(None, self.start_idx);
        self.states[i_idx] = self.states[i_idx].add_transition(None, f_idx);

        self.states[self.accept_idx] = self.states[self.accept_idx].add_transition(None, self.start_idx);
        self.states[self.accept_idx] = self.states[self.accept_idx].add_transition(None, f_idx);

        self.start_idx = i_idx;
        self.accept_idx = f_idx;

        self
    }

    // TKTKTK
    const fn one_or_more(mut self, lhs: ()) -> Self {
        self
    }
}

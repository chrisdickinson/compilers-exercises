const TRANSITIONS_PER_STATE: usize = 4;

// Depending on the N parameter to NFA, we can use smaller types to represent indices.
type NFASize = u8;
type DFASize = u8;

#[derive(Clone, Copy, Default)]
struct Transition {
    on_character: Option<u8>,
    to_state_idx: NFASize,
}

#[derive(Clone, Copy)]
struct State {
    transition_count: NFASize,
    transitions: [Transition; TRANSITIONS_PER_STATE],
}

impl Default for State {
    fn default() -> Self {
        Self {
            transition_count: 0,
            transitions: [Transition {
                on_character: None,
                to_state_idx: 0,
            }; TRANSITIONS_PER_STATE],
        }
    }
}

impl State {
    const fn add_transition(mut self, on_character: Option<u8>, to_state_idx: NFASize) -> Self {
        self.transitions[self.transition_count as usize].on_character = on_character;
        self.transitions[self.transition_count as usize].to_state_idx = to_state_idx;
        self.transition_count += 1;
        self
    }
}

/// # NFA: Nondeterministic Finite Automaton
pub(crate) struct Nfa<const N: usize> {
    states: [State; N],
    state_count: NFASize,
    accept_idx: NFASize,
    start_idx: NFASize,
}

pub(crate) struct Dfa<const N: usize> {
    states: [(u8, DFASize); N],
    state_count: DFASize,
    accept_idx: DFASize,
    start_id: DFASize,
}

impl<const N: usize> Default for Nfa<N> {
    fn default() -> Self {
        Self {
            states: [Default::default(); N],
            state_count: 0,
            start_idx: 0,
            accept_idx: 0,
        }
    }
}

fn ε_closure<const N: usize>(n: &Nfa<N>) -> Dfa<N> {
    /*
     * T is a set of NFA states
     * push all states in T onto stack
     * initialize ε_closure(T) to T
     *
     * while stack is not empty; do
     *   pop t off of stack
     *   for each state u with an edge from t to u labeled ε; do
     *     if u is not in ε_closure(T); do
     *       add u to ε_closure(T)
     *       push u onto stack
     *     done
     *   done
     * done
     */
    Dfa {
        states: [Default::default(); N],
        state_count: 0,
        accept_idx: 0,
        start_id: 0,
    }
}

// Input: a regular expression r over an alphabet Σ
// Output: an NFA N accepting L(r)
impl<const N: usize> Nfa<N> {
    pub(crate) const fn from_regex_bytes(input: &'static [u8]) -> Self {
        // I sure wish we could use Default::default() in const functions.
        let nfa = Self {
            states: [State {
                transition_count: 0,
                transitions: [Transition {
                    on_character: None,
                    to_state_idx: 0,
                }; TRANSITIONS_PER_STATE],
            }; N],
            state_count: 0,
            start_idx: 0,
            accept_idx: 0,
        };

        let (nfa, idx) = nfa.expr(input, 0);
        if idx != input.len() {
            panic!("unexpected character");
        }

        nfa
    }

    /* Language:
     *
     *     expr     -> term rest
     *     rest     -> "|" expr
     *               | "(" expr ")" postfix
     *               | term
     *     term     -> "\\" <any> postfix
     *               | 𝛼 in Σ postfix
     *               | ε
     *     postfix -> "*"
     *               | ε
     */
    const fn expr(mut self, input: &'static [u8], mut idx: usize) -> (Self, usize) {
        (self, idx) = self.term(input, idx);
        while idx < input.len() {
            let last_idx = idx;

            (self, idx) = self.rest(input, idx);
            if last_idx == idx {
                return (self, idx);
            }
        }

        (self, input.len())
    }

    const fn term(mut self, input: &'static [u8], mut idx: usize) -> (Self, usize) {
        (self, idx) = match input.get(idx) {
            Some(b'\\') => self.escaped_term(input, idx + 1),
            Some(
                chara @ (b'a'..=b'z'
                | b'A'..=b'Z'
                | b'0'..=b'9'
                | b'!'
                | b'@'
                | b'#'
                | b'%'
                | b'&'
                | b'-'
                | b'='
                | b'+'
                | b';'
                | b':'
                | b'"'
                | b','
                | b'<'
                | b'>'
                | b'/'
                | b'`'
                | b'~'
                | b' '
                | b'\''),
            ) => (self.add_alphabet_term(*chara), idx + 1),

            None => return (self, idx),
            _ => return (self.add_empty_term(), idx),
        };

        self.postfix(input, idx)
    }

    const fn postfix(self, input: &'static [u8], idx: usize) -> (Self, usize) {
        if let Some(b'*') = input.get(idx) {
            return (self.kleene_star(), idx + 1);
        }

        (self, idx)
    }

    const fn rest(self, input: &'static [u8], idx: usize) -> (Self, usize) {
        let (mut nfa, mut idx) = (self, idx);

        let last_start_idx = nfa.start_idx;
        let last_accept_idx = nfa.accept_idx;
        match input.get(idx) {
            Some(b'(') => nfa.group(input, idx),
            Some(b'|') => nfa.alternate(input, idx),
            _ => {
                (nfa, idx) = nfa.term(input, idx);
                (nfa.product(last_start_idx, last_accept_idx), idx)
            }
        }
    }

    const fn escaped_term(self, input: &'static [u8], idx: usize) -> (Self, usize) {
        match input.get(idx).copied() {
            Some(b'n') => (self.add_alphabet_term(b'\n'), idx + 1),
            Some(b't') => (self.add_alphabet_term(b'\t'), idx + 1),

            Some(
                chara @ (b'$' | b'^' | b'(' | b')' | b'{' | b'}' | b'[' | b']' | b'|' | b'?' | b'*'
                | b'\\'),
            ) => (self.add_alphabet_term(chara), idx + 1),

            None => panic!("unexpected end of input: expected escaped character"),
            _ => panic!("unexpected escaped character value"),
        }
    }

    // Rule 1: For ε, construct an NFA where "i" is a new start state and "f" is a new accepting
    // state. This NFA recognizes the empty string, ε.
    //
    //                 +---+  ε   +===+
    //     start ----> | i | ---> ‖ f ‖
    //                 +---+      +===+
    //
    const fn add_empty_term(self) -> Self {
        self.add_term(None)
    }

    // Rule 2: For a in Σ, construct an NFA where "i" is a new start state and "f" is a new accepting
    // state. This NFA recognizes the character represented by "a".
    //
    //                 +---+  a   +===+
    //     start ----> | i | ---> ‖ f ‖
    //                 +---+      +===+
    //
    const fn add_alphabet_term(self, chara: u8) -> Self {
        self.add_term(Some(chara))
    }

    const fn add_term(mut self, chara: Option<u8>) -> Self {
        // create two states: i and f; link them
        self.start_idx = self.state_count;
        self.accept_idx = self.state_count + 1;
        self.states[self.start_idx as usize] =
            self.states[self.start_idx as usize].add_transition(chara, self.accept_idx);
        self.state_count += 2;
        self
    }

    // Rule 3.a: for the regular expression s|t, construct an NFA:
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
    const fn alternate(mut self, input: &'static [u8], mut idx: usize) -> (Self, usize) {
        let prev_start_idx = self.start_idx;
        let prev_accept_idx = self.accept_idx;

        self.start_idx = self.state_count;
        self.accept_idx = self.state_count;
        self.state_count += 2;

        (self, idx) = self.expr(input, idx + 1);

        // 1. alloc two new states: i & f
        // 2. add two transitions from i on ε; one to prev_start_idx, and one to start_idx
        // 3. add transition from prev_accept to f
        // 4. add transition from self.accept_idx to f
        // 5. set self.start_idx to i
        // 6. set self.accept_idx to f
        let i_idx = self.state_count;
        let f_idx = self.state_count + 1;
        self.state_count += 2;

        self.states[i_idx as usize] =
            self.states[i_idx as usize].add_transition(None, prev_start_idx);
        self.states[i_idx as usize] =
            self.states[i_idx as usize].add_transition(None, self.start_idx);

        self.states[prev_accept_idx as usize] =
            self.states[prev_accept_idx as usize].add_transition(None, f_idx);
        self.states[self.accept_idx as usize] =
            self.states[self.accept_idx as usize].add_transition(None, f_idx);

        self.start_idx = i_idx;
        self.accept_idx = f_idx;

        (self, idx)
    }

    // 3.b: for the regular expression "st", construct an NFA:
    //
    //                 +-------+------+
    //     start ----> Ⓘ  N(s) ○ N(t) Ⓕ
    //                 +-------+------+
    //
    const fn product(mut self, last_start_idx: NFASize, last_accept_idx: NFASize) -> Self {
        // take all transitions out of start(N(t)) and add them to accept(N(s))
        // remove all transitions out of start(N(t))

        let mut idx = 0;
        while idx < self.states[self.start_idx as usize].transition_count {
            self.states[last_accept_idx as usize].transitions
                [self.states[last_accept_idx as usize].transition_count as usize] =
                self.states[self.start_idx as usize].transitions[idx as usize];
            self.states[last_accept_idx as usize].transition_count += 1;
            idx += 1;
        }
        self.states[self.start_idx as usize].transition_count = 0;
        self.start_idx = last_start_idx;
        self
    }

    // Rule 3.c: for the regular expression s*, construct an NFA:
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

        self.states[i_idx as usize] =
            self.states[i_idx as usize].add_transition(None, self.start_idx);
        self.states[i_idx as usize] = self.states[i_idx as usize].add_transition(None, f_idx);

        self.states[self.accept_idx as usize] =
            self.states[self.accept_idx as usize].add_transition(None, self.start_idx);
        self.states[self.accept_idx as usize] =
            self.states[self.accept_idx as usize].add_transition(None, f_idx);

        self.start_idx = i_idx;
        self.accept_idx = f_idx;

        self
    }

    // Rule 3.d: for the regular expression (s), construct NFA(s), consuming the left and right parens.
    const fn group(mut self, input: &'static [u8], mut idx: usize) -> (Self, usize) {
        if input[idx] != b'(' {
            panic!("expected '('");
        }

        let last_start_idx = self.start_idx;
        let last_accept_idx = self.accept_idx;
        (self, idx) = self.expr(input, idx + 1);

        if let Some(b')') = input.get(idx) {
            (self, idx) = self.postfix(input, idx + 1);
            return (self.product(last_start_idx, last_accept_idx), idx);
        }

        panic!("unterminated group, expected ')'");
    }

    pub(crate) fn debug_print(&self, prefix: &'static [u8]) {
        use crate::io::{itoa, puts};

        puts("subgraph ");
        puts(prefix);
        puts(" {\n");
        puts("  label = \"");
        puts(prefix);
        puts("\";\n");
        puts("  rankdir=\"LR\";\n");
        puts("  ");
        puts(prefix);
        puts(itoa(self.start_idx as u32));
        puts(" [shape=box];\n");
        puts("  ");
        puts(prefix);
        puts(itoa(self.accept_idx as u32));
        puts(" [shape=doublecircle];\n");
        for (idx, state) in self.states[0..self.state_count as usize].iter().enumerate() {
            for transition in &state.transitions[0..state.transition_count as usize] {
                puts("  ");
                puts(prefix);
                puts(itoa(transition.to_state_idx as u32));
                puts("[label=\"S");
                puts(itoa(transition.to_state_idx as u32));
                puts("\"];\n");
                puts("  ");
                puts(prefix);
                puts(itoa(idx as u32));
                puts(" -> ");
                puts(prefix);
                puts(itoa(transition.to_state_idx as u32));

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
}

#[allow(dead_code)]
fn dbgnfa<const N: usize>(prefix: &[u8], nfa: &Nfa<N>) {
    use crate::io::{eputs, flush, itoa};
    eputs("\x1b[33m");
    eputs(prefix);
    eputs("\x1b[0m: NFA<");
    eputs(itoa(N as u32));
    eputs("> { start_idx: ");
    eputs(itoa(nfa.start_idx as u32));
    eputs(", accept_idx: ");
    eputs(itoa(nfa.accept_idx as u32));
    eputs(", state_count: ");
    eputs(itoa(nfa.state_count as u32));
    eputs("}\n");

    for (idx, state) in nfa.states[0..nfa.state_count as usize].iter().enumerate() {
        eputs("  ");
        eputs(if idx == nfa.start_idx as usize {
            "^ "
        } else if idx == nfa.accept_idx as usize {
            "$ "
        } else {
            "- "
        });
        eputs(itoa(idx as u32));
        eputs(": {");
        for transition in &state.transitions[0..state.transition_count as usize] {
            if let Some(xs) = transition.on_character {
                eputs([xs]);
            } else {
                eputs("ε");
            }
            eputs("→");
            eputs(itoa(transition.to_state_idx as u32));
            eputs(", ");
        }
        eputs("}\n");
    }
    flush();
}

#[allow(dead_code)]
fn error_input_progress(input: &'static [u8], idx: usize) {
    use crate::io::{eputs, flush};
    eputs(input);
    eputs("\n");
    if idx > 0 {
        for _ in 0..idx {
            eputs("~");
        }
    }
    eputs("^\n");
    flush();
}

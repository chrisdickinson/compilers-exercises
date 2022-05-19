

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
    /*const*/fn add_transition(mut self, on_character: Option<u8>, to_state_idx: usize) -> Self {
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

    pub(crate) /*const*/fn from_regex_bytes(input: &'static [u8]) -> Self {
        // I sure wish we could use Default::default() in const functions.
        let nfa = Self {
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

        let (nfa, idx) = nfa.expr(input, 0);
        if idx != input.len() {
            error_input_progress(input, idx);
            panic!("unexpected character");
        }

        nfa
    }

    /*const*/fn expr(self, input: &'static [u8], mut idx: usize) -> (Self, usize) {
        /* Language:
         *
         *     expr ::= ε
         *        | 𝛼 in Σ
         *        | expr "|" expr
         *        | expr expr
         *        | expr "*"
         *        | "(" expr ")"
         *
         * Rewritten to avoid left recursion:
         *
         *     expr -> term rest
         *     rest -> "|" expr
         *           | "*" expr
         *           | "(" expr ")"
         *           | "\\" term
         *           | term
         *     term -> "\\" <any>
         *           | 𝛼 in Σ
         *           | ε
         */
        if idx == input.len() {
            return (self, idx)
        }

        let mut nfa = self;

        while idx < input.len() {
            let last_idx = idx;
            (nfa, idx) = nfa.term(input, idx);
            (nfa, idx) = nfa.rest(input, idx);
            if last_idx == idx {
                return (nfa, idx)
            }
        }

        (nfa, input.len())
    }

    /*const*/fn rest(self, input: &'static [u8], idx: usize) -> (Self, usize) {
        if idx >= input.len() {
            return (self, idx)
        }

        match input[idx] {
            b'|' => self.alternate(input, idx),
            b'*' => self.kleene_star(input, idx),
            b'(' => self.group(input, idx),
            b'\\' => self.escaped_term(input, idx + 1),

            // 3.b: for the regular expression "st", construct an NFA:
            //
            //                 +-------+------+
            //     start ----> Ⓘ  N(s) ○ N(t) Ⓕ
            //                 +-------+------+
            //
            _ => return (self, idx)
        }
    }

    /*const*/fn term(self, input: &'static [u8], idx: usize) -> (Self, usize) {
        if idx >= input.len() {
            return (self.add_empty_term(), idx)
        }

        match input[idx] {
            // Positively match the characters we expect from Σ.
            chara @ (
                b'a'..=b'z' |
                b'A'..=b'Z' |
                b'0'..=b'9' |
                b'!' | b'@' |
                b'#' | b'%' |
                b'&' | b'-' |
                b'=' | b'+' |
                b';' | b':' |
                b'"' | b',' |
                b'<' | b'>' |
                b'/' | b'`' |
                b'~' | b' ' |
                b'\''

            ) => (self.add_alphabet_term(chara), idx + 1),

            _ => (self.add_empty_term(), idx)
        }
    }

    /*const*/fn escaped_term(self, input: &'static [u8], idx: usize) -> (Self, usize) {
        if idx >= input.len() {
            panic!("unexpected end of input: expected escaped character");
        }

        match input[idx] {
            b'n' => (self.add_alphabet_term(b'\n'), idx + 1),
            b't' => (self.add_alphabet_term(b'\t'), idx + 1),

            chara @ (
                b'$' | b'^' |
                b'(' | b')' |
                b'{' | b'}' |
                b'[' | b']' |
                b'|' | b'?' |
                b'*' | 
                b'\\'
            ) => (self.add_alphabet_term(chara), idx + 1),

            _ => panic!("unexpected escaped character value")
        }
    }

    // Rule 1: For ε, construct an NFA where "i" is a new start state and "f" is a new accepting
    // state. This NFA recognizes the empty string, ε.
    // 
    //                 +---+  ε   +===+
    //     start ----> | i | ---> ‖ f ‖
    //                 +---+      +===+
    //
    /*const*/fn add_empty_term(self) -> Self {
        self.add_term(None)
    }

    // Rule 2: For a in Σ, construct an NFA where "i" is a new start state and "f" is a new accepting
    // state. This NFA recognizes the character represented by "a".
    // 
    //                 +---+  a   +===+
    //     start ----> | i | ---> ‖ f ‖
    //                 +---+      +===+
    //
    /*const*/fn add_alphabet_term(self, chara: u8) -> Self {
        self.add_term(Some(chara))
    }

    /*const*/fn add_term(mut self, chara: Option<u8>) -> Self {
        self.states[self.accept_idx] = self.states[self.accept_idx].add_transition(chara, self.state_count);
        self.accept_idx = self.state_count;
        self.state_count += 1;
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
    /*const*/fn alternate(mut self, input: &'static [u8], idx: usize) -> (Self, usize) {
        let prev_start_idx = self.start_idx;
        let prev_accept_idx = self.accept_idx;

        self.start_idx = self.state_count;
        self.accept_idx = self.state_count;
        self.state_count += 1;

        let (mut nfa, idx) = self.expr(input, idx + 1);

        // 1. alloc two new states: i & f
        // 2. add two transitions from i on ε; one to prev_start_idx, and one to start_idx
        // 3. add transition from prev_accept to f
        // 4. add transition from nfa.accept_idx to f
        // 5. set nfa.start_idx to i
        // 6. set nfa.accept_idx to f
        let i_idx = nfa.state_count;
        let f_idx = nfa.state_count + 1;
        nfa.state_count += 2;

        nfa.states[i_idx] = nfa.states[i_idx].add_transition(None, prev_start_idx);
        nfa.states[i_idx] = nfa.states[i_idx].add_transition(None, nfa.start_idx);

        nfa.states[prev_accept_idx] = nfa.states[prev_accept_idx].add_transition(None, f_idx);
        nfa.states[nfa.accept_idx] = nfa.states[nfa.accept_idx].add_transition(None, f_idx);

        nfa.start_idx = i_idx;
        nfa.accept_idx = f_idx;

        (nfa, idx)
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
    /*const*/fn kleene_star(mut self, input: &'static [u8], idx: usize) -> (Self, usize) {
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

        self.expr(input, idx + 1)
    }

    // Rule 3.d: for the regular expression (s), construct NFA(s), consuming the left and right parens.
    /*const*/fn group(self, input: &'static [u8], idx: usize) -> (Self, usize) {
        if input[idx] != b'(' {
            panic!("expected '('");
        }

        let (nfa, idx) = self.expr(input, idx + 1);

        if idx >= input.len() {
            panic!("unexpected end of input: expected ')'");
        }

        if input[idx] != b')' {
            panic!("unterminated group, expected ')'");
        }

        (nfa, idx + 1)
    }
}

fn error_input_progress(input: &'static [u8], idx: usize) {
    use crate::io::{itoa, eputs};
    eputs(input);
    eputs("\n");
    if idx > 0 {
        for i in 0..idx {
            eputs("~");
        }
    }
    eputs("^\n");
}




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
    pub(crate) fn debug_print(&self, prefix: &'static [u8]) {
        use crate::io::{ itoa, puts };

        puts("subgraph "); puts(prefix); puts(" {\n");
        puts("  label = \""); puts(prefix); puts("\";\n");
        puts("  rankdir=\"LR\";\n");
        puts("  "); puts(prefix); puts(itoa(self.start_idx as u32)); puts(" [shape=box];\n");
        puts("  "); puts(prefix); puts(itoa(self.accept_idx as u32)); puts(" [shape=doublecircle];\n");
        for (idx, state) in self.states[0..self.state_count].iter().enumerate() {
            for transition in &state.transitions[0..state.transition_count] {

                puts("  "); puts(prefix); puts(itoa(transition.to_state_idx as u32)); puts("[label=\"S"); puts(itoa(transition.to_state_idx as u32)); puts("\"];\n");
                puts("  "); puts(prefix); puts(itoa(idx as u32)); puts(" -> ");  puts(prefix); puts(itoa(transition.to_state_idx as u32));

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
            state_count: 0,
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

    /*const*/fn expr(self, input: &'static [u8], idx: usize) -> (Self, usize) {
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

        // what do we do with kleene_star?
        // it must: bind to terminal characters more tightly than the product rule
        //
        dbgnfa(b"expr", &self);
        let (mut nfa, mut idx) = self.term(input, idx);

        while idx < input.len() {
            let last_idx = idx;

            (nfa, idx) = nfa.rest(input, idx);
            if last_idx == idx {
                return (nfa, idx)
            }
        }

        (nfa, input.len())
    }

    /*const*/fn term(self, input: &'static [u8], idx: usize) -> (Self, usize) {
        dbgnfa(b"term", &self);
        let (nfa, idx) = match input.get(idx) {
            None => return (self, idx),

            Some(b'\\') => self.escaped_term(input, idx + 1),
            Some(chara) if is_term_char(chara) => (self.add_alphabet_term(*chara), idx + 1),
            _ => (self.add_empty_term(), idx),
        };

        if let Some(b'*') = input.get(idx) {
            return (nfa.kleene_star(), idx + 1)
        }

        (nfa, idx)
    }

    /*const*/fn rest(self, input: &'static [u8], idx: usize) -> (Self, usize) {
        dbgnfa(b"rest", &self);
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

    /*const*/fn escaped_term(self, input: &'static [u8], idx: usize) -> (Self, usize) {
        match input.get(idx).copied() {
            Some(b'n') => (self.add_alphabet_term(b'\n'), idx + 1),
            Some(b't') => (self.add_alphabet_term(b'\t'), idx + 1),

            Some(chara @ (
                b'$' | b'^' |
                b'(' | b')' |
                b'{' | b'}' |
                b'[' | b']' |
                b'|' | b'?' |
                b'*' | 
                b'\\'
            )) => (self.add_alphabet_term(chara), idx + 1),

            None => panic!("unexpected end of input: expected escaped character"),
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
        // create two states: i and f; link them
        self.start_idx = self.state_count;
        self.accept_idx = self.state_count + 1;
        self.states[self.start_idx] = self.states[self.start_idx].add_transition(chara, self.accept_idx);
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
    /*const*/fn alternate(mut self, input: &'static [u8], idx: usize) -> (Self, usize) {
        let prev_start_idx = self.start_idx;
        let prev_accept_idx = self.accept_idx;

        self.start_idx = self.state_count;
        self.accept_idx = self.state_count;
        self.state_count += 2;

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

    // 3.b: for the regular expression "st", construct an NFA:
    //
    //                 +-------+------+
    //     start ----> Ⓘ  N(s) ○ N(t) Ⓕ
    //                 +-------+------+
    //
    /*const*/fn product(mut self, last_start_idx: usize, last_accept_idx: usize) -> Self {

        // take all transitions out of start(N(t)) and add them to accept(N(s))
        // remove all transitions out of start(N(t))

        for i in 0..self.states[self.start_idx].transition_count {
            self.states[last_accept_idx].transitions[self.states[last_accept_idx].transition_count] = self.states[self.start_idx].transitions[i];
            self.states[last_accept_idx].transition_count += 1;
        }
        self.states[self.start_idx].transition_count = 0;
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
    /*const*/fn kleene_star(mut self) -> Self {
        dbgnfa(b"kleene_star", &self);
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

    // Rule 3.d: for the regular expression (s), construct NFA(s), consuming the left and right parens.
    /*const*/fn group(self, input: &'static [u8], idx: usize) -> (Self, usize) {
        if input[idx] != b'(' {
            panic!("expected '('");
        }

        let (nfa, idx) = self.expr(input, idx + 1);

        if let Some(b')') = input.get(idx) {
            return (nfa, idx + 1)
        }

        panic!("unterminated group, expected ')'");
    }
}

/*const*/fn is_term_char(chara: &u8) -> bool {
    match chara {
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
        ) => true,

        _ => false
    }
}

fn dbgnfa<const N: usize>(prefix: &[u8], nfa: &NFA<N>) {
    use crate::io::{itoa, eputs};
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


    for (idx, state) in nfa.states[0..nfa.state_count].iter().enumerate() {
        eputs("  ");
        eputs(if idx == nfa.start_idx {
            "^"
        } else if idx == nfa.accept_idx {
            "$"
        } else {
            "-"
        });
        eputs(" S"); eputs(itoa(idx as u32)); eputs(": {");
        for transition in &state.transitions[0..state.transition_count] {
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


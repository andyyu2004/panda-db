use clap::Parser;
use rustyline::completion::{Candidate, Completer};
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hint, Hinter};
use rustyline::validate::Validator;
use rustyline::{Editor, Helper};

#[derive(Parser)]
struct Opts {}

fn main() -> anyhow::Result<()> {
    let _opts = Opts::parse();

    let mut rl = Editor::<Autocomplete>::new();
    loop {
        let line = rl.readline("panda> ")?;
        println!("{}", line);
    }
}

struct Autocomplete;

impl Helper for Autocomplete {
}

struct PandaCandidate {}

impl Candidate for PandaCandidate {
    fn display(&self) -> &str {
        ""
    }

    fn replacement(&self) -> &str {
        ""
    }
}

impl Completer for Autocomplete {
    type Candidate = PandaCandidate;
}

impl Validator for Autocomplete {
}

impl Highlighter for Autocomplete {
}

impl Hinter for Autocomplete {
    type Hint = PandaHint;
}

struct PandaHint {}

impl Hint for PandaHint {
    fn display(&self) -> &str {
        ""
    }

    fn completion(&self) -> Option<&str> {
        None
    }
}

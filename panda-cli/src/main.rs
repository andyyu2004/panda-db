mod cli;

use std::future;

use clap::Parser;
use panda_sql::PandaEngine;
use rustyline::completion::{Candidate, Completer};
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hint, Hinter};
use rustyline::validate::Validator;
use rustyline::{Editor, Helper};
use tokio_postgres::NoTls;

use crate::cli::{Cmd, Opts};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let opts = Opts::parse();
    match opts.cmd {
        Cmd::Start(opts) => {
            let _engine =
                PandaEngine::new(opts.conn.listen_addr, opts.conn.pg_addr, opts.join).await?;
            future::pending::<()>().await;
        }
        Cmd::StartSingleNode(opts) => {
            let _engine =
                PandaEngine::new(opts.conn.listen_addr, opts.conn.pg_addr, vec![]).await?;
            future::pending::<()>().await;
        }
        Cmd::Shell => {
            let (client, connection) =
                tokio_postgres::connect("host=localhost port=26630", NoTls).await?;
            tokio::spawn(async move {
                if let Err(err) = connection.await {
                    eprintln!("connection error: {}", err);
                }
            });
            let mut rl = Editor::<Autocomplete>::new();
            loop {
                let line = rl.readline("panda> ")?;
                client.query(&line, &[]).await?;
                println!("{}", line);
            }
        }
    }
    Ok(())
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

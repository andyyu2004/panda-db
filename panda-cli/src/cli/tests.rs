use super::*;
use clap::Parser;
use panda_sql::PandaResult;

macro_rules! parse {
    ($input:expr) => {{ Opts::try_parse_from(shellwords::split($input)?) }};
}

#[test]
fn test_parse_start_cli_args() -> PandaResult<()> {
    let opts = parse!(
        "panda start --listen-addr=localhost:1234 --pg-addr=localhost:1235 --join localhost:2255,localhost:33445"
    )?;
    assert_eq!(
        opts.cmd,
        Cmd::Start(CmdStart {
            conn: ConnConfig {
                listen_addr: "localhost:1234".to_owned(),
                pg_addr: "localhost:1235".to_owned()
            },
            join: vec!["localhost:2255".to_owned(), "localhost:33445".to_owned()],
        })
    );
    Ok(())
}

#[test]
fn test_parse_start_cli_args_missing_join_addr() -> PandaResult<()> {
    parse!("panda start --listen-addr=localhost:1234").unwrap_err();
    Ok(())
}

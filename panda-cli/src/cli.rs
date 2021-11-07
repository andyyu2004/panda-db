use clap::Parser;

#[derive(Debug, Parser)]
pub struct Opts {
    #[clap(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, PartialEq, Parser)]
pub enum Cmd {
    Start(CmdStart),
    StartSingleNode(CmdStartSingleNode),
    Shell,
}

#[derive(Debug, PartialEq, Parser)]
pub struct CmdStart {
    #[clap(flatten)]
    pub conn: ConnConfig,
    /// Comma separated list of node addresses to connect to
    #[clap(long, required = true, multiple_values = true, value_delimiter = ',', min_values = 1)]
    pub join: Vec<String>,
}

#[derive(Debug, PartialEq, Parser)]
pub struct ConnConfig {
    #[clap(long, default_value = panda_sql::DEFAULT_LISTEN_ADDR)]
    pub listen_addr: String,
    #[clap(long, default_value = panda_sql::DEFAULT_PG_ADDR)]
    pub pg_addr: String,
}

#[derive(Debug, PartialEq, Parser)]
pub struct CmdStartSingleNode {
    #[clap(flatten)]
    pub conn: ConnConfig,
}

#[cfg(test)]
mod tests;

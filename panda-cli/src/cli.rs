use clap::Parser;

#[derive(Debug, Parser)]
pub struct Opts {
    #[clap(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, PartialEq, Parser)]
pub enum Cmd {
    Start(CmdStart),
    Shell,
}

#[derive(Debug, PartialEq, Parser)]
pub struct CmdStart {
    #[clap(long)]
    pub listen_addr: String,
    /// Comma separated list of node addresses to connect to
    #[clap(long, required = true, multiple_values = true, value_delimiter = ',', min_values = 1)]
    pub join: Vec<String>,
}

#[cfg(test)]
mod tests;

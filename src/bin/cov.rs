use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Clone, Debug, Eq, PartialEq, StructOpt)]
pub struct Opts {
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Clone, Debug, Eq, PartialEq, StructOpt)]
pub enum Command {
    Show {
        #[structopt(flatten)]
        show: ShowCommand,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, StructOpt)]
pub struct ShowCommand {
    /// File with the profile data obtained after an instrumented run
    #[structopt(long = "instr-profile")]
    instr_profile: PathBuf,
    /// Coverage executable or object file
    #[structopt(long = "object")]
    object: Vec<PathBuf>,
}

impl ShowCommand {
    fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        todo!();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::from_args();
    match opts.cmd {
        Command::Show { show } => show.run(),
    }
}

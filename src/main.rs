use llvm_profparser::parse;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Clone, Debug, Eq, PartialEq, StructOpt)]
pub enum Command {
    Show {
        /// Input profraw file to show some information about
        #[structopt(name = "input", long = "input", short = "i")]
        input: PathBuf,
    },
    Merge,
    Overlap,
}

#[derive(Clone, Debug, Eq, PartialEq, StructOpt)]
pub struct Opts {
    #[structopt(subcommand)]
    cmd: Command,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::from_args();
    match opts.cmd {
        Command::Show { input } => {
            let profile = parse(&input)?;
            println!("Instrumentation level: ?");
            println!("Total functions: ?");
            println!("Maximum function count: ?");
            println!("Maximum internal block count: ?");
        }
        _ => {
            panic!("Unsupported command");
        }
    }
    Ok(())
}

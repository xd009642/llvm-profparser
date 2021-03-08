use llvm_profparser::parse;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Clone, Debug, Eq, PartialEq, StructOpt)]
pub enum Command {
    Show {
        #[structopt(flatten)]
        show: ShowCommand,
    },
    Merge,
    Overlap,
}

#[derive(Clone, Debug, Eq, PartialEq, StructOpt)]
pub struct ShowCommand {
    /// Input profraw file to show some information about
    #[structopt(name = "input", long = "input", short = "i")]
    input: PathBuf,
    /// Details for every function
    #[structopt(long = "all-functions")]
    all_functions: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, StructOpt)]
pub struct Opts {
    #[structopt(subcommand)]
    cmd: Command,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::from_args();
    match opts.cmd {
        Command::Show { show } => {
            let profile = parse(&show.input)?;
            println!("Version: {}", profile.version());
            let is_ir_instr = profile.is_ir_level_profile();
            let mut shown_funcs = 0;
            if show.all_functions {
                for func in &profile.records {
                    if func.name.is_none() || func.hash.is_none() {
                        continue;
                    }
                    shown_funcs += 1;
                    println!("  {}:", func.name.as_ref().unwrap());
                    println!("    Hash: {}", func.hash.unwrap());
                    println!("    Counters: {}", func.record.counts.len());
                    if !is_ir_instr {
                        println!("    Function Count: {}", func.record.counts[0]);
                    }
                }
            }
            println!("Instrumentation level: ?");
            if show.all_functions {
                println!("Functions shown: {}", shown_funcs);
            }
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

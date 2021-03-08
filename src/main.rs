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
    /// Show instr profile data in text dump format
    #[structopt(long = "text")]
    text: bool,
    /// Show indirect call site target values for shown functions
    #[structopt(long = "ic-targets")]
    ic_targets: bool,
    /// Show the profiled sizes of the memory intrinsic call for shown functions"
    #[structopt(long = "memop-sizes")]
    memop_sizes: bool,
    /// Show detailed profile summary
    #[structopt(long = "show_detailed_summary")]
    show_detailed_summary: bool,
    /// Cutoff percentages (times 10000) for generating detailed summary
    #[structopt(long = "detailed_summary_cutoffs")]
    detailed_summary_cutoffs: Vec<usize>,
    /// Show profile summary of a list of hot functions
    #[structopt(long = "show_hot_fn_list")]
    show_hot_fn_list: bool,
    /// Show context sensitive counts
    #[structopt(long = "showcs")]
    showcs: bool,
    /// Details for matching functions
    #[structopt(long = "function")]
    function: String,
    /// Output file
    #[structopt(long = "output", short = "o")]
    output: String,
    /// Show the list of functions with the largest internal counts
    #[structopt(long = "topn")]
    topn: Option<usize>,
    /// Set the count value cutoff. Functions with the maximum count less than
    /// this value will not be printed out. (Default is 0)
    #[structopt(long = "value_cutoff")]
    value_cutoff: Option<usize>,
    /// Set the count value cutoff. Functions with the maximum count below the
    /// cutoff value
    #[structopt(long = "only_list_below")]
    only_list_below: bool,
    /// Show profile symbol list if it exists in the profile.
    #[structopt(long = "show_profile_sym_list")]
    show_profile_sym_list: bool,
    /// Show the information of each section in the sample profile. The flag is
    /// only usable when the sample profile is in extbinary format
    #[structopt(long = "show_section_info_only")]
    show_section_info_only: bool,
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
            println!("Instrumentation level: {}", profile.get_level());
            if show.all_functions {
                println!("Functions shown: {}", shown_funcs);
            }
            println!("Total functions: {}", profile.symtab.len());
            println!("Maximum function count: ?");
            println!("Maximum internal block count: ?");
        }
        _ => {
            panic!("Unsupported command");
        }
    }
    Ok(())
}

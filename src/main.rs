use llvm_profparser::instrumentation_profile::stats::*;
use llvm_profparser::instrumentation_profile::summary::*;
use llvm_profparser::instrumentation_profile::types::*;
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
    /// Show counter values for shown functions
    #[structopt(long = "counts")]
    show_counts: bool,
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
    function: Option<String>,
    /// Output file
    #[structopt(long = "output", short = "o")]
    output: Option<String>,
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

impl ShowCommand {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let profile = parse(&self.input)?;
        let mut summary = ProfileSummary::new();
        let mut stats = vec![ValueSiteStats::default(); ValueKind::len()];

        let is_ir_instr = profile.is_ir_level_profile();
        let mut shown_funcs = 0;
        println!("Counters:");
        if self.all_functions {
            for func in &profile.records {
                if func.name.is_none() || func.hash.is_none() {
                    continue;
                }
                summary.add_record(&func.record);
                shown_funcs += 1;
                println!("  {}:", func.name.as_ref().unwrap());
                println!("    Hash: 0x{:x}", func.hash.unwrap());
                println!("    Counters: {}", func.record.counts.len());
                if !is_ir_instr {
                    println!("    Function Count: {}", func.record.counts[0]);
                }
                if self.ic_targets {
                    println!(
                        "    Indirect Call Site Count: {}",
                        func.num_value_sites(ValueKind::IndirectCallTarget)
                    );
                    stats[ValueKind::IndirectCallTarget as usize].traverse_sites(
                        &func.record,
                        ValueKind::IndirectCallTarget,
                        Some(&profile.symtab),
                    );
                }
                let num_memop_calls = func.num_value_sites(ValueKind::MemOpSize);
                if self.memop_sizes && num_memop_calls > 0 {
                    println!("    Number of Memory Intrinsics Calls: {}", num_memop_calls);
                    stats[ValueKind::MemOpSize as usize].traverse_sites(
                        &func.record,
                        ValueKind::MemOpSize,
                        None,
                    );
                }
                if self.show_counts {
                    let start = if is_ir_instr { 0 } else { 1 };
                    let counts = func
                        .counts()
                        .iter()
                        .skip(start)
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>()
                        .join(",");
                    println!("    Block counts: [{}]", counts);
                }
                if self.ic_targets {
                    println!("    Indirect Target Results:");
                }
            }
        }
        println!("Instrumentation level: {}", profile.get_level());
        if self.all_functions {
            println!("Functions shown: {}", shown_funcs);
        }
        println!("Total functions: {}", summary.num_functions());
        println!("Maximum function count: {}", summary.max_function_count());
        println!(
            "Maximum internal block count: {}",
            summary.max_internal_block_count()
        );
        if let Some(_topn) = self.topn {}

        if self.ic_targets && shown_funcs > 0 {
            println!("Statistics for indirect call sites profile:");
            println!("{}", stats[ValueKind::IndirectCallTarget as usize]);
        }

        if self.memop_sizes && shown_funcs > 0 {
            println!("Statistics for memory instrinsic calls sizes profile:");
            println!("{}", stats[ValueKind::MemOpSize as usize]);
        }

        if self.show_detailed_summary {
            println!("Total number of blocks: ?");
            println!("Total count: ?");
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::from_args();
    match opts.cmd {
        Command::Show { show } => show.run(),
        _ => {
            panic!("Unsupported command");
        }
    }
}

use llvm_profparser::instrumentation_profile::stats::*;
use llvm_profparser::instrumentation_profile::summary::*;
use llvm_profparser::instrumentation_profile::types::*;
use llvm_profparser::parse;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Clone, Debug, Eq, PartialEq, StructOpt)]
pub enum Command {
    Show {
        #[structopt(flatten)]
        show: ShowCommand,
    },
    Merge {
        #[structopt(flatten)]
        merge: MergeCommand,
    },
    Overlap {
        #[structopt(flatten)]
        overlap: OverlapCommand,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, StructOpt)]
pub struct ShowCommand {
    /// Input profraw file to show some information about
    #[structopt(name = "<filename...>", long = "input", short = "i")]
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
    #[structopt(long = "value_cutoff", default_value = "0")]
    value_cutoff: u64,
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
pub struct MergeCommand {
    /// Input files to merge
    #[structopt(name = "<filename...>", long = "input", short = "i")]
    input: Vec<PathBuf>,
    /// List of weights and filenames in `<weight>,<filename>` format
    #[structopt(long = "weighted-input", parse(try_from_str=try_parse_weighted))]
    weighted_input: Vec<(u64, PathBuf)>,
    /// Number of merge threads to use (will autodetect by default)
    #[structopt(long = "num-threads", short = "j")]
    jobs: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, StructOpt)]
pub struct OverlapCommand {
    #[structopt(name = "<base profile file>")]
    base_file: PathBuf,
    #[structopt(name = "<test profile file>")]
    test_file: PathBuf,
    #[structopt(long = "output", short = "o")]
    output: Option<PathBuf>,
    /// For context sensitive counts
    #[structopt(long = "cs")]
    context_sensitive_counts: bool,
    /// Function level overlap information for every function in test profile with max count value
    /// greater than the parameter value
    #[structopt(long = "value-cutoff")]
    value_cutoff: Option<usize>,
    /// Function level overlap information for matching functions
    #[structopt(long = "function")]
    function: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, StructOpt)]
pub struct Opts {
    #[structopt(subcommand)]
    cmd: Command,
}

fn try_parse_weighted(input: &str) -> Result<(u64, PathBuf), String> {
    todo!()
}

fn check_function(name: Option<&String>, pattern: Option<&String>) -> bool {
    match pattern {
        Some(pat) => name.map(|x| x.contains(pat)).unwrap_or(false),
        None => false,
    }
}

#[derive(Clone, Debug, Eq)]
struct HotFn {
    name: String,
    count: u64,
}

impl PartialOrd for HotFn {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HotFn {
    fn cmp(&self, other: &Self) -> Ordering {
        // Do the reverse here
        other.count.cmp(&self.count)
    }
}

impl PartialEq for HotFn {
    fn eq(&self, other: &Self) -> bool {
        self.count == other.count
    }
}

impl ShowCommand {
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let profile = parse(&self.input)?;
        let mut summary = ProfileSummary::new();
        let mut stats = vec![ValueSiteStats::default(); ValueKind::len()];

        let is_ir_instr = profile.is_ir_level_profile();
        let mut hotties =
            BinaryHeap::<HotFn>::with_capacity(self.topn.unwrap_or_default() as usize);
        let mut shown_funcs = 0;
        let mut below_cutoff_funcs = 0;
        let topn = self.topn.unwrap_or_default();
        for func in &profile.records {
            if func.name.is_none() || func.hash.is_none() {
                continue;
            }
            if is_ir_instr && func.has_cs_flag() != self.showcs {
                continue;
            }
            let show =
                self.all_functions || check_function(func.name.as_ref(), self.function.as_ref());

            if show && self.text {
                // TODO text format dump
                continue;
            }
            summary.add_record(&func.record);

            let (func_max, func_sum) = func
                .counts()
                .iter()
                .fold((0, 0), |acc, x| (*x.max(&acc.0), acc.1 + x));
            if func_max < self.value_cutoff {
                below_cutoff_funcs += 1;
                if self.only_list_below {
                    println!(
                        "  {}: (Max = {} Sum = {})",
                        func.name.as_ref().unwrap(),
                        func_max,
                        func_sum
                    );
                    continue;
                }
            } else if self.only_list_below {
                continue;
            }
            if topn > 0 {
                if hotties.len() == topn {
                    let top = hotties.peek().unwrap();
                    if top.count < func_max {
                        hotties.pop();
                        hotties.push(HotFn {
                            name: func.name.as_ref().unwrap().to_string(),
                            count: func_max,
                        });
                    }
                } else {
                    hotties.push(HotFn {
                        name: func.name.as_ref().unwrap().to_string(),
                        count: func_max,
                    });
                }
            }
            if show {
                if shown_funcs == 0 {
                    println!("Counters:");
                }
                shown_funcs += 1;
                println!("  {}:", func.name.as_ref().unwrap());
                println!("    Hash: {:#018x}", func.hash.unwrap());
                println!("    Counters: {}", func.counts().len());
                if !is_ir_instr {
                    println!("    Function count: {}", func.counts()[0]);
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
                if self.memop_sizes && num_memop_calls > 0 {
                    println!("    Memory Intrinsic Size Results:");
                }
            }
        }
        println!("Instrumentation level: {}", profile.get_level());
        if self.all_functions || self.function.is_some() {
            println!("Functions shown: {}", shown_funcs);
        }
        println!("Total functions: {}", summary.num_functions());
        if self.value_cutoff > 0 {
            println!(
                "Number of functions with maximum count (< {} ): {}",
                self.value_cutoff, below_cutoff_funcs
            );
            println!(
                "Number of functions with maximum count (>= {}): {}",
                self.value_cutoff,
                summary.num_functions() - below_cutoff_funcs
            );
        }
        println!("Maximum function count: {}", summary.max_function_count());
        println!(
            "Maximum internal block count: {}",
            summary.max_internal_block_count()
        );
        if let Some(topn) = self.topn {
            println!(
                "Top {} functions with the largest internal block counts: ",
                topn
            );
            let hotties = hotties.into_sorted_vec();
            for f in hotties.iter() {
                println!("  {}, max count = {}", f.name, f.count);
            }
        }

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

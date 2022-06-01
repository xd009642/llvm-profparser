use llvm_profparser::*;
use std::fs;
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
    /// File with the profile data obtained after an instrumented run. This differs from llvm-cov
    /// in that if multiple profiles are given it will do the equivalent of a llvm-profdata merge
    /// on them.
    #[structopt(long = "instr-profile")]
    instr_profile: Vec<PathBuf>,
    /// Coverage executable or object file
    #[structopt(long = "object")]
    objects: Vec<PathBuf>,
    /// Pair of paths for a remapping to allow loading files after move. Comma separated in the
    /// order `source,dest`
    #[structopt(long = "path-equivalence")]
    path_remapping: Option<PathRemapping>,
}

impl ShowCommand {
    fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let instr_prof = if self.instr_profile.len() == 1 {
            parse(&self.instr_profile[0])?
        } else if self.instr_profile.len() > 1 {
            merge_profiles(&self.instr_profile)?
        } else {
            panic!("Must provide an instrumentation profile");
        };
        let mapping = CoverageMapping::new(&self.objects, &instr_prof)?;
        let mut report = mapping.generate_report();
        if let Some(remapping) = self.path_remapping.as_ref() {
            report.apply_remapping(remapping);
        }
        println!("Coverage report:");

        for (path, result) in report.files.iter() {
            println!("Processing: {}", path.display());
            // Read file to string
            let source = fs::read_to_string(path)?;
            let column_width = result.max_hits().to_string().len();
            let mut empty = (0..column_width).map(|_| ' ').collect::<String>();
            empty.push('|');
            println!("{}", path.display());
            for (line, source) in source.lines().enumerate() {
                if let Some(hits) = result.hits_for_line(line + 1) {
                    print!("{:1$}|", hits, column_width);
                    println!("{}", source);
                } else {
                    println!("{}{}", empty, source);
                }
            }
            println!("");
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = Opts::from_args();
    match opts.cmd {
        Command::Show { show } => show.run(),
    }
}

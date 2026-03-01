use bad_piggies_level_core::{reader, types, writer};
use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

/// Parser and writer for Bad Piggies binary level files (.bytes)
#[derive(Parser)]
#[command(name = "bad-piggies-level", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Unpack: .bytes → YAML
    Unpack {
        /// Input .bytes level file
        input: PathBuf,
        /// Output YAML file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Pack: YAML → .bytes
    Pack {
        /// Input YAML file
        input: PathBuf,
        /// Output .bytes level file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Unpack { input, output } => {
            let file = File::open(&input)?;
            let mut r = BufReader::new(file);
            let level = reader::read_level(&mut r)?;

            let yaml = serde_yaml::to_string(&level)?;

            if let Some(out_path) = &output {
                std::fs::write(out_path, &yaml)?;
                eprintln!(
                    "Unpacked {} objects → {}",
                    level.object_count,
                    out_path.display()
                );
            } else {
                println!("{}", yaml);
            }
        }
        Commands::Pack { input, output } => {
            let yaml = std::fs::read_to_string(&input)?;
            let level: types::LevelFile = serde_yaml::from_str(&yaml)?;

            let out_path = output.unwrap_or_else(|| input.with_extension("bytes"));
            let file = File::create(&out_path)?;
            let mut w = BufWriter::new(file);
            writer::write_level(&mut w, &level)?;

            eprintln!(
                "Packed {} objects → {}",
                level.objects.len(),
                out_path.display()
            );
        }
    }

    Ok(())
}

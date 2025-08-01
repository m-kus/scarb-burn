mod profiler;

use anyhow::{ensure, Context, Result};
use cairo_lang_runner::Arg;
use cairo_lang_sierra::program::VersionedProgram;
use cairo_lang_utils::bigint::BigUintAsHex;
use camino::Utf8PathBuf;
use clap::{Parser, ValueEnum};
use inferno::flamegraph::{from_lines, Options};
use num_bigint::BigInt;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::process::{Command, ExitCode};
use std::time::SystemTime;
use webbrowser;

use flate2::write::GzEncoder;
use flate2::Compression;
use pprof::protos::Message;
use pprof::{Frames, Report, Symbol};
use scarb_metadata::{Metadata, MetadataCommand, ScarbCommand};
use scarb_ui::args::PackagesFilter;

#[derive(ValueEnum, Clone, Debug)]
enum OutputType {
    Flamegraph,
    Pprof,
}

/// Execute the main function of a package.
#[derive(Parser, Clone, Debug)]
#[command(author, version)]
struct Args {
    /// Name of the package.
    #[command(flatten)]
    packages_filter: PackagesFilter,

    /// Do not rebuild the package.
    #[arg(long, default_value_t = false)]
    no_build: bool,

    /// Serialized arguments to the executable function.
    #[arg(long, value_delimiter = ',')]
    #[arg(long, conflicts_with_all = ["arguments_file", "profile_file"])]
    arguments: Vec<BigInt>,

    /// Serialized arguments to the executable function from a file.
    #[arg(long, conflicts_with_all = ["arguments", "profile_file"])]
    arguments_file: Option<Utf8PathBuf>,

    /// Use a scoped profile file instead of running the program.
    #[arg(long, conflicts_with_all = ["arguments", "arguments_file"])]
    profile_file: Option<Utf8PathBuf>,

    /// Output file type
    #[arg(long, value_enum, default_value_t = OutputType::Flamegraph)]
    output_type: OutputType,

    /// Path to write the output file.
    #[arg(long)]
    output_file: Utf8PathBuf,

    /// Open output in browser:
    /// - For flamegraph: opens the SVG file directly
    /// - For pprof: starts a pprof web server on port 8000 (requires Go toolchain installed)
    #[arg(long, default_value_t = false)]
    open_in_browser: bool,
}

fn main() -> ExitCode {
    let args: Args = Args::parse();
    if let Err(err) = main_inner(args) {
        println!("\x1b[1;31m(•͡˘_•͡˘)ノð\x1b[0m {err:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn main_inner(args: Args) -> Result<()> {
    let result = if let Some(path) = &args.profile_file {
        std::fs::read_to_string(path)
        .with_context(|| format!("failed to read profile file at {}", path))?
    } else {
        let metadata = MetadataCommand::new().inherit_stderr().exec()?;
        let package = args.packages_filter.match_one(&metadata)?;

        let program_args: Vec<Arg> = if let Some(path) = args.arguments_file {
            let file = fs::File::open(&path).with_context(|| "reading arguments file failed")?;
            let as_vec: Vec<BigUintAsHex> =
                serde_json::from_reader(file).with_context(|| "deserializing arguments file failed")?;
            as_vec
                .into_iter()
                .map(|v| Arg::Value(v.value.into()))
                .collect()
        } else {
            args.arguments
                .iter()
                .map(|v| Arg::Value(v.into()))
                .collect()
        };

        if !args.no_build {
            let filter = PackagesFilter::generate_for::<Metadata>(vec![package.clone()].iter());
            ScarbCommand::new()
                .arg("build")
                .env("SCARB_TARGET_KINDS", "lib")
                .env("SCARB_PACKAGES_FILTER", filter.to_env())
                .run()?;
        }

        let filename = format!("{}.sierra.json", package.name);
        let path = Utf8PathBuf::from(env::var("SCARB_TARGET_DIR")?)
            .join(env::var("SCARB_PROFILE")?)
            .join(filename.clone());

        ensure!(
            path.exists(),
            format!(
                r#"
                Package has not been compiled, file does not exist: {filename}
                make sure you have `[lib]` target in Scarb.toml
            "#
            )
        );

        let program = serde_json::from_str::<VersionedProgram>(
            &fs::read_to_string(path.clone())
                .with_context(|| format!("failed to read Sierra file: {path}"))?,
        )
        .with_context(|| format!("failed to deserialize Sierra program: {path}"))?;

        let profiling_info = profiler::profile(program, program_args)?;
        profiling_info.to_string()
    };

    match args.output_type {
        OutputType::Flamegraph => {
            let mut opt = Options::default();
            let file = fs::File::create(&args.output_file)
                .with_context(|| "failed to create output file")?;
            from_lines(&mut opt, result.lines(), file)
                .with_context(|| "failed to write flamegraph")?;

            println!("Flamegraph written to {}", args.output_file);

            if args.open_in_browser {
                let absolute_path = fs::canonicalize(&args.output_file)?;
                let url = format!("file://{}", absolute_path.display());
                webbrowser::open(&url)?;
            }
        }
        OutputType::Pprof => {
            write_pprof(result.lines(), &args.output_file)?;
            println!("Profile file written to {}", args.output_file);

            if args.open_in_browser {
                Command::new("go")
                    .args([
                        "tool",
                        "pprof",
                        "-http=:8000",
                        &args.output_file.to_string(),
                    ])
                    .status()
                    .with_context(|| "failed to start pprof server")?;
            }
        }
    }

    Ok(())
}

fn write_pprof<'a, I>(lines: I, output_path: &Utf8PathBuf) -> Result<()>
where
    I: Iterator<Item = &'a str>,
{
    let mut data: HashMap<Frames, isize> = HashMap::new();
    for line in lines {
        let (stack, count_str) = line
            .rsplit_once(' ')
            .ok_or_else(|| anyhow::anyhow!("invalid line format: {line}"))?;

        let frames: Vec<Vec<Symbol>> = stack
            .split(';')
            .rev()
            .map(|name| {
                let symbol = Symbol {
                    name: Some(name.as_bytes().to_vec()),
                    filename: None,
                    lineno: None,
                    addr: None,
                };
                vec![symbol]
            })
            .collect();
        let count: isize = count_str
            .parse()
            .context(format!("failed to parse sample count: `{}`", line))?;

        let frame = Frames {
            frames,
            thread_name: "main".into(),
            thread_id: 0,
            sample_timestamp: SystemTime::now(),
        };
        data.insert(frame, count);
    }

    let report = Report {
        data,
        timing: Default::default(),
    };
    let profile = report.pprof()?;
    let file =
        fs::File::create(output_path).with_context(|| "failed to create pprof output file")?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    profile
        .write_to_writer(&mut encoder)
        .with_context(|| "failed to write pprof data")?;
    encoder.finish()?;

    Ok(())
}

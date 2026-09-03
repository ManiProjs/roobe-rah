use std::env;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rah_config::{Config, ProjectConfig};
use rah_core::project::Project;

#[derive(Parser)]
#[command(name = "rah")]
#[command(about = "Roobe Rah — a developer toolbox")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    Inspect {
        #[arg(long)]
        json: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => init()?,
        Commands::Inspect { json } => inspect(json)?,
    }

    Ok(())
}

fn init() -> Result<()> {
    let cwd = env::current_dir()?;

    let project =
        Project::discover(&cwd).context("could not detect a project from the current directory")?;

    let path = project.root().join("rah.toml");

    if path.exists() {
        anyhow::bail!("rah.toml already exists");
    }

    let name = project
        .root()
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project");

    let config = Config {
        project: Some(ProjectConfig {
            name: Some(name.to_owned()),
        }),
    };

    std::fs::write(&path, config.to_toml()?)?;

    println!("Initialized rah.toml in {}", project.root().display());

    Ok(())
}

fn inspect(json: bool) -> Result<()> {
    let cwd = env::current_dir()?;

    let project =
        Project::discover(&cwd).context("could not detect a project from the current directory")?;

    let name = project
        .root()
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");

    let detections = project.detect();

    if json {
        let output = serde_json::json!({
            "project": {
                "name": name,
                "root": project.root(),
                "rah_config": project.has_rah_config(),
            },
            "detections": detections.iter().map(|d| {
                serde_json::json!({
                    "name": d.name,
                    "reason": d.reason,
                })
            }).collect::<Vec<_>>()
        });

        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Project");
        println!("───────");
        println!("Name   {}", name);
        println!("Root   {}", project.root().display());

        println!();
        println!("Rah");
        println!("───");

        if project.has_rah_config() {
            println!("✓ rah.toml");
        } else {
            println!("○ rah.toml not initialized");
        }

        if !detections.is_empty() {
            println!();
            println!("Detected");
            println!("────────");

            for detection in detections {
                println!("✓ {}", detection.name);
            }
        }
    }

    Ok(())
}

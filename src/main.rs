use anyhow::Result;
use clap::{Parser, Subcommand};

use memocap::{cli, install, paths::Paths, tui};

#[derive(Parser)]
#[command(
    name = "memocap",
    version,
    about = "Local SQLite memory shared by four hosts"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Store an explicit local memory.
    Remember {
        content: String,
        #[arg(long, default_value = "context")]
        r#type: String,
        #[arg(long, default_value = "")]
        tags: String,
    },
    /// Search local memory using SQLite full-text search.
    Recall {
        query: String,
        #[arg(long, default_value_t = 5)]
        limit: usize,
    },
    /// Show newest local memories.
    List {
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Delete one memory by ID.
    Forget { id: i64 },
    /// Copy this binary and configure a managed AGENTS.md block.
    Install {
        /// Configure ~/.codex/AGENTS.md instead of ./AGENTS.md.
        #[arg(long)]
        global: bool,
    },
    /// Remove only memocap's managed AGENTS.md block.
    Uninstall {
        #[arg(long)]
        global: bool,
    },
    /// Print install and database status.
    Status {
        #[arg(long)]
        global: bool,
    },
    /// Open the interactive installer.
    Ui,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    match args.command.unwrap_or(Command::Ui) {
        Command::Remember {
            content,
            r#type,
            tags,
        } => {
            let paths = Paths::discover()?;
            let id = cli::remember(&paths.database, &content, &r#type, &tags)?;
            println!("saved #{id}");
        }
        Command::Recall { query, limit } => {
            let paths = Paths::discover()?;
            print!(
                "{}",
                cli::format_memories(&cli::recall(&paths.database, &query, limit)?)
            );
        }
        Command::List { limit } => {
            let paths = Paths::discover()?;
            print!(
                "{}",
                cli::format_memories(&cli::list(&paths.database, limit)?)
            );
        }
        Command::Forget { id } => {
            let paths = Paths::discover()?;
            if cli::forget(&paths.database, id)? {
                println!("deleted #{id}");
            } else {
                println!("not found #{id}");
            }
        }
        Command::Install { global } => {
            let result = install::install(global)?;
            println!("已配置：{}", result.agents_path.display());
            println!("程序：{}", result.binary.display());
            println!("数据库：{}", result.database.display());
        }
        Command::Uninstall { global } => {
            println!(
                "{}",
                if install::uninstall(global)? {
                    "removed memocap config"
                } else {
                    "no memocap config found"
                }
            );
        }
        Command::Status { global } => {
            let result = install::status(global)?;
            let count = cli::count(&result.database).unwrap_or(0);
            print!(
                "{}",
                cli::format_status(
                    &result.database,
                    count,
                    &result.agents_path,
                    result.configured
                )
            );
        }
        Command::Ui => tui::run()?,
    }
    Ok(())
}

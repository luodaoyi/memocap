use anyhow::Result;
use clap::{Parser, Subcommand};

use memocap::{cli, config, config::Target, hosts, install, paths::Paths, remote, server, tui};

#[derive(Parser)]
#[command(
    name = "memocap",
    version,
    about = "SQLite memory shared by four hosts"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Store an explicit memory.
    Remember {
        content: String,
        #[arg(long, default_value = "context")]
        r#type: String,
        #[arg(long, default_value = "")]
        tags: String,
        /// Insert even if similar memories exist.
        #[arg(long)]
        force: bool,
        /// Overwrite an existing memory by id.
        #[arg(long)]
        id: Option<i64>,
    },
    /// Search memory using SQLite full-text search.
    Recall {
        query: String,
        #[arg(long, default_value_t = memocap::store::DEFAULT_RECALL_LIMIT)]
        limit: usize,
        #[arg(long)]
        r#type: Option<String>,
        #[arg(long)]
        max_chars: Option<usize>,
    },
    /// Show newest memories.
    List {
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Delete one memory by ID.
    Forget { id: i64 },
    /// Copy this binary and configure official host rule files.
    Install {
        #[arg(long)]
        global: bool,
        /// Agent hosts (repeatable and comma-separated: codex,claude,grok,pi,opencode).
        #[arg(long = "host", value_delimiter = ',', action = clap::ArgAction::Append)]
        host: Vec<String>,
        /// Every file-writing host (codex, claude, grok).
        #[arg(long)]
        all: bool,
    },
    /// Remove only memocap's managed rule blocks.
    Uninstall {
        #[arg(long)]
        global: bool,
        /// Agent hosts (repeatable and comma-separated: codex,claude,grok,pi,opencode).
        #[arg(long = "host", value_delimiter = ',', action = clap::ArgAction::Append)]
        host: Vec<String>,
        /// Every file-writing host (codex, claude, grok).
        #[arg(long)]
        all: bool,
    },
    /// Print install and database status.
    Status {
        #[arg(long)]
        global: bool,
        /// Agent hosts (repeatable and comma-separated: codex,claude,grok,pi,opencode).
        #[arg(long = "host", value_delimiter = ',', action = clap::ArgAction::Append)]
        host: Vec<String>,
        /// Every file-writing host (codex, claude, grok).
        #[arg(long)]
        all: bool,
    },
    /// Serve the same SQLite over HTTP. Token required.
    Serve {
        #[arg(long, default_value = "127.0.0.1:8787")]
        bind: String,
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
            force,
            id,
        } => {
            let id = match config::resolve_target()? {
                Target::Local { database } => {
                    cli::remember(&database, &content, &r#type, &tags, force, id)?
                }
                Target::Remote { address, token } => {
                    remote::remember(&address, &token, &content, &r#type, &tags, force, id)?
                }
            };
            println!("saved #{id}");
        }
        Command::Recall {
            query,
            limit,
            r#type,
            max_chars,
        } => {
            let kind = r#type.as_deref();
            let memories = match config::resolve_target()? {
                Target::Local { database } => {
                    cli::recall(&database, &query, limit, kind, max_chars)?
                }
                Target::Remote { address, token } => {
                    remote::recall(&address, &token, &query, limit, kind, max_chars)?
                }
            };
            print!("{}", cli::format_memories(&memories));
        }
        Command::List { limit } => {
            let memories = match config::resolve_target()? {
                Target::Local { database } => cli::list(&database, limit)?,
                Target::Remote { address, token } => remote::list(&address, &token, limit)?,
            };
            print!("{}", cli::format_memories(&memories));
        }
        Command::Forget { id } => {
            let deleted = match config::resolve_target()? {
                Target::Local { database } => cli::forget(&database, id)?,
                Target::Remote { address, token } => remote::forget(&address, &token, id)?,
            };
            println!(
                "{}",
                if deleted {
                    format!("deleted #{id}")
                } else {
                    format!("not found #{id}")
                }
            );
        }
        Command::Install { global, host, all } => {
            let paths = Paths::discover()?;
            let selection = hosts::resolve_hosts(all, &host, &paths)?;
            if host.is_empty() && !all {
                if selection.used_fallback {
                    println!("no file-writing hosts detected; installing Codex and Claude");
                } else {
                    println!("detected: {}", hosts::join_host_names(&selection.hosts));
                }
            }
            let result = install::install(global, &selection.hosts)?;
            for path in &result.written {
                println!("已配置：{}", path.display());
            }
            for hint in &result.hints {
                println!("{hint}");
            }
            println!("程序：{}", result.binary.display());
            println!("数据库：{}", result.database.display());
        }
        Command::Uninstall { global, host, all } => {
            let paths = Paths::discover()?;
            let selection = hosts::resolve_hosts(all, &host, &paths)?;
            println!(
                "{}",
                if install::uninstall(global, &selection.hosts)? {
                    "removed memocap config"
                } else {
                    "no memocap config found"
                }
            );
        }
        Command::Status { global, host, all } => {
            let paths = Paths::discover()?;
            let selection = hosts::resolve_hosts(all, &host, &paths)?;
            let result = install::status(global, &selection.hosts)?;
            match config::resolve_target()? {
                Target::Local { database } => {
                    let count = cli::count(&database).unwrap_or(0);
                    print!(
                        "{}",
                        cli::format_status(&database, count, &result.written, result.configured)
                    );
                }
                Target::Remote { address, token } => {
                    let count = remote::count(&address, &token).unwrap_or(0);
                    print!(
                        "{}",
                        cli::format_remote_status(
                            &address,
                            count,
                            &result.written,
                            result.configured
                        )
                    );
                }
            }
        }
        Command::Serve { bind } => {
            let token = config::require_token()?;
            let paths = Paths::discover()?;
            server::serve(&bind, &token, &paths.database)?;
        }
        Command::Ui => tui::run()?,
    }
    Ok(())
}

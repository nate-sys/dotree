use std::{env, fs, path::PathBuf, process::exit};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use console::Term;
use dotree::{
    core::run,
    parser::{self, Config, Node, ShellDef},
    rt_conf,
};

fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();

    let (conf_path, local_conf_dir) = if args.local_mode {
        if let Some(path) = search_local_config().context("Searching local config")? {
            let conf_dir = path.parent().unwrap().to_owned();
            (path, Some(conf_dir))
        } else {
            eprintln!("Couldnt find a local config");
            exit(1);
        }
    } else if let Some(p) = args.conf_file {
        (p, None)
    } else {
        (
            get_default_config_dir()
                .ok_or(anyhow!("Couldn't determin config dir"))?
                .join("dotree.dt"),
            None,
        )
    };

    if !conf_path.exists() {
        eprintln!(
            "Expected config file at {}, but couldn't find it. Please create one.",
            conf_path.display()
        );
        exit(1);
    }

    let conf_src = fs::read_to_string(conf_path).context("loading config")?;
    let Config {
        menu,
        shell_def: file_shell_def,
        snippet_table,
    } = parser::parse(&conf_src).context("Parsing Config")?;

    let env_shell = get_shell_from_env()
        .context("Getting Shell from Env")?
        .unwrap_or_default();
    let shell = file_shell_def.unwrap_or(env_shell);
    rt_conf::init(local_conf_dir, shell);

    let term = Term::stdout();
    term.hide_cursor()?;
    let res = run(&Node::Menu(menu), &args.input, &snippet_table);
    if let Err(e) = term.show_cursor() {
        eprintln!("Warning, couldn't show cursor again:\n{e:?}");
    }
    res
}

fn get_default_config_dir() -> Option<PathBuf> {
    if let Ok(path) = env::var("XDG_CONFIG_HOME") {
        Some(path.into())
    } else {
        dirs::config_dir()
    }
}

fn get_shell_from_env() -> Result<Option<ShellDef>> {
    Ok(if let Ok(src) = std::env::var("DT_DEFAULT_SHELL") {
        Some(parser::parse_shell_string(&format!("shell {src}"))?)
    } else {
        None
    })
}

fn search_local_config() -> Result<Option<PathBuf>> {
    let cwd = std::env::current_dir().context("getting cwd")?;
    let mut cur_dir = cwd.as_path();
    loop {
        let attempt = cur_dir.join("dotree.dt");
        if attempt.exists() {
            return Ok(Some(attempt));
        }
        if let Some(parent) = cur_dir.parent() {
            cur_dir = parent;
        } else {
            return Ok(None);
        }
    }
}

#[derive(Parser)]
struct Args {
    /// Input that will be process character by character, as if it was entered
    input: Vec<String>,

    /// path to config file. Defaults to $XDG_CONFIG_HOME/dotree.dt
    #[arg(long, short)]
    conf_file: Option<PathBuf>,

    /// instead of reading the config file, search all directories from current
    /// to root for a dotree.dt file, and use this, if it is found.
    /// All commands are executed from the files directory
    #[arg(long, short)]
    local_mode: bool,
}

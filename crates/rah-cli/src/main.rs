pub mod helpers;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use console::Style;

use crate::helpers::{failure, print_header, spinner, success};
use rah_config::add_tool;
use rah_core::{project::Project, tools::ToolRequirement};

#[derive(Debug, Parser)]
#[command(
    name = "rah",
    version,
    about = "Roobe Rah — your development environment, sorted."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Add and install tools for the current project.
    Use(UseArgs),

    /// Install tools declared in rah.toml.
    Install(InstallArgs),

    /// Remove a tool installation.
    Uninstall(UninstallArgs),

    /// List configured or installed tools.
    Ls(LsArgs),

    /// Show currently active tool versions.
    Current,

    /// Show the executable path for a tool.
    Which(WhichArgs),

    /// Execute a command inside the Rah environment.
    Exec(ExecArgs),

    /// Run a configured task.
    Run(RunArgs),

    /// List available tasks.
    Tasks,

    /// Show outdated tools.
    Outdated,

    /// Upgrade tools.
    Upgrade,

    /// Remove unused installations.
    Prune,

    /// Show the resolved environment.
    Env(EnvArgs),

    /// Generate shell activation code.
    Activate(ActivateArgs),

    /// Initialize rah.toml.
    Init,

    /// Set a configuration value.
    Set(SetArgs),

    /// Inspect configuration.
    Config(ConfigArgs),

    /// Show or modify Rah settings.
    Settings,

    /// Diagnose Rah and the current project.
    Doctor,

    /// Manage Rah backends/plugins.
    Plugins,

    /// List available tool backends.
    Backends,

    /// Generate shell completions.
    Completion(CompletionArgs),

    /// Trust the current project.
    Trust,

    /// Create/update rah.lock.
    Lock,

    /// Update Rah itself.
    SelfUpdate,
}

#[derive(Debug, Args)]
struct UseArgs {
    /// Install the tool globally instead of in the current project.
    #[arg(short = 'g', long = "global")]
    global: bool,

    /// Tools to use, for example node@24 or rust@stable.
    #[arg(required = true)]
    tools: Vec<String>,
}

#[derive(Debug, Args)]
struct InstallArgs {
    /// Install specific tools instead of the configured toolchain.
    tools: Vec<String>,

    /// Do not install anything that is not already present in rah.lock.
    #[arg(long)]
    locked: bool,
}

#[derive(Debug, Args)]
struct UninstallArgs {
    /// Tool to uninstall.
    tool: String,

    /// Keep the installation and only remove it from configuration.
    #[arg(long)]
    keep: bool,
}

#[derive(Debug, Args)]
struct LsArgs {
    /// Show globally configured tools.
    #[arg(short = 'g', long = "global")]
    global: bool,

    /// Show all installed versions.
    #[arg(long)]
    installed: bool,

    /// Filter by tool name.
    tool: Option<String>,
}

#[derive(Debug, Args)]
struct WhichArgs {
    /// Tool whose executable should be located.
    tool: String,
}

#[derive(Debug, Args)]
struct ExecArgs {
    /// Optional tool/version override.
    #[arg(long)]
    tool: Option<String>,

    /// Command to execute.
    #[arg(required = true, trailing_var_arg = true)]
    command: Vec<String>,
}

#[derive(Debug, Args)]
struct RunArgs {
    /// Task to run.
    task: String,

    /// Arguments passed to the task.
    #[arg(trailing_var_arg = true)]
    args: Vec<String>,
}

#[derive(Debug, Args)]
struct EnvArgs {
    /// Output environment as JSON.
    #[arg(long)]
    json: bool,

    /// Output shell-compatible environment.
    #[arg(long)]
    shell: Option<String>,
}

#[derive(Debug, Args)]
struct ActivateArgs {
    /// Shell to generate activation code for.
    shell: String,
}

#[derive(Debug, Args)]
struct SetArgs {
    /// Set the value globally.
    #[arg(short = 'g', long = "global")]
    global: bool,

    /// Configuration assignment, e.g. NODE_ENV=development.
    value: String,
}

#[derive(Debug, Args)]
struct ConfigArgs {
    /// Read a specific configuration key.
    #[arg(long)]
    get: Option<String>,

    /// Show the active configuration path.
    #[arg(long)]
    path: bool,
}

#[derive(Debug, Args)]
struct CompletionArgs {
    /// Shell to generate completions for.
    shell: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Use(args) => command_use(args).await,

        Commands::Install(args) => command_install(args),

        Commands::Uninstall(args) => command_uninstall(args),

        Commands::Ls(args) => command_ls(args).await,

        Commands::Current => command_current().await,

        Commands::Which(args) => command_which(args).await,

        Commands::Exec(args) => command_exec(args).await,

        Commands::Run(args) => command_run(args).await,

        Commands::Tasks => command_tasks(),

        Commands::Outdated => command_outdated(),

        Commands::Upgrade => command_upgrade(),

        Commands::Prune => command_prune(),

        Commands::Env(args) => command_env(args).await,

        Commands::Activate(args) => command_activate(args).await,

        Commands::Init => command_init(),

        Commands::Set(args) => command_set(args),

        Commands::Config(args) => command_config(args),

        Commands::Settings => command_settings(),

        Commands::Doctor => command_doctor(),

        Commands::Plugins => command_plugins(),

        Commands::Backends => command_backends(),

        Commands::Completion(args) => command_completion(args),

        Commands::Trust => command_trust(),

        Commands::Lock => command_lock(),

        Commands::SelfUpdate => command_self_update(),
    }
}

async fn command_use(args: UseArgs) -> Result<()> {
    let requirements = args
        .tools
        .iter()
        .map(|tool| ToolRequirement::parse(tool))
        .collect::<Result<Vec<_>>>()?;

    if requirements.is_empty() {
        anyhow::bail!("no tools specified");
    }

    print_header("use");

    let config_path = if args.global {
        global_config_path()?
    } else {
        let project = Project::discover(".")
            .context("could not find a project from the current directory")?;

        success("project", project.root.display());

        project.root.join("rah.toml")
    };

    for requirement in &requirements {
        let config_name = if let Some(backend) = &requirement.backend {
            format!("{backend}:{}", requirement.name)
        } else {
            requirement.name.clone()
        };

        add_tool(&config_path, &config_name, &requirement.version)?;
    }

    success(
        "config",
        config_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("rah.toml"),
    );

    let installer = rah_core::tools::installer::ToolInstaller::new();

    for requirement in &requirements {
        let bar = spinner("install", requirement);

        match installer.install(requirement).await {
            Ok(_) => {
                bar.finish_and_clear();

                success("install", requirement);
            }

            Err(error) => {
                bar.finish_and_clear();

                failure("install", requirement);

                println!();
                println!(
                    "    {}",
                    Style::new().red().apply_to("Could not install tool.")
                );

                println!();

                return Err(error);
            }
        }
    }

    println!();
    println!("  {}", Style::new().green().bold().apply_to("done"));
    println!();

    Ok(())
}

fn command_install(args: InstallArgs) -> Result<()> {
    if args.tools.is_empty() {
        println!("Installing tools declared in rah.toml...");

        if args.locked {
            println!("Using rah.lock.");
        }

        // TODO: Load rah.toml and install every requirement.
        return Ok(());
    }

    for tool in args.tools {
        let requirement = ToolRequirement::parse(&tool)?;

        println!("Installing {requirement}...");

        // TODO: Resolve and install through ToolBackend.
    }

    Ok(())
}

fn command_uninstall(args: UninstallArgs) -> Result<()> {
    println!("Uninstalling {}...", args.tool);

    if args.keep {
        println!("Keeping the physical installation.");
    }

    // TODO: Implement installation removal.

    Ok(())
}

async fn command_ls(args: LsArgs) -> Result<()> {
    if args.global {
        anyhow::bail!("global tool listing is not implemented yet");
    }

    let requirements = discover_project_tools()?;

    if requirements.is_empty() {
        println!("No tools configured.");
        return Ok(());
    }

    let environment = rah_core::tools::environment::resolve_environment(&requirements).await?;

    println!("Project tools:");
    println!();

    for requirement in &requirements {
        if let Some(filter) = &args.tool {
            if requirement.name != *filter {
                continue;
            }
        }

        let status = if environment.executables.contains_key(&requirement.name) {
            "installed"
        } else {
            "missing"
        };

        println!(
            "  {:<16} {:<12} {}",
            requirement.name, requirement.version, status
        );
    }

    Ok(())
}

async fn command_current() -> Result<()> {
    let requirements = discover_project_tools()?;

    if requirements.is_empty() {
        println!("No tools configured.");
        return Ok(());
    }

    let environment = rah_core::tools::environment::resolve_environment(&requirements).await?;

    println!("Current tool versions:");
    println!();

    for requirement in &requirements {
        let executable = environment.executables.get(&requirement.name);

        match executable {
            Some(path) => {
                let version = std::process::Command::new(path).arg("--version").output();

                match version {
                    Ok(output) => {
                        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();

                        if text.is_empty() {
                            println!("  {:<16} {}", requirement.name, requirement.version);
                        } else {
                            println!("  {:<16} {}", requirement.name, text);
                        }
                    }

                    Err(_) => {
                        println!("  {:<16} {}", requirement.name, requirement.version);
                    }
                }
            }

            None => {
                println!(
                    "  {:<16} {} (not found)",
                    requirement.name, requirement.version
                );
            }
        }
    }

    Ok(())
}

async fn command_which(args: WhichArgs) -> Result<()> {
    let requirements = discover_project_tools()?;

    if requirements.is_empty() {
        anyhow::bail!("no tools configured");
    }

    let environment = rah_core::tools::environment::resolve_environment(&requirements).await?;

    let executable = environment.executables.get(&args.tool).ok_or_else(|| {
        anyhow::anyhow!(
            "tool `{}` is not available in the Rah environment",
            args.tool
        )
    })?;

    println!("{}", executable);

    Ok(())
}

async fn command_exec(args: ExecArgs) -> Result<()> {
    if args.command.is_empty() {
        anyhow::bail!("no command specified");
    }

    let project = Project::discover(".").context("could not find a project")?;

    let requirements = if let Some(tool) = args.tool {
        vec![ToolRequirement::parse(&tool)?]
    } else {
        discover_project_tools()?
    };

    let environment = rah_core::tools::environment::resolve_environment(&requirements).await?;

    let command_name = &args.command[0];

    let executable = environment
        .executables
        .get(command_name)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(command_name));

    let mut command = std::process::Command::new(&executable);

    command.args(&args.command[1..]).current_dir(&project.root);

    for (key, value) in environment.variables {
        command.env(key, value);
    }

    let status = command
        .status()
        .with_context(|| format!("failed to execute `{}`", executable.display()))?;

    std::process::exit(status.code().unwrap_or(1));
}

fn discover_project_tools() -> Result<Vec<ToolRequirement>> {
    let project = Project::discover(".")?;

    let config_path = project.root.join("rah.toml");

    if !config_path.exists() {
        anyhow::bail!("no rah.toml found in {}", project.root.display());
    }

    let contents = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;

    let document: toml::Table = toml::from_str(&contents).context("failed to parse rah.toml")?;

    let Some(tools) = document.get("tools").and_then(|value| value.as_table()) else {
        return Ok(Vec::new());
    };

    let mut requirements = Vec::new();

    for (name, value) in tools {
        let version = value
            .as_str()
            .with_context(|| format!("tool `{name}` version must be a string"))?;

        let requirement = ToolRequirement::parse(&format!("{name}@{version}"))?;

        requirements.push(requirement);
    }

    Ok(requirements)
}

async fn command_run(args: RunArgs) -> Result<()> {
    let project =
        Project::discover(".").context("could not find a project from the current directory")?;

    let config_path = project.root.join("rah.toml");

    if !config_path.exists() {
        anyhow::bail!("no rah.toml found in {}", config_path.display());
    }

    let contents = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;

    let document: toml::Table = toml::from_str(&contents).context("failed to parse rah.toml")?;

    let tasks = document
        .get("tasks")
        .and_then(|value| value.as_table())
        .ok_or_else(|| anyhow::anyhow!("no tasks defined in rah.toml"))?;

    let requirements = discover_project_tools()?;

    let environment = rah_core::tools::environment::resolve_environment(&requirements).await?;

    let mut visited = std::collections::HashSet::new();
    let mut active = std::collections::HashSet::new();

    run_task(
        &args.task,
        tasks,
        &args.args,
        &project.root,
        &environment,
        &mut visited,
        &mut active,
    )?;

    Ok(())
}

fn run_task(
    name: &str,
    tasks: &toml::map::Map<String, toml::Value>,
    args: &[String],
    project_root: &std::path::Path,
    environment: &rah_core::tools::environment::ResolvedEnvironment,
    visited: &mut std::collections::HashSet<String>,
    active: &mut std::collections::HashSet<String>,
) -> Result<()> {
    if visited.contains(name) {
        return Ok(());
    }

    // Detect circular dependencies.
    if !active.insert(name.to_string()) {
        anyhow::bail!("circular task dependency involving `{name}`");
    }

    let task = tasks
        .get(name)
        .and_then(|value| value.as_table())
        .ok_or_else(|| anyhow::anyhow!("task `{name}` not found"))?;

    let dependencies = task
        .get("depends")
        .and_then(|value| value.as_array())
        .map(|dependencies| {
            dependencies
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("task `{name}` has a non-string dependency"))
                })
                .collect::<Result<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();

    // Run dependencies first.
    for dependency in dependencies {
        run_task(
            dependency,
            tasks,
            &[],
            project_root,
            environment,
            visited,
            active,
        )?;
    }

    let run = task
        .get("run")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow::anyhow!("task `{name}` has no `run` command"))?;

    println!();
    println!(
        "  {} {}",
        console::style("▶").cyan(),
        console::style(name).bold()
    );

    println!("    {}", console::style(run).dim());

    let mut command = std::process::Command::new("sh");

    command.arg("-c").arg(run).current_dir(project_root);

    // Pass task arguments through.
    //
    // `$@` inside the task command can be used to access them.
    command.args(args);

    // Inject Rah's resolved environment.
    for (key, value) in &environment.variables {
        command.env(key, value);
    }

    let status = command
        .status()
        .with_context(|| format!("failed to execute task `{name}`"))?;

    if !status.success() {
        let code = status.code().unwrap_or(1);

        active.remove(name);

        anyhow::bail!("task `{name}` failed with exit code {code}");
    }

    visited.insert(name.to_string());
    active.remove(name);

    println!(
        "    {} {}",
        console::style("✓").green(),
        console::style("done").green()
    );

    Ok(())
}

fn command_tasks() -> Result<()> {
    let project =
        Project::discover(".").context("could not find a project from the current directory")?;

    let config_path = project.root.join("rah.toml");

    if !config_path.exists() {
        anyhow::bail!("no rah.toml found in {}", config_path.display());
    }

    let contents = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;

    let document: toml::Table = toml::from_str(&contents).context("failed to parse rah.toml")?;

    let Some(tasks) = document.get("tasks").and_then(|value| value.as_table()) else {
        println!("No tasks defined.");
        return Ok(());
    };

    if tasks.is_empty() {
        println!("No tasks defined.");
        return Ok(());
    }

    println!("Tasks:");
    println!();

    let mut names = tasks.keys().collect::<Vec<_>>();
    names.sort();

    for name in names {
        let task = tasks
            .get(name)
            .and_then(|value| value.as_table())
            .ok_or_else(|| anyhow::anyhow!("invalid task `{name}`"))?;

        let run = task.get("run").and_then(|value| value.as_str());

        let depends = task
            .get("depends")
            .and_then(|value| value.as_array())
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| value.as_str())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        match (run, depends.is_empty()) {
            (Some(run), true) => {
                println!(
                    "  {:<16} {}",
                    console::style(name).bold(),
                    console::style(run).dim()
                );
            }

            (Some(run), false) => {
                println!(
                    "  {:<16} {}",
                    console::style(name).bold(),
                    console::style(run).dim()
                );

                println!("  {:<16} depends: {}", "", depends.join(", "));
            }

            (None, false) => {
                println!(
                    "  {:<16} depends: {}",
                    console::style(name).bold(),
                    depends.join(", ")
                );
            }

            (None, true) => {
                println!(
                    "  {:<16} {}",
                    console::style(name).bold(),
                    console::style("(no command)").dim()
                );
            }
        }
    }

    println!();

    Ok(())
}

fn command_outdated() -> Result<()> {
    println!("Checking for outdated tools...");

    // TODO: Query backends for newer matching versions.

    Ok(())
}

fn command_upgrade() -> Result<()> {
    println!("Upgrading tools...");

    // TODO: Resolve and install newer compatible versions.

    Ok(())
}

fn command_prune() -> Result<()> {
    println!("Looking for unused installations...");

    // TODO: Find installations not referenced by any config/lockfile.

    Ok(())
}

async fn command_env(args: EnvArgs) -> Result<()> {
    let requirements = discover_project_tools()?;

    let environment = rah_core::tools::environment::resolve_environment(&requirements).await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&environment)?);
        return Ok(());
    }

    if let Some(shell) = args.shell {
        let output = rah_core::tools::environment::shell_environment(&requirements, &shell).await?;

        print!("{output}");

        return Ok(());
    }

    let mut variables = environment.variables.into_iter().collect::<Vec<_>>();

    variables.sort_by(|a, b| a.0.cmp(&b.0));

    for (key, value) in variables {
        println!("{key}={value}");
    }

    Ok(())
}

async fn command_activate(args: ActivateArgs) -> Result<()> {
    match args.shell.as_str() {
        "zsh" => {
            print!(
                r#"
if [[ -z "${{__RAH_ORIG_PATH:-}}" ]]; then
    export __RAH_ORIG_PATH="$PATH"
fi

typeset -g __RAH_LAST_PROJECT=""

_rah_hook() {{
    local project

    project="$(command rah config --path 2>/dev/null)" || project=""

    if [[ "$project" == "$__RAH_LAST_PROJECT" ]]; then
        return
    fi

    __RAH_LAST_PROJECT="$project"

    eval "$(command rah env --shell zsh)"

    rehash
}}

_rah_hook_chpwd() {{
    _rah_hook
}}

autoload -Uz add-zsh-hook
add-zsh-hook chpwd _rah_hook_chpwd

_rah_hook

# ==============================
# Use `eval` to use Roobe Rah
# eval "$(rah activate zsh)"
# ==============================
"#
            );
        }

        "bash" => {
            print!(
                r#"
if [[ -z "${{__RAH_ORIG_PATH:-}}" ]]; then
    export __RAH_ORIG_PATH="$PATH"
fi

_rah_hook() {{
    eval "$(command rah env --shell bash)"
    hash -r
}}

_rah_hook
"#
            );
        }

        "fish" => {
            print!(
                r#"
if not set -q __RAH_ORIG_PATH
    set -gx __RAH_ORIG_PATH $PATH
end

function __rah_hook --on-variable PWD
    eval (command rah env --shell fish)
end

eval (command rah env --shell fish)
"#
            );
        }

        "powershell" => {
            print!(
                r#"
if (-not $env:__RAH_ORIG_PATH) {{
    $env:__RAH_ORIG_PATH = $env:PATH
}}

function global:Invoke-RahHook {{
    Invoke-Expression (rah env --shell powershell | Out-String)
}}

Invoke-RahHook
"#
            );
        }

        shell => {
            anyhow::bail!("unsupported shell `{shell}`");
        }
    }

    Ok(())
}

fn command_init() -> Result<()> {
    let project = Project::discover(".")?;

    let path = project.root.join("rah.toml");

    if path.exists() {
        anyhow::bail!("rah.toml already exists at {}", path.display());
    }

    rah_config::init(&path)?;

    println!("Initialized Rah project.");
    println!("  Root: {}", project.root.display());
    println!("  Config: {}", path.display());

    if !project.detections.is_empty() {
        println!();
        println!("Detected:");

        for detection in &project.detections {
            println!("  ✓ {}", detection.name);
        }
    }

    Ok(())
}

fn command_set(args: SetArgs) -> Result<()> {
    let (key, value) = args.value.split_once('=').ok_or_else(|| {
        anyhow::anyhow!("invalid assignment `{}`; expected KEY=VALUE", args.value)
    })?;

    if key.is_empty() {
        anyhow::bail!("configuration key cannot be empty");
    }

    let path = if args.global {
        global_config_path()?
    } else {
        let project = Project::discover(".")
            .context("could not find a project from the current directory")?;

        project.root.join("rah.toml")
    };

    println!("Setting {} = {:?} in {}", key, value, path.display());

    // TODO: Proper TOML editing.

    Ok(())
}

fn command_config(args: ConfigArgs) -> Result<()> {
    if args.path {
        if let Some(path) = rah_config::find(".") {
            println!("{}", path.display());
        } else {
            println!("No rah.toml found.");
        }

        return Ok(());
    }

    if let Some(key) = args.get {
        println!("Getting configuration key: {key}");

        // TODO: Resolve dotted config key.

        return Ok(());
    }

    if let Some(path) = rah_config::find(".") {
        println!("Configuration:");
        println!("  {}", path.display());

        let config = rah_config::load(&path)?;

        println!();
        println!("Tools:");

        for (name, version) in config.tools {
            println!("  {name} = {version}");
        }

        println!();
        println!("Environment:");

        for (name, value) in config.env {
            println!("  {name} = {value}");
        }

        println!();
        println!("Tasks:");

        for (name, task) in config.tasks {
            println!("  {name}");

            if let Some(run) = task.run {
                println!("    run: {run}");
            }

            if !task.depends.is_empty() {
                println!("    depends: {:?}", task.depends);
            }
        }

        return Ok(());
    }

    println!("No rah.toml found.");

    Ok(())
}

fn command_settings() -> Result<()> {
    println!("Rah settings:");

    // TODO: Implement settings.

    Ok(())
}

fn command_doctor() -> Result<()> {
    println!("Roobe Rah Doctor");
    println!("────────────────");
    println!();

    // TODO: Implement diagnostics.

    println!("No diagnostics implemented yet.");

    Ok(())
}

fn command_plugins() -> Result<()> {
    println!("Rah plugins:");
    println!();
    println!("No plugins installed.");

    // TODO: Plugin management.

    Ok(())
}

fn command_backends() -> Result<()> {
    println!("Rah backends:");
    println!();

    // TODO: Backend registry.

    println!("No backends registered.");

    Ok(())
}

fn command_completion(args: CompletionArgs) -> Result<()> {
    match args.shell.as_str() {
        "bash" | "zsh" | "fish" | "powershell" | "elvish" => {
            println!("# TODO: generate {} completion script", args.shell);
        }

        shell => {
            anyhow::bail!("unsupported shell `{shell}`");
        }
    }

    Ok(())
}

fn command_trust() -> Result<()> {
    let project =
        Project::discover(".").context("could not find a project from the current directory")?;

    println!("Trusted project:");
    println!("  {}", project.root.display());

    // TODO: Persist trust information.

    Ok(())
}

fn command_lock() -> Result<()> {
    println!("Resolving tool versions...");

    // TODO: Resolve rah.toml and write rah.lock.

    Ok(())
}

fn command_self_update() -> Result<()> {
    println!("Checking for Rah updates...");

    // TODO: Self-update implementation.

    Ok(())
}

fn global_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;

    let config_dir = home.join(".config").join("rah");

    std::fs::create_dir_all(&config_dir)
        .with_context(|| format!("failed to create {}", config_dir.display()))?;

    Ok(config_dir.join("config.toml"))
}

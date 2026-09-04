use console::Style;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub fn print_header(command: &str) {
    let dim = Style::new().dim();
    let bold = Style::new().bold();

    println!();
    println!("  {}  {}", bold.apply_to("rah"), dim.apply_to(command));
    println!();
}

pub fn success(label: &str, value: impl std::fmt::Display) {
    println!(
        "  {} {:<10} {}",
        Style::new().green().bold().apply_to("✓"),
        Style::new().dim().apply_to(label),
        value
    );
}

pub fn failure(label: &str, value: impl std::fmt::Display) {
    println!(
        "  {} {:<10} {}",
        Style::new().red().bold().apply_to("✗"),
        Style::new().dim().apply_to(label),
        value
    );
}

pub fn spinner(label: &str, value: impl std::fmt::Display) -> ProgressBar {
    let bar = ProgressBar::new_spinner();

    bar.set_style(
        ProgressStyle::with_template("  {spinner} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );

    bar.enable_steady_tick(Duration::from_millis(80));

    bar.set_message(format!("{:<10} {}", label, value));

    bar
}

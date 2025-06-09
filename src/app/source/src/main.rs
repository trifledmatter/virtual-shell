mod vfs;
mod command;
mod context;
mod commands;

use context::TerminalContext;
use command::{Command, CommandRegistry};
use std::io::{self, Write};

fn main() {
    let mut ctx = TerminalContext::new();
    let registry = CommandRegistry::default_commands();
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        print!("[virt::core] ➤ ");
        stdout.flush().unwrap();
        let mut input = String::new();
        if stdin.read_line(&mut input).is_err() {
            break;
        }
        let input = input.trim();
        if input == "exit" { break; }
        match command::run_command(input, &mut ctx, &registry) {
            Ok(output) => if !output.is_empty() { println!("{}", output); },
            Err(e) => println!("Error: {}", e),
        }
    }
}

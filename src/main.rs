use std::process::{Stdio, Command, Child};
use std::io::{stdin, stdout, Write, Read};
use std::path::Path;
use std::env;

fn main() {
    loop {
        print!("> ");
        let _ = stdout().flush();

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();

        let mut commands = input
            .trim()
            .split(|c| c == '|')
            .flat_map(|s| s.split(" .p "))
            .peekable();
        let mut previous_command_output: Option<Child> = None;

        while let Some(command_str) = commands.next() {
            let mut parts = command_str.trim().split_whitespace();
            if let Some(command) = parts.next() {
                let args: Vec<&str> = parts.collect();

                match command {
                    "cd" => {
                        let new_dir = args.first().unwrap_or(&"/");
                        if let Err(e) = env::set_current_dir(&Path::new(new_dir)) {
                            eprintln!("{}", e);
                        }
                        previous_command_output = None;
                    }
                    "exit" => return,
                    "grep" => {
                        let mut child = Command::new("grep")
                            .args(&args)
                            .stdin(
                                previous_command_output
                                    .take()
                                    .map_or(Stdio::inherit(), |c| Stdio::from(c.stdout.unwrap())),
                            )
                            .stdout(Stdio::piped())
                            .spawn()
                            .expect("failed to spawn grep");

                        let mut output_str = String::new();
                        child
                            .stdout
                            .as_mut()
                            .unwrap()
                            .read_to_string(&mut output_str)
                            .unwrap();
                        child.wait().unwrap();

                        if let Some(pattern) = args.first() {
                            let styled_output = output_str.replace(
                                pattern,
                                &format!("\x1b[1;31m{}\x1b[0m", pattern),
                            );
                            print!("{}", styled_output);
                        } else {
                            print!("{}", output_str);
                        }

                        previous_command_output = None;
                    }
                    _ => {
                        let stdin = previous_command_output
                            .take()
                            .map_or(Stdio::inherit(), |c| Stdio::from(c.stdout.unwrap()));

                        let stdout = if commands.peek().is_some() {
                            Stdio::piped()
                        } else {
                            Stdio::inherit()
                        };

                        let child = Command::new(command)
                            .args(&args)
                            .stdin(stdin)
                            .stdout(stdout)
                            .spawn();

                        match child {
                            Ok(child) => previous_command_output = Some(child),
                            Err(e) => {
                                eprintln!("{}", e);
                                previous_command_output = None;
                            }
                        }
                    }
                }
            }
        }

        if let Some(mut final_command) = previous_command_output {
            let _ = final_command.wait();
        }
    }
}

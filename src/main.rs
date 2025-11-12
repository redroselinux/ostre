use std::process::{Command, Stdio, Child};
use std::io::{stdin, stdout, Write, Read};
use std::env;
use std::path::Path;
use std::collections::HashMap;

fn main() {
    let mut env_vars: HashMap<String, String> = env::vars().collect();
    let mut aliases: HashMap<String, Vec<String>> = HashMap::new();

    loop {
        print!("> ");
        let _ = stdout().flush();

        let mut input = String::new();
        if stdin().read_line(&mut input).is_err() { break; }
        let input = input.trim();
        if input.is_empty() { continue; }

        let segments: Vec<&str> = input.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if segments.is_empty() { continue; }

        let mut previous_child: Option<Child> = None;
        let mut wait_children: Vec<Child> = Vec::new();

        for (idx, seg) in segments.iter().enumerate() {
            let last = idx + 1 == segments.len();
            let mut parts_iter = seg.split_whitespace();
            let first = match parts_iter.next() { Some(f) => f, None => continue };
            let mut command = first.to_string();
            let mut args: Vec<String> = parts_iter.map(|s| s.to_string()).collect();

            if command == "var" {
                let def = args.join(" ");
                if let Some((k, v)) = def.split_once(':') {
                    env_vars.insert(k.trim().to_string(), v.trim().to_string());
                }
                previous_child = None;
                continue;
            }

            if let Some(alias_cmd) = aliases.get(&command) {
                let mut alias_iter = alias_cmd.clone().into_iter();
                if let Some(first_alias) = alias_iter.next() {
                    command = first_alias;
                    let mut new_args: Vec<String> = alias_iter.collect();
                    new_args.extend(args);
                    args = new_args;
                }
            }

            args = args.into_iter()
                .map(|arg| {
                    if arg.starts_with('?') { env_vars.get(&arg[1..]).cloned().unwrap_or_default() } else { arg }
                })
                .collect();

            if command == "echo" {
                if last {
                    println!("{}", args.join(" "));
                    previous_child = None;
                    continue;
                } else {
                    let child = Command::new("printf")
                        .arg("%s\n")
                        .args(&args)
                        .stdout(Stdio::piped())
                        .spawn();
                    match child {
                        Ok(c) => {
                            previous_child = Some(c);
                        }
                        Err(e) => {
                            eprintln!("{}", e);
                            previous_child = None;
                        }
                    }
                    continue;
                }
            }

            match command.as_str() {
                "cd" => {
                    let new_dir = args.first().cloned().unwrap_or("/".to_string());
                    let _ = env::set_current_dir(&Path::new(&new_dir));
                    previous_child = None;
                }
                "exit" => return,
                "alias" => {
                    if !args.is_empty() {
                        let def = args.join(" ");
                        if let Some((name, value)) = def.split_once('=') {
                            aliases.insert(name.trim().to_string(), value.trim().split_whitespace().map(|s| s.to_string()).collect());
                        }
                    } else {
                        for (name, cmd) in &aliases {
                            println!("{}='{}'", name, cmd.join(" "));
                        }
                    }
                    previous_child = None;
                }
                _ => {
                    let stdin_spec = if let Some(mut pc) = previous_child.take() {
                        if let Some(out) = pc.stdout.take() {
                            wait_children.push(pc);
                            Stdio::from(out)
                        } else {
                            wait_children.push(pc);
                            Stdio::inherit()
                        }
                    } else {
                        Stdio::inherit()
                    };

                    let stdout_spec = if last { Stdio::inherit() } else { Stdio::piped() };

                    let child_res = Command::new(&command)
                        .args(&args)
                        .envs(&env_vars)
                        .stdin(stdin_spec)
                        .stdout(stdout_spec)
                        .spawn();

                    match child_res {
                        Ok(mut child) => {
                            if last {
                                if let Some(mut out) = child.stdout.take() {
                                    let mut buf = String::new();
                                    let _ = out.read_to_string(&mut buf);
                                    let _ = child.wait();
                                    if !buf.is_empty() { print!("{}", buf); }
                                } else {
                                    let _ = child.wait();
                                }
                                previous_child = None;
                            } else {
                                previous_child = Some(child);
                            }
                        }
                        Err(e) => {
                            eprintln!("{}", e);
                            previous_child = None;
                        }
                    }
                }
            }
        }

        if let Some(mut final_child) = previous_child {
            let _ = final_child.wait();
        }
        for mut c in wait_children {
            let _ = c.wait();
        }
    }
}

use std::collections::HashMap;
use std::io::{self, Write};
use std::process::Command;
use std::{env, fs};

const BUILTIN_CMDS: [&str; 5] = ["cd", "echo", "exit", "type", "pwd"];

fn main() -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        input.clear();
        stdin.read_line(&mut input).unwrap();

        let mut exit_status = 0;
        let mut cmds = input.trim_end().split(' ').peekable();

        if let Some(c) = cmds.next() {
            match c {
                "cd" => {
                    if let Some(path) = cmds.next() {
                        if let Err(_) = env::set_current_dir(path) {
                            print!("cd: {}: No such file or directory\n", path)
                        }
                    }
                }
                "pwd" => {
                    let pwd = env::current_dir()?;
                    let pwd = pwd.to_str().unwrap();
                    print!("{}\n", pwd);
                }
                "type" => {
                    if let Some(cmd) = cmds.next() {
                        if BUILTIN_CMDS.contains(&cmd) {
                            print!("{} is a shell builtin\n", cmd);
                        } else {
                            let path_cmds = path_cmds()?;
                            if path_cmds.contains_key(cmd) {
                                let path = &path_cmds[cmd];
                                print!("{} is {}/{}\n", cmd, path, cmd)
                            } else {
                                print!("{}: not found\n", cmd);
                            }
                        }
                    }
                }
                "echo" => {
                    loop {
                        if let Some(c) = cmds.next() {
                            print!("{}", c);
                            if cmds.peek() != None {
                                print!(" ");
                            }
                        } else {
                            break;
                        }
                    }
                    print!("\n");
                }
                "exit" => {
                    if let Some(es) = cmds.next() {
                        exit_status = es.parse().unwrap();
                    }
                    std::process::exit(exit_status);
                }
                cmd => {
                    let path_cmds = path_cmds()?;
                    if path_cmds.contains_key(cmd) {
                        let path = format!("{}/{}", path_cmds[cmd], cmd);
                        Command::new(path)
                            .args(cmds)
                            .status()
                            .expect("failed to execute process");
                    } else {
                        print!("{}: command not found\n", cmd.trim_end());
                    }
                }
            }
        }
        io::stdout().flush().unwrap();
    }
}

fn path_cmds() -> anyhow::Result<HashMap<String, String>> {
    let mut cmds = HashMap::new();

    let key = "PATH";
    if let Some(paths) = env::var_os(key) {
        for path in env::split_paths(&paths) {
            for entry in fs::read_dir(&path)? {
                let entry = entry?;
                if entry.path().is_file() {
                    let cmd = entry
                        .path()
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned();
                    let path = path.clone().to_str().unwrap().to_owned();

                    cmds.entry(cmd).or_insert(path);
                }
            }
        }
    } else {
        println!("{key} is not defined in the environment.");
    }
    Ok(cmds)
}

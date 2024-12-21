use std::collections::HashMap;
use std::io::{self, Write};
use std::iter::Peekable;
use std::process::Command;
use std::str::Chars;
use std::{env, fs};

const BUILTIN_CMDS: [&str; 5] = ["cd", "echo", "exit", "type", "pwd"];

fn parse_cmds(src: &str) -> Vec<String> {
    let mut cmds: Vec<String> = Vec::new();
    let mut src = src.chars().peekable();

    loop {
        match src.peek() {
            None => {
                break;
            }
            Some(c) if c.is_whitespace() => {
                _ = src.next(); // consume white space
                continue;
            }
            Some('\'') => {
                _ = src.next(); // consume opening quote
                let cmd = parse_single_quote(&mut src);
                _ = src.next_if_eq(&'\''); // consume closing quote
                cmds.push(cmd);
            }
            Some('"') => {
                _ = src.next(); // consume opening quote
                let cmd = parse_double_quote(&mut src);
                _ = src.next_if_eq(&'"'); // consume closing quote
                cmds.push(cmd);
            }
            Some('\\') => {
                _ = src.next();
                let cmd = parse_escape_character(&mut src);
                cmds.push(cmd);
            }
            _ => {
                let mut cmd = String::new();
                'a: loop {
                    if let Some(c) =
                        src.next_if(|c| !(['\'', '"', '\\'].contains(c) || c.is_whitespace()))
                    {
                        cmd.push(c);
                    } else {
                        break 'a;
                    }
                }
                cmds.push(cmd);
            }
        }
    }

    cmds
}

fn parse_escape_character(src: &mut Peekable<Chars<'_>>) -> String {
    todo!()
}

fn parse_double_quote(src: &mut Peekable<Chars<'_>>) -> String {
    todo!()
}

fn parse_single_quote(src: &mut Peekable<Chars<'_>>) -> String {
    let mut cmd = String::new();
    while let Some(c) = src.next_if(|c| *c != '\'') {
        cmd.push(c);
    }
    cmd
}

fn main() -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        input.clear();
        stdin.read_line(&mut input).unwrap();

        let mut exit_status = 0;
        let cmds = parse_cmds(&input);
        let mut cmds = cmds.iter().map(|s| s.as_str()).peekable();
        // let mut cmds = input.trim_end().split(' ').peekable();

        if let Some(c) = cmds.next() {
            match c {
                "cd" => {
                    if cmds.peek() == Some(&"~") {
                        cmds.next();
                        let key = "HOME";
                        if let Some(home_path) = env::var_os(key) {
                            if let Err(_) = env::set_current_dir(&home_path) {
                                print!(
                                    "cd: {}: No such file or directory\n",
                                    home_path.into_string().unwrap()
                                )
                            }
                        }
                    }
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

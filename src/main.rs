use std::collections::HashMap;
use std::io::{self, Write};
use std::process::Command;
use std::{env, fs};

const BUILTIN_CMDS: [&str; 5] = ["cd", "echo", "exit", "type", "pwd"];

enum ParseState {
    None,
    SingleQuote,
    DoubleQuote,
    BackSlash,
}

fn parse(src: &str) -> Vec<String> {
    let src: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut state = ParseState::None;
    let mut cmds = Vec::new();
    let mut cmd = String::new();

    loop {
        match state {
            ParseState::None => {
                if i >= src.len() {
                    if !cmd.is_empty() {
                        cmds.push(cmd.clone());
                        cmd.clear();
                    }
                    break;
                }
                if i < src.len() {
                    match src[i] {
                        '\'' => {
                            i += 1; // consume single quote
                            state = ParseState::SingleQuote;
                        }
                        '"' => {
                            i += 1; // consume double quote
                            state = ParseState::DoubleQuote;
                        }
                        '\\' => {
                            i += 1; // consume backslash quote
                            state = ParseState::BackSlash;
                        }
                        c if c.is_whitespace() => {
                            if !cmd.is_empty() {
                                cmds.push(cmd.clone());
                                cmd.clear();
                            }
                            i += 1;
                        }
                        c => {
                            cmd.push(c);
                            i += 1;
                        }
                    }
                }
            }
            ParseState::SingleQuote => {
                while i < src.len() && src[i] != '\'' {
                    cmd.push(src[i]);
                    i += 1;
                }
                i += 1; // consume single quote
                state = ParseState::None;
            }
            ParseState::DoubleQuote => {
                while i < src.len() && src[i] != '"' {
                    if src[i] == '\\'
                        && i + 1 < src.len()
                        && ['\\', '$', '"', '`', '\n'].contains(&src[i + 1])
                    {
                        i += 1;
                        cmd.push(src[i]);
                    } else {
                        cmd.push(src[i]);
                    }
                    i += 1;
                }
                i += 1; // consume double quote
                state = ParseState::None;
            }
            ParseState::BackSlash => {
                if i < src.len() {
                    cmd.push(src[i]);
                    i += 1;
                }
                state = ParseState::None;
            }
        }
    }

    cmds
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
        let mut i = 0;
        let cmds = parse(&input);

        if i < cmds.len() {
            let cmd = &cmds[i];
            i += 1;
            match cmd.as_str() {
                "cd" => {
                    if i < cmds.len() && cmds[i] == "~" {
                        i += 1;
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
                    if i < cmds.len() {
                        let path = &cmds[i];
                        i += 1;
                        if let Err(_) = env::set_current_dir(path) {
                            print!("cd: {}: No such file or directory\n", path)
                        }
                        _ = i; // NOTE: just to remove unused assignment warning
                    }
                }
                "pwd" => {
                    let pwd = env::current_dir()?;
                    let pwd = pwd.to_str().unwrap();
                    print!("{}\n", pwd);
                }
                "type" => {
                    if i < cmds.len() {
                        let cmd = &cmds[i];
                        i += 1;
                        _ = i;
                        if BUILTIN_CMDS.contains(&cmd.as_str()) {
                            print!("{} is a shell builtin\n", cmd);
                        } else {
                            let path_cmds = path_cmds()?;
                            if path_cmds.contains_key(cmd.as_str()) {
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
                        if i == cmds.len() {
                            break;
                        }
                        print!("{}", cmds[i]);
                        if i != cmds.len() - 1 {
                            print!(" ");
                        }
                        i += 1;
                    }
                    print!("\n");
                }
                "exit" => {
                    if i < cmds.len() {
                        exit_status = cmds[i].parse().unwrap_or_default();
                        i += 1;
                        _ = i;
                    }
                    std::process::exit(exit_status);
                }
                cmd => {
                    let path_cmds = path_cmds()?;
                    if path_cmds.contains_key(cmd) {
                        let path = format!("{}/{}", path_cmds[cmd], cmd);
                        Command::new(path)
                            .args(&cmds[i..])
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
            let dir = match fs::read_dir(&path) {
                Ok(d) => d,
                Err(_) => continue,
            };
            for entry in dir {
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

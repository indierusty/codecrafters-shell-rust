use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::process::{Command, Output};
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

#[derive(Debug)]
enum RedirectionMode {
    Default,
    Append(String),
    Direct(String),
}

#[derive(Debug)]
enum Channel {
    Stdout,
    Stderr,
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
        let mut stdout_redirection = RedirectionMode::Default;
        let mut stderr_redirection = RedirectionMode::Default;

        let cmds = parse(&input);
        let mut new_cmds: Vec<String> = Vec::new();

        let mut i = 0;
        new_cmds.push(cmds[i].clone());
        i += 1;

        while i < cmds.len() {
            if [">", "1>", ">>", "1>>"].contains(&cmds[i].as_str()) {
                i += 1;
                if i < cmds.len() {
                    let mode = cmds[i - 1].as_str();
                    let path = cmds[i].clone();
                    stdout_redirection = if mode == ">" || mode == "1>" {
                        RedirectionMode::Direct(path)
                    } else {
                        RedirectionMode::Append(path)
                    };
                } else {
                    eprint!("ERR: path is not given for output redirection\n")
                }
            } else if ["2>", "2>>"].contains(&cmds[i].as_str()) {
                i += 1;
                if i < cmds.len() {
                    let mode = cmds[i - 1].as_str();
                    let path = cmds[i].clone();
                    stderr_redirection = if mode == "2>>" {
                        RedirectionMode::Append(path)
                    } else {
                        RedirectionMode::Direct(path)
                    };
                } else {
                    eprint!("ERR: path is not given for output redirection\n")
                }
            } else {
                new_cmds.push(cmds[i].clone());
            }
            i += 1;
        }

        let cmds = new_cmds;
        let mut i = 0;

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
                                write(
                                    &stderr_redirection,
                                    Channel::Stderr,
                                    format!(
                                        "cd: {}: No such file or directory\n",
                                        home_path.into_string().unwrap()
                                    )
                                    .as_bytes(),
                                );
                            }
                        }
                    }
                    if i < cmds.len() {
                        let path = &cmds[i];
                        i += 1;
                        if let Err(_) = env::set_current_dir(path) {
                            // print!("cd: {}: No such file or directory\n", path);
                            write(
                                &stderr_redirection,
                                Channel::Stderr,
                                format!("cd: {}: No such file or directory\n", path).as_bytes(),
                            );
                        }
                        _ = i; // NOTE: just to remove unused assignment warning
                    }
                }
                "pwd" => {
                    let pwd = env::current_dir()?;
                    let pwd = pwd.to_str().unwrap();
                    write(
                        &stdout_redirection,
                        Channel::Stdout,
                        format!("{}\n", pwd).as_bytes(),
                    );
                    // print!("{}\n", pwd);
                }
                "type" => {
                    if i < cmds.len() {
                        let cmd = &cmds[i];

                        i += 1;
                        _ = i;

                        let res;

                        if BUILTIN_CMDS.contains(&cmd.as_str()) {
                            res = Some(format!("{} is a shell builtin\n", cmd));
                        } else {
                            let path_cmds = path_cmds()?;
                            if path_cmds.contains_key(cmd.as_str()) {
                                let path = &path_cmds[cmd];
                                res = Some(format!("{} is {}/{}\n", cmd, path, cmd));
                            } else {
                                res = Some(format!("{}: not found\n", cmd));
                            }
                        }

                        if let Some(res) = res {
                            write(&stdout_redirection, Channel::Stdout, res.as_bytes());
                        }
                    }
                }
                "echo" => {
                    let mut res = String::new();
                    loop {
                        if i == cmds.len() {
                            break;
                        }
                        res.push_str(cmds[i].as_str());
                        if i != cmds.len() - 1 {
                            res.push(' ');
                        }
                        i += 1;
                    }
                    res.push('\n');
                    write(&stdout_redirection, Channel::Stdout, res.as_bytes());
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
                        let Output {
                            status: _,
                            stdout,
                            stderr,
                        } = Command::new(cmd)
                            .args(&cmds[i..])
                            .output()
                            .expect("failed to execute process");

                        write(&stdout_redirection, Channel::Stdout, &stdout);
                        write(&stderr_redirection, Channel::Stderr, &stderr);
                    } else {
                        let res = format!("{}: command not found\n", cmd.trim_end());
                        write(&stderr_redirection, Channel::Stderr, res.as_bytes());
                    }
                }
            }
        }
        io::stdout().flush().unwrap();
    }
}

fn write(mode: &RedirectionMode, std_type: Channel, src: &[u8]) {
    match mode {
        RedirectionMode::Default => match std_type {
            Channel::Stdout => io::stdout().write_all(src).unwrap(),
            Channel::Stderr => io::stderr().write_all(src).unwrap(),
        },
        RedirectionMode::Append(path) => {
            let mut file = OpenOptions::new()
                .write(true)
                .append(true)
                .open(path)
                .unwrap();

            file.write_all(&src).unwrap();
        }
        RedirectionMode::Direct(path) => fs::write(path, src).unwrap(),
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

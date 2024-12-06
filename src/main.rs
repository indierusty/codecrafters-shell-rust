use std::io::{self, Write};

const BUILTIN_CMDS: [&str; 3] = ["echo", "exit", "type"];

fn main() {
    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        input.clear();
        stdin.read_line(&mut input).unwrap();

        let mut exit_status = 0;
        let mut cmd = input.trim_end().split(' ').peekable();

        if let Some(c) = cmd.next() {
            match c {
                "type" => {
                    if let Some(c) = cmd.next() {
                        if BUILTIN_CMDS.contains(&c) {
                            print!("{} is a shell builtin\n", c);
                        } else {
                            print!("{}: not found\n", c);
                        }
                    }
                }
                "echo" => {
                    loop {
                        if let Some(c) = cmd.next() {
                            print!("{}", c);
                            if cmd.peek() != None {
                                print!(" ");
                            }
                        } else {
                            break;
                        }
                    }
                    print!("\n");
                }
                "exit" => {
                    if let Some(es) = cmd.next() {
                        exit_status = es.parse().unwrap();
                    }
                    std::process::exit(exit_status);
                }
                invalid => {
                    print!("{}: command not found\n", invalid.trim_end());
                }
            }
        }
        io::stdout().flush().unwrap();
    }
}

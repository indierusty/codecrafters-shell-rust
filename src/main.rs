use std::io::{self, Write};

fn main() {
    let stdin = io::stdin();
    let mut cmd = String::new();

    loop {
        cmd.clear();

        print!("$ ");
        io::stdout().flush().unwrap();

        stdin.read_line(&mut cmd).unwrap();
        match &cmd {
            invalid => {
                print!("{}: command not found\n", invalid.trim_end());
            }
        }
        io::stdout().flush().unwrap();
    }
}

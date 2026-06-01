use std::fs;
use std::path::PathBuf;

use clap::CommandFactory;

fn main() -> std::io::Result<()> {
    let out_dir = PathBuf::from("target/man");
    fs::create_dir_all(&out_dir)?;

    let cmd = atomwrite::cli::Cli::command();

    let man = clap_mangen::Man::new(cmd.clone());
    let mut buf = Vec::new();
    man.render(&mut buf)?;
    fs::write(out_dir.join("atomwrite.1"), &buf)?;

    for sub in cmd.get_subcommands() {
        let name = format!("atomwrite-{}", sub.get_name());
        let man = clap_mangen::Man::new(sub.clone());
        let mut buf = Vec::new();
        man.render(&mut buf)?;
        fs::write(out_dir.join(format!("{name}.1")), &buf)?;
    }

    println!("Man pages generated in {}", out_dir.display());
    Ok(())
}

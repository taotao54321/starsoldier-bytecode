use structopt::StructOpt;

use starsoldier_bytecode as bytecode;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(parse(from_os_str))]
    path_in: std::path::PathBuf,

    #[structopt(parse(from_os_str))]
    path_out: std::path::PathBuf,
}

fn main() -> eyre::Result<()> {
    let opt = Opt::from_args();

    let buf = bytecode::asm(std::fs::read(opt.path_in)?.as_slice())?;

    std::fs::write(opt.path_out, buf)?;

    Ok(())
}

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

    let rdr = std::io::BufReader::new(std::fs::File::open(opt.path_in)?);
    let buf = bytecode::asm(rdr)?;

    std::fs::write(opt.path_out, buf)?;

    Ok(())
}

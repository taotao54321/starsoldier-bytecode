use structopt::StructOpt;

use starsoldier_bytecode as bytecode;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(parse(from_os_str))]
    path_in: std::path::PathBuf,
}

fn main() -> eyre::Result<()> {
    const BUF_LEN_MAX: usize = 0x100;

    let opt = Opt::from_args();

    let buf = std::fs::read(opt.path_in)?;
    if buf.len() > BUF_LEN_MAX {
        eprintln!("warning: buffer length exceeds {}", BUF_LEN_MAX);
    }

    let wtr = std::io::stdout();
    let wtr = std::io::BufWriter::new(wtr.lock());
    bytecode::disasm(wtr, &buf)?;

    Ok(())
}

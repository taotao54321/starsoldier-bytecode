//! バイトコードを逆アセンブル -> アセンブルして元に戻るかテストする。

use structopt::StructOpt;

use starsoldier_bytecode as bytecode;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(parse(from_os_str))]
    path_in: std::path::PathBuf,
}

fn main() -> eyre::Result<()> {
    let opt = Opt::from_args();

    let buf_orig = std::fs::read(opt.path_in)?;

    let mut assembly = Vec::<u8>::new();
    bytecode::disasm(&mut assembly, &buf_orig)?;

    let buf = bytecode::asm(assembly.as_slice())?;

    assert_eq!(buf, buf_orig);

    Ok(())
}

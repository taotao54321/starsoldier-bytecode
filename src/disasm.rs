use std::collections::HashMap;
use std::io::Write;

use thiserror::Error;

use crate::op::*;

#[derive(Debug, Error)]
pub enum DisasmError {
    #[error("address {addr:#04X}: decode failed")]
    Decode {
        addr: usize,
        #[source]
        source: DecodeError,
    },

    #[error("address {addr:#04X}: invalid destination: {addr_dst:#04X}")]
    InvalidDestination { addr: usize, addr_dst: u8 },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type DisasmResult<T> = Result<T, DisasmError>;

pub fn disasm<W: Write>(mut wtr: W, buf: &[u8]) -> DisasmResult<()> {
    #[derive(Debug)]
    struct Statement {
        addr: usize,
        op: Op,
    }

    let mut stmts = vec![];
    let mut labels = HashMap::new();

    let mut addr = 0;
    while !buf[addr..].is_empty() {
        let op = Op::decode(&buf[addr..]).map_err(|e| DisasmError::Decode { addr, source: e })?;

        // ジャンプ命令などの場合、飛び先をラベルを振るべきアドレスとして記録。
        //
        // SetJumpOnDamage の場合、オペランドがバッファ内オフセットとして正しければ無条件でラベルを
        // 振り、さもなくばボスのHP設定とみなして無視する。
        // これだとボスの場合に余計なラベルが発生しうるが、それは許容する。
        if let Some(addr_dst) = op.addr_destination() {
            if (0..buf.len()).contains(&usize::from(addr_dst)) {
                labels.insert(usize::from(addr_dst), format!("L{:02X}", addr_dst));
            } else {
                if !matches!(op, Op::SetJumpOnDamage(_)) {
                    return Err(DisasmError::InvalidDestination { addr, addr_dst });
                }
            }
        }

        stmts.push(Statement { addr, op });
        addr += op.len();
    }

    for stmt in stmts {
        if let Some(label) = labels.get(&stmt.addr) {
            writeln!(wtr, "{}:", label)?;
        }

        // TODO: ループも含めたインデント管理
        write!(wtr, "        ")?;

        match stmt.op {
            Op::Move(dir) => writeln!(wtr, "move {:#04X}", dir.index())?,
            Op::Jump(addr) => writeln!(wtr, "jump {}", labels.get(&usize::from(addr)).unwrap())?,
            Op::SetSleepTimer(idx) => writeln!(wtr, "set_sleep_timer {}", idx)?,
            Op::LoopBegin(idx) => writeln!(wtr, "loop_begin {}", idx)?,
            Op::LoopEnd => writeln!(wtr, "loop_end")?,
            Op::ShootDirection(dir) => writeln!(wtr, "shoot_direction {:#04X}", dir.index())?,
            Op::SetSprite(idx) => writeln!(wtr, "set_sprite {}", idx)?,
            Op::SetHomingTimer(idx) => writeln!(wtr, "set_homing_timer {}", idx)?,
            Op::SetInversion(inv_x, inv_y) => writeln!(
                wtr,
                "set_inversion {}, {}",
                u8::from(inv_x),
                u8::from(inv_y)
            )?,
            Op::SetPosition(x, y) => writeln!(wtr, "set_position {}, {}", x, y)?,
            Op::SetJumpOnDamage(addr) => {
                if (0..buf.len()).contains(&usize::from(addr)) {
                    writeln!(
                        wtr,
                        "set_jump_on_damage {}",
                        labels.get(&usize::from(addr)).unwrap()
                    )?;
                } else {
                    writeln!(wtr, "set_health {}", addr)?;
                }
            }
            Op::IncrementSprite => writeln!(wtr, "increment_sprite")?,
            Op::DecrementSprite => writeln!(wtr, "decrement_sprite")?,
            Op::SetPart(part) => writeln!(wtr, "set_part {}", part)?,
            Op::RandomizeX(mask) => writeln!(wtr, "randomize_x {:#04X}", mask)?,
            Op::RandomizeY(mask) => writeln!(wtr, "randomize_y {:#04X}", mask)?,
            Op::BccX(addr) => writeln!(wtr, "bcc_x {}", labels.get(&usize::from(addr)).unwrap())?,
            Op::BcsX(addr) => writeln!(wtr, "bcs_x {}", labels.get(&usize::from(addr)).unwrap())?,
            Op::BccY(addr) => writeln!(wtr, "bcc_y {}", labels.get(&usize::from(addr)).unwrap())?,
            Op::BcsY(addr) => writeln!(wtr, "bcs_y {}", labels.get(&usize::from(addr)).unwrap())?,
            Op::ShootAim => writeln!(wtr, "shoot_aim")?,
        }
    }

    Ok(())
}

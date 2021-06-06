use std::collections::HashMap;
use std::io::BufRead;

use logos::{Lexer, Logos};
use thiserror::Error;

use crate::direction::Direction;
use crate::op::Op;

#[derive(Debug, Error)]
pub enum AsmError {
    #[error("line {lineno}: parse error: {msg}")]
    Parse { lineno: usize, msg: String },

    #[error("line {lineno}: code size overflow")]
    Overflow { lineno: usize },

    #[error("line {lineno}: undefined label: {label}")]
    UndefinedLabel { lineno: usize, label: String },

    #[error("line {lineno}: set_jump_on_damage 0 is not permitted")]
    SetJumpOnDamageZero { lineno: usize },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type AsmResult<T> = Result<T, AsmError>;

#[derive(Debug, Logos)]
enum Token {
    #[regex(r"move")]
    MnemonicMove,

    #[regex(r"jump")]
    MnemonicJump,

    #[regex(r"set_sleep_timer")]
    MnemonicSetSleepTimer,

    #[regex(r"loop_begin")]
    MnemonicLoopBegin,

    #[regex(r"loop_end")]
    MnemonicLoopEnd,

    #[regex(r"shoot_direction")]
    MnemonicShootDirection,

    #[regex(r"set_sprite")]
    MnemonicSetSprite,

    #[regex(r"set_homing_timer")]
    MnemonicSetHomingTimer,

    #[regex(r"set_inversion")]
    MnemonicSetInversion,

    #[regex(r"set_position")]
    MnemonicSetPosition,

    #[regex(r"set_jump_on_damage")]
    MnemonicSetJumpOnDamage,

    #[regex(r"unset_jump_on_damage")]
    MnemonicUnsetJumpOnDamage,

    // バイトコードは set_jump_on_damage と同一。
    #[regex(r"set_health")]
    MnemonicSetHealth,

    #[regex(r"increment_sprite")]
    MnemonicIncrementSprite,

    #[regex(r"decrement_sprite")]
    MnemonicDecrementSprite,

    #[regex(r"set_part")]
    MnemonicSetPart,

    #[regex(r"randomize_x")]
    MnemonicRandomizeX,

    #[regex(r"randomize_y")]
    MnemonicRandomizeY,

    #[regex(r"bcc_x")]
    MnemonicBccX,

    #[regex(r"bcs_x")]
    MnemonicBcsX,

    #[regex(r"bcc_y")]
    MnemonicBccY,

    #[regex(r"bcs_y")]
    MnemonicBcsY,

    #[regex(r"shoot_aim")]
    MnemonicShootAim,

    #[regex(r"change_music")]
    MnemonicChangeMusic,

    #[regex(r"[A-Za-z_][[:word:]]*:", |lex| lex.slice()[0..lex.slice().len()-1].to_owned())]
    LabelDefinition(String),

    #[regex(r"[A-Za-z_][[:word:]]*", |lex| lex.slice().to_owned())]
    LabelReference(String),

    #[regex(r"0x[A-Fa-f0-9]+", |lex| u8::from_str_radix(&lex.slice()[2..], 16))]
    #[regex(r"0o[0-7]+", |lex| u8::from_str_radix(&lex.slice()[2..], 8))]
    #[regex(r"0b[01]+", |lex| u8::from_str_radix(&lex.slice()[2..], 2))]
    #[regex(r"[0-9]+", |lex| u8::from_str_radix(lex.slice(), 10))]
    Number(u8),

    #[regex(r",")]
    Comma,

    #[error]
    #[regex(r"[[:space:]]+", logos::skip)]
    Error,
}

#[derive(Debug)]
struct Statement {
    lineno: usize,
    addr: usize,
    op: Op,
    label: Option<String>,
}

impl Statement {
    fn new(lineno: usize, addr: usize, op: Op) -> Self {
        Self {
            lineno,
            addr,
            op,
            label: None,
        }
    }

    fn with_label(lineno: usize, addr: usize, op: Op, label: String) -> Self {
        Self {
            lineno,
            addr,
            op,
            label: Some(label),
        }
    }
}

pub fn asm<R: BufRead>(rdr: R) -> AsmResult<Vec<u8>> {
    let mut stmts = vec![];
    let mut label_to_addr = HashMap::new();

    let mut addr = 0;
    for (i, line) in rdr.lines().enumerate() {
        let lineno = i + 1;
        let line = line?;
        let line = trim_comment(&line);
        if line.trim().is_empty() {
            continue;
        }

        parse_line(lineno, line, &mut addr, &mut stmts, &mut label_to_addr)?;
        if addr > 0x100 {
            return Err(AsmError::Overflow { lineno });
        }
    }

    resolve_labels(&mut stmts, &label_to_addr)?;

    let mut buf = vec![0_u8; addr];
    emit_code(&mut buf, &stmts);

    Ok(buf)
}

fn emit_code(buf: &mut [u8], stmts: &[Statement]) {
    let mut addr = 0;
    for op in stmts.iter().map(|stmt| stmt.op) {
        op.encode(&mut buf[addr..]);
        addr += op.len();
    }
}

fn resolve_labels(stmts: &mut [Statement], label_to_addr: &HashMap<String, u8>) -> AsmResult<()> {
    for stmt in stmts {
        if let Some(label) = stmt.label.take() {
            let addr = *label_to_addr
                .get(&label)
                .ok_or_else(|| AsmError::UndefinedLabel {
                    lineno: stmt.lineno,
                    label,
                })?;
            stmt.op = match stmt.op {
                Op::Jump(_) => Op::Jump(addr),
                Op::SetJumpOnDamage(_) => {
                    if addr == 0 {
                        return Err(AsmError::SetJumpOnDamageZero {
                            lineno: stmt.lineno,
                        });
                    }
                    Op::SetJumpOnDamage(addr)
                }
                Op::BccX(_) => Op::BccX(addr),
                Op::BcsX(_) => Op::BcsX(addr),
                Op::BccY(_) => Op::BccY(addr),
                Op::BcsY(_) => Op::BcsY(addr),
                _ => unreachable!(),
            };
        }
    }

    Ok(())
}

fn parse_line(
    lineno: usize,
    line: &str,
    addr: &mut usize,
    stmts: &mut Vec<Statement>,
    labels: &mut HashMap<String, u8>,
) -> AsmResult<()> {
    use std::convert::TryFrom;

    let mut lex = Token::lexer(line);
    let lex = &mut lex;

    macro_rules! add_stmt {
        ($op:expr) => {{
            let op = $op;
            stmts.push(Statement::new(lineno, *addr, op));
            *addr += op.len();
        }};
    }

    macro_rules! add_stmt_with_label {
        ($op:expr, $label:expr) => {{
            let op = $op;
            stmts.push(Statement::with_label(lineno, *addr, op, $label));
            *addr += op.len();
        }};
    }

    match lex.next() {
        Some(Token::LabelDefinition(label)) => {
            expect_end(lineno, lex)?;
            labels.insert(label, u8::try_from(*addr).unwrap());
        }

        Some(Token::MnemonicMove) => {
            let dir = expect_dir(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_move(dir));
        }

        Some(Token::MnemonicJump) => {
            let label = expect_label_reference(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt_with_label!(Op::new_jump(0), label);
        }

        Some(Token::MnemonicSetSleepTimer) => {
            let idx = expect_nibble(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_set_sleep_timer(idx));
        }

        Some(Token::MnemonicLoopBegin) => {
            let idx = expect_loop_idx(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_loop_begin(idx));
        }

        Some(Token::MnemonicLoopEnd) => {
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_loop_end());
        }

        Some(Token::MnemonicShootDirection) => {
            let dir = expect_dir_shoot(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_shoot_direction(dir));
        }

        Some(Token::MnemonicSetSprite) => {
            let idx = expect_nibble(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_set_sprite(idx));
        }

        Some(Token::MnemonicSetHomingTimer) => {
            let idx = expect_nibble(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_set_homing_timer(idx));
        }

        Some(Token::MnemonicSetInversion) => {
            let inv_x = expect_bool(lineno, lex)?;
            expect_comma(lineno, lex)?;
            let inv_y = expect_bool(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_set_inversion(inv_x, inv_y));
        }

        Some(Token::MnemonicSetPosition) => {
            let x = expect_number(lineno, lex)?;
            expect_comma(lineno, lex)?;
            let y = expect_number(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_set_position(x, y));
        }

        Some(Token::MnemonicSetJumpOnDamage) => {
            let label = expect_label_reference(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt_with_label!(Op::new_set_jump_on_damage(0xFF), label);
        }

        Some(Token::MnemonicUnsetJumpOnDamage) => {
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_unset_jump_on_damage());
        }

        Some(Token::MnemonicSetHealth) => {
            let health = expect_number(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_set_jump_on_damage(health));
        }

        Some(Token::MnemonicIncrementSprite) => {
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_increment_sprite());
        }

        Some(Token::MnemonicDecrementSprite) => {
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_decrement_sprite());
        }

        Some(Token::MnemonicSetPart) => {
            let part = expect_number(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_set_part(part));
        }

        Some(Token::MnemonicRandomizeX) => {
            let mask = expect_number(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_randomize_x(mask));
        }

        Some(Token::MnemonicRandomizeY) => {
            let mask = expect_number(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_randomize_y(mask));
        }

        Some(Token::MnemonicBccX) => {
            let label = expect_label_reference(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt_with_label!(Op::new_bcc_x(0), label);
        }

        Some(Token::MnemonicBcsX) => {
            let label = expect_label_reference(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt_with_label!(Op::new_bcs_x(0), label);
        }

        Some(Token::MnemonicBccY) => {
            let label = expect_label_reference(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt_with_label!(Op::new_bcc_y(0), label);
        }

        Some(Token::MnemonicBcsY) => {
            let label = expect_label_reference(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt_with_label!(Op::new_bcs_y(0), label);
        }

        Some(Token::MnemonicShootAim) => {
            let unused = expect_nibble(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_shoot_aim(unused));
        }

        Some(Token::MnemonicChangeMusic) => {
            let music = expect_nibble(lineno, lex)?;
            expect_end(lineno, lex)?;
            add_stmt!(Op::new_change_music(music));
        }

        _ => {
            return Err(AsmError::Parse {
                lineno,
                msg: format!("unexpected token: {}", lex.slice()),
            });
        }
    }

    Ok(())
}

fn expect_label_reference(lineno: usize, lex: &mut Lexer<Token>) -> AsmResult<String> {
    if let Some(Token::LabelReference(label)) = lex.next() {
        Ok(label)
    } else {
        Err(AsmError::Parse {
            lineno,
            msg: format!("expected label reference, but got: {}", lex.slice()),
        })
    }
}

fn expect_dir(lineno: usize, lex: &mut Lexer<Token>) -> AsmResult<Direction> {
    let idx = expect_number(lineno, lex)?;

    if !(0..=0x3F).contains(&idx) {
        return Err(AsmError::Parse {
            lineno,
            msg: format!("invalid direction: {}", idx),
        });
    }

    Ok(Direction::new(idx))
}

fn expect_dir_shoot(lineno: usize, lex: &mut Lexer<Token>) -> AsmResult<Direction> {
    let idx = expect_number(lineno, lex)?;

    if !(0..=0xF).contains(&idx) {
        return Err(AsmError::Parse {
            lineno,
            msg: format!("invalid shooting direction: {}", idx),
        });
    }

    Ok(Direction::new(idx))
}

fn expect_nibble(lineno: usize, lex: &mut Lexer<Token>) -> AsmResult<u8> {
    const RANGE: std::ops::RangeInclusive<u8> = 0..=0xF;

    let idx = expect_number(lineno, lex)?;

    if !RANGE.contains(&idx) {
        return Err(AsmError::Parse {
            lineno,
            msg: format!("operand must be within {:?}: {}", RANGE, idx),
        });
    }

    Ok(idx)
}

fn expect_loop_idx(lineno: usize, lex: &mut Lexer<Token>) -> AsmResult<u8> {
    let idx = expect_number(lineno, lex)?;

    if !(0..=0xF).contains(&idx) || idx == 1 {
        return Err(AsmError::Parse {
            lineno,
            msg: "invalid loop index".to_owned(),
        });
    }

    Ok(idx)
}

fn expect_bool(lineno: usize, lex: &mut Lexer<Token>) -> AsmResult<bool> {
    let n = expect_number(lineno, lex)?;

    if !(0..=1).contains(&n) {
        return Err(AsmError::Parse {
            lineno,
            msg: format!("bool value must be 0 or 1: {}", lex.slice()),
        });
    }

    Ok(n != 0)
}

fn expect_number(lineno: usize, lex: &mut Lexer<Token>) -> AsmResult<u8> {
    if let Some(Token::Number(addr)) = lex.next() {
        Ok(addr)
    } else {
        Err(AsmError::Parse {
            lineno,
            msg: format!("expected number, but got: {}", lex.slice()),
        })
    }
}

fn expect_comma(lineno: usize, lex: &mut Lexer<Token>) -> AsmResult<()> {
    if let Some(Token::Comma) = lex.next() {
        Ok(())
    } else {
        Err(AsmError::Parse {
            lineno,
            msg: format!("expected comma, but got: {}", lex.slice()),
        })
    }
}

fn expect_end(lineno: usize, lex: &mut Lexer<Token>) -> AsmResult<()> {
    if lex.next().is_none() {
        Ok(())
    } else {
        Err(AsmError::Parse {
            lineno,
            msg: format!("expected end, but got: {}", lex.slice()),
        })
    }
}

fn trim_comment(s: &str) -> &str {
    let pos = s.find(';').unwrap_or(s.len());
    &s[..pos]
}

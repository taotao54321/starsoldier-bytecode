use thiserror::Error;

use crate::direction::Direction;

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("incomplete op (opcode={opcode:#04x})")]
    Incomplete { opcode: u8 },

    #[error("undefined op (opcode={opcode:#04x})")]
    Undefined { opcode: u8 },
}

pub type DecodeResult<T> = Result<T, DecodeError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Op {
    Move(Direction),
    Jump(u8),
    SetSleepTimer(u8),
    LoopBegin(u8),
    LoopEnd,
    ShootDirection(Direction),
    SetSprite(u8),
    SetHomingTimer(u8),
    SetInversion(bool, bool),
    SetPosition(u8, u8),

    // ザコの場合、被弾時のジャンプ先を設定する。
    // ボスの場合、HP を設定する。
    // バリアントを分けるとバイナリを見ただけでは逆アセンブルできなくなるので分けない。
    SetJumpOnDamage(u8),

    IncrementSprite,
    DecrementSprite,
    SetPart(u8),
    RandomizeX(u8),
    RandomizeY(u8),
    BccX(u8),
    BcsX(u8),
    BccY(u8),
    BcsY(u8),

    // オペコード 0xC0..=0xCF は全て同じ機能と思われる。
    ShootAim(u8),

    ChangeMusic(u8),
}

impl Op {
    pub fn new_move(dir: Direction) -> Self {
        Self::Move(dir)
    }

    pub fn new_jump(addr: u8) -> Self {
        Self::Jump(addr)
    }

    pub fn new_set_sleep_timer(idx: u8) -> Self {
        assert!((0..=0xF).contains(&idx));
        Self::SetSleepTimer(idx)
    }

    pub fn new_loop_begin(idx: u8) -> Self {
        assert!((0..=0xF).contains(&idx));
        assert_ne!(idx, 1);
        Self::LoopBegin(idx)
    }

    pub fn new_loop_end() -> Self {
        Self::LoopEnd
    }

    pub fn new_shoot_direction(dir: Direction) -> Self {
        assert!((0..=0xF).contains(&dir.index()));
        Self::ShootDirection(dir)
    }

    pub fn new_set_sprite(idx: u8) -> Self {
        assert!((0..=0xF).contains(&idx));
        Self::SetSprite(idx)
    }

    pub fn new_set_homing_timer(idx: u8) -> Self {
        assert!((0..=0xF).contains(&idx));
        Self::SetHomingTimer(idx)
    }

    pub fn new_set_inversion(inv_x: bool, inv_y: bool) -> Self {
        Self::SetInversion(inv_x, inv_y)
    }

    pub fn new_set_position(x: u8, y: u8) -> Self {
        Self::SetPosition(x, y)
    }

    pub fn new_set_jump_on_damage(addr: u8) -> Self {
        Self::SetJumpOnDamage(addr)
    }

    pub fn new_increment_sprite() -> Self {
        Self::IncrementSprite
    }

    pub fn new_decrement_sprite() -> Self {
        Self::DecrementSprite
    }

    pub fn new_set_part(part: u8) -> Self {
        Self::SetPart(part)
    }

    pub fn new_randomize_x(mask: u8) -> Self {
        Self::RandomizeX(mask)
    }

    pub fn new_randomize_y(mask: u8) -> Self {
        Self::RandomizeY(mask)
    }

    pub fn new_bcc_x(addr: u8) -> Self {
        Self::BccX(addr)
    }

    pub fn new_bcs_x(addr: u8) -> Self {
        Self::BcsX(addr)
    }

    pub fn new_bcc_y(addr: u8) -> Self {
        Self::BccY(addr)
    }

    pub fn new_bcs_y(addr: u8) -> Self {
        Self::BcsY(addr)
    }

    pub fn new_shoot_aim(unused: u8) -> Self {
        assert!((0..=0xF).contains(&unused));
        Self::ShootAim(unused)
    }

    pub fn new_change_music(music: u8) -> Self {
        assert!((0..=0xF).contains(&music));
        Self::ChangeMusic(music)
    }

    pub fn len(self) -> usize {
        match self {
            Self::Move(..) => 1,
            Self::Jump(..) => 2,
            Self::SetSleepTimer(..) => 1,
            Self::LoopBegin(..) => 1,
            Self::LoopEnd => 1,
            Self::ShootDirection(..) => 1,
            Self::SetSprite(..) => 1,
            Self::SetHomingTimer(..) => 1,
            Self::SetInversion(..) => 1,
            Self::SetPosition(..) => 3,
            Self::SetJumpOnDamage(..) => 2,
            Self::IncrementSprite => 1,
            Self::DecrementSprite => 1,
            Self::SetPart(..) => 2,
            Self::RandomizeX(..) => 2,
            Self::RandomizeY(..) => 2,
            Self::BccX(..) => 2,
            Self::BcsX(..) => 2,
            Self::BccY(..) => 2,
            Self::BcsY(..) => 2,
            Self::ShootAim(..) => 1,
            Self::ChangeMusic(..) => 1,
        }
    }

    pub fn addr_destination(self) -> Option<u8> {
        match self {
            Self::Jump(addr) => Some(addr),
            Self::SetJumpOnDamage(addr) => Some(addr),
            Self::BccX(addr) => Some(addr),
            Self::BcsX(addr) => Some(addr),
            Self::BccY(addr) => Some(addr),
            Self::BcsY(addr) => Some(addr),
            _ => None,
        }
    }

    pub fn decode(buf: &[u8]) -> DecodeResult<Self> {
        assert!(!buf.is_empty());

        let opcode = buf[0];

        macro_rules! ensure_buf_len {
            ($len:expr) => {{
                if buf.len() < $len {
                    return Err(DecodeError::Incomplete { opcode });
                }
            }};
        }

        match opcode {
            0x00..=0x3F => Ok(Self::new_move(Direction::new(opcode))),
            0x40 => {
                ensure_buf_len!(2);
                let addr = buf[1];
                Ok(Self::new_jump(addr))
            }
            0x41..=0x4F => Ok(Self::new_set_sleep_timer(opcode & 0xF)),
            0x50 | 0x52..=0x5F => Ok(Self::new_loop_begin(opcode & 0xF)),
            0x51 => Ok(Self::new_loop_end()),
            0x60..=0x6F => Ok(Self::new_shoot_direction(Direction::new(opcode & 0xF))),
            0x70..=0x7F => Ok(Self::new_set_sprite(opcode & 0xF)),
            0x80..=0x8F => Ok(Self::new_set_homing_timer(opcode & 0xF)),
            0x90..=0x93 => Ok(Self::new_set_inversion(
                (opcode & 1) != 0,
                (opcode & 2) != 0,
            )),
            0xA0 => {
                ensure_buf_len!(3);
                let x = buf[1];
                let y = buf[2];
                Ok(Self::new_set_position(x, y))
            }
            0xA1 => {
                ensure_buf_len!(2);
                let addr = buf[1];
                Ok(Self::new_set_jump_on_damage(addr))
            }
            0xA2 => Ok(Self::new_increment_sprite()),
            0xA3 => Ok(Self::new_decrement_sprite()),
            0xA4 => {
                ensure_buf_len!(2);
                let part = buf[1];
                Ok(Self::new_set_part(part))
            }
            0xA5 => {
                ensure_buf_len!(2);
                let mask = buf[1];
                Ok(Self::new_randomize_x(mask))
            }
            0xA6 => {
                ensure_buf_len!(2);
                let mask = buf[1];
                Ok(Self::new_randomize_y(mask))
            }
            0xB0 => {
                ensure_buf_len!(2);
                let addr = buf[1];
                Ok(Self::new_bcc_x(addr))
            }
            0xB1 => {
                ensure_buf_len!(2);
                let addr = buf[1];
                Ok(Self::new_bcs_x(addr))
            }
            0xB2 => {
                ensure_buf_len!(2);
                let addr = buf[1];
                Ok(Self::new_bcc_y(addr))
            }
            0xB3 => {
                ensure_buf_len!(2);
                let addr = buf[1];
                Ok(Self::new_bcs_y(addr))
            }
            0xC0..=0xCF => Ok(Self::new_shoot_aim(opcode & 0xF)),
            0xF0..=0xFF => Ok(Self::new_change_music(opcode & 0xF)),
            _ => Err(DecodeError::Undefined { opcode }),
        }
    }

    pub fn encode(self, buf: &mut [u8]) {
        match self {
            Self::Move(dir) => buf[0] = dir.index(),
            Self::Jump(addr) => {
                buf[0] = 0x40;
                buf[1] = addr;
            }
            Self::SetSleepTimer(idx) => buf[0] = 0x40 | idx,
            Self::LoopBegin(idx) => buf[0] = 0x50 | idx,
            Self::LoopEnd => buf[0] = 0x51,
            Self::ShootDirection(dir) => buf[0] = 0x60 | dir.index(),
            Self::SetSprite(idx) => buf[0] = 0x70 | idx,
            Self::SetHomingTimer(idx) => buf[0] = 0x80 | idx,
            Self::SetInversion(inv_x, inv_y) => {
                buf[0] = 0x90 | u8::from(inv_x) | (u8::from(inv_y) << 1);
            }
            Self::SetPosition(x, y) => {
                buf[0] = 0xA0;
                buf[1] = x;
                buf[2] = y;
            }
            Self::SetJumpOnDamage(addr) => {
                buf[0] = 0xA1;
                buf[1] = addr;
            }
            Self::IncrementSprite => buf[0] = 0xA2,
            Self::DecrementSprite => buf[0] = 0xA3,
            Self::SetPart(part) => {
                buf[0] = 0xA4;
                buf[1] = part;
            }
            Self::RandomizeX(mask) => {
                buf[0] = 0xA5;
                buf[1] = mask;
            }
            Self::RandomizeY(mask) => {
                buf[0] = 0xA6;
                buf[1] = mask;
            }
            Self::BccX(addr) => {
                buf[0] = 0xB0;
                buf[1] = addr;
            }
            Self::BcsX(addr) => {
                buf[0] = 0xB1;
                buf[1] = addr;
            }
            Self::BccY(addr) => {
                buf[0] = 0xB2;
                buf[1] = addr;
            }
            Self::BcsY(addr) => {
                buf[0] = 0xB3;
                buf[1] = addr;
            }
            Self::ShootAim(unused) => buf[0] = 0xC0 | unused,
            Self::ChangeMusic(music) => buf[0] = 0xF0 | music,
        }
    }
}

use thiserror::Error;

use crate::direction::Direction;
use crate::op::*;

pub trait Game {
    fn is_second_round(&self) -> bool;
    fn stage(&self) -> u8; // 面 (1..=16)

    fn hero_x(&self) -> u8;
    fn hero_y(&self) -> u8;

    fn rand(&mut self) -> u8;

    fn try_shoot_aim(&mut self, x: u8, y: u8, speed_mask: u8, force_homing: bool);

    fn restore_music(&mut self);
    fn play_sound(&mut self, sound: u8);
}

#[derive(Debug, Error)]
pub enum InterpretError {
    #[error("address {addr:#04X}: decode failed")]
    Decode {
        addr: usize,
        #[source]
        source: DecodeError,
    },
}

pub type InterpretResult<T> = Result<T, InterpretError>;

#[derive(Debug)]
pub struct InterpreterInit {
    pub program: Vec<u8>,
    pub pc: usize,

    pub boss: bool,
    pub difficulty: u8,
    pub shot_with_rank: bool,        // 低ランクでは自機狙い弾を撃たない
    pub accel_shot_with_rank: bool,  // ランクが上がると自機狙い弾が高速化
    pub homing_shot_with_rank: bool, // ランクが上がると自機狙い弾が誘導弾になる
    pub extra_act_with_rank: bool,   // ランクが上がると移動後に再行動する
    pub accel_with_rank: bool,       // ランクが上がると移動スピード増加
    pub rank: u8,

    pub x: u8,
    pub y: u8,
}

impl InterpreterInit {
    pub fn init(self) -> Interpreter {
        assert!((0..=7).contains(&self.rank));

        Interpreter {
            program: self.program,
            pc: self.pc,

            boss: self.boss,
            difficulty: self.difficulty,
            shot_with_rank: self.shot_with_rank,
            accel_shot_with_rank: self.accel_shot_with_rank,
            homing_shot_with_rank: self.homing_shot_with_rank,
            extra_act_with_rank: self.extra_act_with_rank,
            accel_with_rank: self.accel_with_rank,
            rank: self.rank,

            state: EnemyState::Alive,
            x: self.x,
            y: self.y,
            inv_x: false,
            inv_y: false,
            health: 0,
            sprite_idx: 0,
            part: 0,

            sleep_timer: 0,
            homing_timer: 0,
            loop_start_addr: self.pc,
            loop_counter: 0,
            jump_on_damage: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EnemyState {
    Alive,
    Dying,
    Leaving,
}

#[derive(Debug)]
pub struct Interpreter {
    program: Vec<u8>,
    pc: usize,

    boss: bool,
    difficulty: u8,
    shot_with_rank: bool,
    accel_shot_with_rank: bool,
    homing_shot_with_rank: bool,
    extra_act_with_rank: bool,
    accel_with_rank: bool,
    rank: u8,

    state: EnemyState,
    x: u8,
    y: u8,
    inv_x: bool,
    inv_y: bool,
    health: u8,
    sprite_idx: u8,
    part: u8,

    sleep_timer: u8,
    homing_timer: u8,
    loop_start_addr: usize,
    loop_counter: u8,
    jump_on_damage: u8,
}

impl Interpreter {
    pub fn step<G: Game>(&mut self, game: &mut G) -> InterpretResult<()> {
        assert!(matches!(self.state, EnemyState::Alive));

        if self.sleep_timer > 0 {
            self.sleep_timer -= 1;
            return Ok(());
        }

        let mut do_try_homing = true;
        let mut do_try_extra_act = self.extra_act_with_rank;

        loop {
            // ホーミング処理(基本的には1回のみ)
            if do_try_homing && self.homing_timer > 0 {
                self.homing_timer -= 1;
                let dir = Direction::aim((self.x, self.y), (game.hero_x(), game.hero_y()));
                let (dx, dy) = dir.displacement_object();
                self.x = self.x.wrapping_add(dx as u8);
                self.y = self.y.wrapping_add(dy as u8);
                let extra_act = self.clip(game, &mut do_try_extra_act);
                if extra_act {
                    continue;
                } else {
                    return Ok(());
                }
            }
            do_try_homing = false;

            let op = self.fetch()?;

            match op {
                Op::Move(dir) => {
                    // 低速移動は特定条件下で高速化
                    let dir = if (0..=0x1F).contains(&dir.index()) {
                        if self.cond_accel1(game) {
                            Direction::new(dir.index() + 0x10)
                        } else if self.cond_accel2(game) {
                            Direction::new(dir.index() + 0x20)
                        } else {
                            dir
                        }
                    } else {
                        dir
                    };
                    let (dx, dy) = dir.displacement_object();
                    let dx = if self.inv_x { -dx } else { dx };
                    let dy = if self.inv_y { -dy } else { dy };
                    self.x = self.x.wrapping_add(dx as u8);
                    self.y = self.y.wrapping_add(dy as u8);
                    let extra_act = self.clip(game, &mut do_try_extra_act);
                    if !extra_act {
                        return Ok(());
                    }
                }
                Op::Jump(addr) => {
                    self.pc = usize::from(addr);
                }
                Op::SetSleepTimer(idx) => {
                    self.sleep_timer = 4 * idx;
                    return Ok(());
                }
                Op::LoopBegin(idx) => {
                    self.loop_start_addr = self.pc;
                    self.loop_counter = idx;
                }
                Op::LoopEnd => {
                    self.loop_counter = self.loop_counter.wrapping_sub(1);
                    if self.loop_counter > 0 {
                        self.pc = self.loop_start_addr;
                    }
                }
                Op::ShootDirection(_dir) => {
                    todo!();
                }
                Op::SetSprite(idx) => {
                    self.sprite_idx = idx;
                }
                Op::SetHomingTimer(idx) => {
                    self.homing_timer = if idx == 0 { 252 } else { 4 * idx };
                    do_try_homing = true;
                }
                Op::SetInversion(inv_x, inv_y) => {
                    self.inv_x = inv_x;
                    self.inv_y = inv_y;
                }
                Op::SetPosition(x, y) => {
                    self.x = x;
                    self.y = y;
                }
                Op::SetJumpOnDamage(addr) => {
                    assert!(!self.boss);
                    self.jump_on_damage = addr;
                    return Ok(());
                }
                Op::UnsetJumpOnDamage => {
                    assert!(!self.boss);
                    self.jump_on_damage = 0;
                    return Ok(());
                }
                Op::SetHealth(health) => {
                    assert!(self.boss);
                    self.health = health;
                    return Ok(());
                }
                Op::IncrementSprite => {
                    self.sprite_idx += 1;
                }
                Op::DecrementSprite => {
                    self.sprite_idx -= 1;
                }
                Op::SetPart(part) => {
                    self.part = part;
                }
                Op::RandomizeX(mask) => {
                    self.x = (self.x & !mask) | (game.rand() & mask);
                }
                Op::RandomizeY(mask) => {
                    self.y = (self.y & !mask) | (game.rand() & mask);
                }
                Op::BccX(addr) => {
                    if self.x < game.hero_x() {
                        self.pc = usize::from(addr);
                    }
                }
                Op::BcsX(addr) => {
                    if self.x >= game.hero_x() {
                        self.pc = usize::from(addr);
                    }
                }
                Op::BccY(addr) => {
                    if self.y < game.hero_y() {
                        self.pc = usize::from(addr);
                    }
                }
                Op::BcsY(addr) => {
                    if self.y >= game.hero_y() {
                        self.pc = usize::from(addr);
                    }
                }
                Op::ShootAim(_) => {
                    if !self.cond_shoot_aim() {
                        continue;
                    }
                    let (speed_mask, force_homing) = self.shoot_aim_param(game);
                    game.try_shoot_aim(self.x, self.y, speed_mask, force_homing);
                }
                Op::RestoreMusic => {
                    game.restore_music();
                }
                Op::PlaySound(sound) => {
                    game.play_sound(sound);
                }
            }
        }
    }

    pub fn damage<G: Game>(&mut self, _game: &mut G) {
        assert!(matches!(self.state, EnemyState::Alive));

        if self.boss {
            if self.health == 0 {
                self.state = EnemyState::Dying;
                // TODO: 本来は撃破音が鳴る
            } else {
                self.health -= 1;
                // TODO: 本来はダメージ音が鳴る
            }
        } else {
            if self.jump_on_damage == 0 {
                self.state = EnemyState::Dying;
                // TODO: 本来は撃破音が鳴る
            } else {
                self.pc = usize::from(self.jump_on_damage);
                // TODO: 本来はダメージ音が鳴る
            }
        }
    }

    fn fetch(&mut self) -> InterpretResult<Op> {
        let mut op = Op::decode(&self.program[self.pc..]).map_err(|e| InterpretError::Decode {
            addr: self.pc,
            source: e,
        })?;
        if self.boss {
            match op {
                Op::SetJumpOnDamage(addr) => op = Op::SetHealth(addr),
                Op::UnsetJumpOnDamage => op = Op::SetHealth(0),
                _ => {}
            }
        }
        self.pc += op.len();
        Ok(op)
    }

    /// 画面外に出たら消滅させる。
    /// また、do_try_extra_act が真の場合、再行動条件を満たしているか判定する。
    /// 再行動するかどうかを返す。
    fn clip<G: Game>(&mut self, game: &G, do_try_extra_act: &mut bool) -> bool {
        const CLIP_Y_MIN: u8 = 239;

        if self.y >= CLIP_Y_MIN {
            self.state = EnemyState::Leaving;
            return false;
        }

        if *do_try_extra_act && game.stage() >= self.difficulty && self.rank >= 4 {
            *do_try_extra_act = false;
            return true;
        }

        false
    }

    /// 自機狙い弾を撃つ条件を満たしているかどうかを返す。
    fn cond_shoot_aim(&self) -> bool {
        !(self.shot_with_rank && self.rank < 4)
    }

    /// 自機狙い弾のパラメータ (スピード指定マスク, 誘導弾フラグ) を返す。
    fn shoot_aim_param<G: Game>(&self, game: &G) -> (u8, bool) {
        // 誘導弾にする場合、スピード指定マスクは 0 にする。
        if self.homing_shot_with_rank && game.is_second_round() && self.rank == 7 {
            (0, true)
        } else if self.accel_shot_with_rank {
            ((self.rank << 3) & 0x30, false)
        } else {
            (0, false)
        }
    }

    /// 移動スピード1段階増加条件を満たしているかどうかを返す。
    fn cond_accel1<G: Game>(&self, game: &G) -> bool {
        self.accel_with_rank && game.stage() >= self.difficulty && (4..=6).contains(&self.rank)
    }

    /// 移動スピード2段階増加条件を満たしているかどうかを返す。
    fn cond_accel2<G: Game>(&self, game: &G) -> bool {
        self.accel_with_rank && game.stage() >= self.difficulty && self.rank == 7
    }
}

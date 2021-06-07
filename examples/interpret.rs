use macroquad::prelude::*;

use starsoldier_bytecode as bytecode;

const ASM: &str = r#"
        bcc_x L07
        set_position 224, 16
        jump L16
L07:
        set_position 16, 16
L0A:
        set_sprite 1
        loop_begin 4
        move 0x26
        loop_end
        loop_begin 4
        move 0x15
        loop_end
        loop_begin 15
        move 0x14
        move 0x14
        loop_end
        shoot_aim 0
L16:
        set_sprite 0
        loop_begin 4
        move 0x2A
        loop_end
        loop_begin 4
        move 0x1B
        loop_end
        loop_begin 15
        move 0x1C
        move 0x1C
        loop_end
        shoot_aim 0
        jump L0A
"#;

#[derive(Debug)]
struct Game;

impl bytecode::Game for Game {
    fn is_second_round(&self) -> bool {
        false
    }
    fn stage(&self) -> u8 {
        1
    }

    fn hero_x(&self) -> u8 {
        128
    }
    fn hero_y(&self) -> u8 {
        120
    }

    fn rand(&mut self) -> u8 {
        0
    }

    fn try_shoot_aim(&mut self, _x: u8, _y: u8, _speed_mask: u8, _force_homing: bool) {}

    fn restore_music(&mut self) {}
    fn play_sound(&mut self, _sound: u8) {}
}

fn window_conf() -> Conf {
    Conf {
        window_title: "interpret".to_owned(),
        window_width: 512,
        window_height: 480,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() -> eyre::Result<()> {
    let mut game = Game;

    let mut interp = bytecode::InterpreterInit {
        program: bytecode::asm(ASM.as_bytes())?,
        pc: 0,

        boss: false,
        difficulty: 1,
        shot_with_rank: true,
        accel_shot_with_rank: false,
        homing_shot_with_rank: false,
        extra_act_with_rank: false,
        accel_with_rank: false,
        rank: 0,

        x: 120,
        y: 239,
    }
    .init();

    set_camera(&Camera2D::from_display_rect(Rect::new(
        0.0,
        0.0,
        screen_width() / 2.0,
        screen_height() / 2.0,
    )));

    loop {
        clear_background(BLACK);

        if !matches!(interp.state(), bytecode::EnemyState::Alive) {
            break;
        }
        interp.step(&mut game)?;

        let (x, y) = (interp.x(), interp.y());
        //eprintln!("{:?}", (x, y));
        draw_circle(x.into(), y.into(), 2.0, GREEN);

        next_frame().await
    }

    Ok(())
}

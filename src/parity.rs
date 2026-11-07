// Copyright (c) 2026 lachine. AGPL v3 + Commons Clause - see LICENSE.md
//! Vanilla-client parity: packets a real 1.21.4 client sends that azalea omits.

use azalea::app::{App, Plugin};
use azalea::core::tick::GameTick;
use azalea::ecs::prelude::*;
use azalea::entity::{Jumping, LocalEntity};
use azalea::movement::PhysicsState;
use azalea::packet_handling::game::SendPacketEvent;
use azalea::protocol::packets::game::{
    s_client_tick_end::ServerboundClientTickEnd, s_player_input::ServerboundPlayerInput,
    s_player_loaded::ServerboundPlayerLoaded,
};
use azalea::WalkDirection;

pub struct VanillaParityPlugin;

#[derive(Component, Default, Clone, PartialEq)]
struct LastInput(u8);
#[derive(Component)]
struct LoadedSent;

impl Plugin for VanillaParityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            GameTick,
            (send_player_input, send_player_loaded, send_tick_end)
                .chain()
                .after(azalea::movement::send_position),
        );
    }
}

fn input_bits(dir: WalkDirection, jump: bool) -> u8 {
    use WalkDirection::*;
    let (f, b, l, r) = match dir {
        None => (false, false, false, false),
        Forward => (true, false, false, false),
        Backward => (false, true, false, false),
        Left => (false, false, true, false),
        Right => (false, false, false, true),
        ForwardRight => (true, false, false, true),
        ForwardLeft => (true, false, true, false),
        BackwardRight => (false, true, false, true),
        BackwardLeft => (false, true, true, false),
    };
    (f as u8) | (b as u8) << 1 | (l as u8) << 2 | (r as u8) << 3 | (jump as u8) << 4
}

#[allow(clippy::type_complexity)]
fn send_player_input(
    mut q: Query<(Entity, &PhysicsState, &Jumping, Option<&mut LastInput>), With<LocalEntity>>,
    mut cmd: Commands,
    mut out: EventWriter<SendPacketEvent>,
) {
    for (e, phys, jumping, last) in &mut q {
        let bits = input_bits(phys.move_direction, **jumping);
        let changed = match &last {
            Some(l) => l.0 != bits,
            None => bits != 0,
        };
        if changed {
            out.send(SendPacketEvent::new(
                e,
                ServerboundPlayerInput {
                    forward: bits & 1 != 0,
                    backward: bits & 2 != 0,
                    left: bits & 4 != 0,
                    right: bits & 8 != 0,
                    jump: bits & 16 != 0,
                    shift: false,
                    sprint: phys.trying_to_sprint,
                },
            ));
        }
        match last {
            Some(mut l) => l.0 = bits,
            None => {
                cmd.entity(e).insert(LastInput(bits));
            }
        }
    }
}

fn send_player_loaded(
    q: Query<Entity, (With<LocalEntity>, With<azalea::entity::Position>, Without<LoadedSent>)>,
    mut cmd: Commands,
    mut out: EventWriter<SendPacketEvent>,
) {
    for e in &q {
        out.send(SendPacketEvent::new(e, ServerboundPlayerLoaded));
        cmd.entity(e).insert(LoadedSent);
    }
}

fn send_tick_end(q: Query<Entity, With<LocalEntity>>, mut out: EventWriter<SendPacketEvent>) {
    for e in &q {
        out.send(SendPacketEvent::new(e, ServerboundClientTickEnd));
    }
}

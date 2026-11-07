use azalea_buf::AzBuf;
use azalea_protocol_macros::ClientboundGamePacket;

pub use super::c_sound::{SoundHolder, SoundSource};

#[derive(Clone, Debug, AzBuf, ClientboundGamePacket)]
pub struct ClientboundSoundEntity {
    pub sound: SoundHolder,
    pub source: SoundSource,
    #[var]
    pub id: u32,
    pub volume: f32,
    pub pitch: f32,
    pub seed: u64,
}

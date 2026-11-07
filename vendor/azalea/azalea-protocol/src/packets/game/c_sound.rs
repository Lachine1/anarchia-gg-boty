use std::io::{Cursor, Write};

use azalea_buf::{AzBuf, AzaleaRead, AzaleaReadVar, AzaleaWrite, AzaleaWriteVar, BufReadError};
use azalea_core::resource_location::ResourceLocation;
use azalea_protocol_macros::ClientboundGamePacket;

#[derive(Clone, Debug, AzBuf, ClientboundGamePacket)]
pub struct ClientboundSound {
    pub sound: SoundHolder,
    pub source: SoundSource,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub volume: f32,
    pub pitch: f32,
    pub seed: u64,
}

/// `Holder<SoundEvent>`: varint 0 = inline definition, otherwise registry id + 1.
#[derive(Clone, Debug)]
pub enum SoundHolder {
    Registry(u32),
    Inline { location: ResourceLocation, fixed_range: Option<f32> },
}
impl AzaleaRead for SoundHolder {
    fn azalea_read(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        let id = u32::azalea_read_var(buf)?;
        if id == 0 {
            Ok(SoundHolder::Inline {
                location: ResourceLocation::azalea_read(buf)?,
                fixed_range: Option::<f32>::azalea_read(buf)?,
            })
        } else {
            Ok(SoundHolder::Registry(id - 1))
        }
    }
}
impl AzaleaWrite for SoundHolder {
    fn azalea_write(&self, buf: &mut impl Write) -> Result<(), std::io::Error> {
        match self {
            SoundHolder::Registry(id) => (id + 1).azalea_write_var(buf),
            SoundHolder::Inline { location, fixed_range } => {
                0u32.azalea_write_var(buf)?;
                location.azalea_write(buf)?;
                fixed_range.azalea_write(buf)
            }
        }
    }
}

#[derive(AzBuf, Clone, Copy, Debug)]
pub enum SoundSource {
    Master = 0,
    Music = 1,
    Records = 2,
    Weather = 3,
    Blocks = 4,
    Hostile = 5,
    Neutral = 6,
    Players = 7,
    Ambient = 8,
    Voice = 9,
}

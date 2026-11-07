use azalea_buf::AzBuf;
use azalea_core::position::BlockPos;
use azalea_inventory::ItemStack;
use azalea_registry::ParticleKind;
use bevy_ecs::component::Component;

// the order of this enum must be kept in sync with ParticleKind, otherwise
// we get errors parsing particles.
/// A [`ParticleKind`] with data potentially attached to it.
#[derive(Component, Clone, Debug, AzBuf, Default)]
pub enum Particle {
    #[default]
    AngryVillager,
    Block(BlockParticle),
    BlockMarker(BlockParticle),
    Bubble,
    Cloud,
    Crit,
    DamageIndicator,
    DragonBreath,
    DrippingLava,
    FallingLava,
    LandingLava,
    DrippingWater,
    FallingWater,
    Dust(DustParticle),
    DustColorTransition(DustColorTransitionParticle),
    Effect,
    ElderGuardian,
    EnchantedHit,
    Enchant,
    EndRod,
    EntityEffect(ColorParticle),
    ExplosionEmitter,
    Explosion,
    Gust,
    SmallGust,
    GustEmitterLarge,
    GustEmitterSmall,
    SonicBoom,
    FallingDust(BlockParticle),
    Firework,
    Fishing,
    Flame,
    Infested,
    CherryLeaves,
    PaleOakLeaves,
    SculkSoul,
    SculkCharge(SculkChargeParticle),
    SculkChargePop,
    SoulFireFlame,
    Soul,
    Flash,
    HappyVillager,
    Composter,
    Heart,
    InstantEffect,
    Item(ItemParticle),
    Vibration(VibrationParticle),
    Trail(TrailParticle),
    ItemSlime,
    ItemCobweb,
    ItemSnowball,
    LargeSmoke,
    Lava,
    Mycelium,
    Note,
    Poof,
    Portal,
    Rain,
    Smoke,
    WhiteSmoke,
    Sneeze,
    Spit,
    SquidInk,
    SweepAttack,
    TotemOfUndying,
    Underwater,
    Splash,
    Witch,
    BubblePop,
    CurrentDown,
    BubbleColumnUp,
    Nautilus,
    Dolphin,
    CampfireCosySmoke,
    CampfireSignalSmoke,
    DrippingHoney,
    FallingHoney,
    LandingHoney,
    FallingNectar,
    FallingSporeBlossom,
    Ash,
    CrimsonSpore,
    WarpedSpore,
    SporeBlossomAir,
    DrippingObsidianTear,
    FallingObsidianTear,
    LandingObsidianTear,
    ReversePortal,
    WhiteAsh,
    SmallFlame,
    Snowflake,
    DrippingDripstoneLava,
    FallingDripstoneLava,
    DrippingDripstoneWater,
    FallingDripstoneWater,
    GlowSquidInk,
    Glow,
    WaxOn,
    WaxOff,
    ElectricSpark,
    Scrape,
    Shriek(ShriekParticle),
    EggCrack,
    DustPlume,
    TrialSpawnerDetection,
    TrialSpawnerDetectionOminous,
    VaultConnection,
    DustPillar(BlockParticle),
    OminousSpawning,
    RaidOmen,
    TrialOmen,
    BlockCrumble(BlockParticle),
}

impl From<ParticleKind> for Particle {
    /// Convert a particle kind into particle data. If the particle has data
    /// attached (like block particles), then it's set to the default.
    fn from(kind: ParticleKind) -> Self {
        // this is mostly just here so it fails to compile when a new particle is added
        // to ParticleKind, since `Particle` has to be updated manually
        match kind {
            ParticleKind::AngryVillager => Self::AngryVillager,
            ParticleKind::Block => Self::Block(BlockParticle::default()),
            ParticleKind::BlockMarker => Self::BlockMarker(BlockParticle::default()),
            ParticleKind::Bubble => Self::Bubble,
            ParticleKind::Cloud => Self::Cloud,
            ParticleKind::Crit => Self::Crit,
            ParticleKind::DamageIndicator => Self::DamageIndicator,
            ParticleKind::DragonBreath => Self::DragonBreath,
            ParticleKind::DrippingLava => Self::DrippingLava,
            ParticleKind::FallingLava => Self::FallingLava,
            ParticleKind::LandingLava => Self::LandingLava,
            ParticleKind::DrippingWater => Self::DrippingWater,
            ParticleKind::FallingWater => Self::FallingWater,
            ParticleKind::Dust => Self::Dust(DustParticle::default()),
            ParticleKind::DustColorTransition => {
                Self::DustColorTransition(DustColorTransitionParticle::default())
            }
            ParticleKind::Effect => Self::Effect,
            ParticleKind::ElderGuardian => Self::ElderGuardian,
            ParticleKind::EnchantedHit => Self::EnchantedHit,
            ParticleKind::Enchant => Self::Enchant,
            ParticleKind::EndRod => Self::EndRod,
            ParticleKind::EntityEffect => Self::EntityEffect(ColorParticle::default()),
            ParticleKind::ExplosionEmitter => Self::ExplosionEmitter,
            ParticleKind::Explosion => Self::Explosion,
            ParticleKind::Gust => Self::Gust,
            ParticleKind::SonicBoom => Self::SonicBoom,
            ParticleKind::FallingDust => Self::FallingDust(BlockParticle::default()),
            ParticleKind::Firework => Self::Firework,
            ParticleKind::Fishing => Self::Fishing,
            ParticleKind::Flame => Self::Flame,
            ParticleKind::CherryLeaves => Self::CherryLeaves,
            ParticleKind::PaleOakLeaves => Self::PaleOakLeaves,
            ParticleKind::SculkSoul => Self::SculkSoul,
            ParticleKind::SculkCharge => Self::SculkCharge(SculkChargeParticle::default()),
            ParticleKind::SculkChargePop => Self::SculkChargePop,
            ParticleKind::SoulFireFlame => Self::SoulFireFlame,
            ParticleKind::Soul => Self::Soul,
            ParticleKind::Flash => Self::Flash,
            ParticleKind::HappyVillager => Self::HappyVillager,
            ParticleKind::Composter => Self::Composter,
            ParticleKind::Heart => Self::Heart,
            ParticleKind::InstantEffect => Self::InstantEffect,
            ParticleKind::Item => Self::Item(ItemParticle::default()),
            ParticleKind::Vibration => Self::Vibration(VibrationParticle::default()),
            ParticleKind::ItemSlime => Self::ItemSlime,
            ParticleKind::ItemSnowball => Self::ItemSnowball,
            ParticleKind::LargeSmoke => Self::LargeSmoke,
            ParticleKind::Lava => Self::Lava,
            ParticleKind::Mycelium => Self::Mycelium,
            ParticleKind::Note => Self::Note,
            ParticleKind::Poof => Self::Poof,
            ParticleKind::Portal => Self::Portal,
            ParticleKind::Rain => Self::Rain,
            ParticleKind::Smoke => Self::Smoke,
            ParticleKind::WhiteSmoke => Self::WhiteSmoke,
            ParticleKind::Sneeze => Self::Sneeze,
            ParticleKind::Spit => Self::Spit,
            ParticleKind::SquidInk => Self::SquidInk,
            ParticleKind::SweepAttack => Self::SweepAttack,
            ParticleKind::TotemOfUndying => Self::TotemOfUndying,
            ParticleKind::Underwater => Self::Underwater,
            ParticleKind::Splash => Self::Splash,
            ParticleKind::Witch => Self::Witch,
            ParticleKind::BubblePop => Self::BubblePop,
            ParticleKind::CurrentDown => Self::CurrentDown,
            ParticleKind::BubbleColumnUp => Self::BubbleColumnUp,
            ParticleKind::Nautilus => Self::Nautilus,
            ParticleKind::Dolphin => Self::Dolphin,
            ParticleKind::CampfireCosySmoke => Self::CampfireCosySmoke,
            ParticleKind::CampfireSignalSmoke => Self::CampfireSignalSmoke,
            ParticleKind::DrippingHoney => Self::DrippingHoney,
            ParticleKind::FallingHoney => Self::FallingHoney,
            ParticleKind::LandingHoney => Self::LandingHoney,
            ParticleKind::FallingNectar => Self::FallingNectar,
            ParticleKind::FallingSporeBlossom => Self::FallingSporeBlossom,
            ParticleKind::Ash => Self::Ash,
            ParticleKind::CrimsonSpore => Self::CrimsonSpore,
            ParticleKind::WarpedSpore => Self::WarpedSpore,
            ParticleKind::SporeBlossomAir => Self::SporeBlossomAir,
            ParticleKind::DrippingObsidianTear => Self::DrippingObsidianTear,
            ParticleKind::FallingObsidianTear => Self::FallingObsidianTear,
            ParticleKind::LandingObsidianTear => Self::LandingObsidianTear,
            ParticleKind::ReversePortal => Self::ReversePortal,
            ParticleKind::WhiteAsh => Self::WhiteAsh,
            ParticleKind::SmallFlame => Self::SmallFlame,
            ParticleKind::Snowflake => Self::Snowflake,
            ParticleKind::DrippingDripstoneLava => Self::DrippingDripstoneLava,
            ParticleKind::FallingDripstoneLava => Self::FallingDripstoneLava,
            ParticleKind::DrippingDripstoneWater => Self::DrippingDripstoneWater,
            ParticleKind::FallingDripstoneWater => Self::FallingDripstoneWater,
            ParticleKind::GlowSquidInk => Self::GlowSquidInk,
            ParticleKind::Glow => Self::Glow,
            ParticleKind::WaxOn => Self::WaxOn,
            ParticleKind::WaxOff => Self::WaxOff,
            ParticleKind::ElectricSpark => Self::ElectricSpark,
            ParticleKind::Scrape => Self::Scrape,
            ParticleKind::Shriek => Self::Shriek(ShriekParticle::default()),
            ParticleKind::EggCrack => Self::EggCrack,
            ParticleKind::DustPlume => Self::DustPlume,
            ParticleKind::SmallGust => Self::SmallGust,
            ParticleKind::GustEmitterLarge => Self::GustEmitterLarge,
            ParticleKind::GustEmitterSmall => Self::GustEmitterSmall,
            ParticleKind::Infested => Self::Infested,
            ParticleKind::ItemCobweb => Self::ItemCobweb,
            ParticleKind::TrialSpawnerDetection => Self::TrialSpawnerDetection,
            ParticleKind::TrialSpawnerDetectionOminous => Self::TrialSpawnerDetectionOminous,
            ParticleKind::VaultConnection => Self::VaultConnection,
            ParticleKind::DustPillar => Self::DustPillar(BlockParticle::default()),
            ParticleKind::OminousSpawning => Self::OminousSpawning,
            ParticleKind::RaidOmen => Self::RaidOmen,
            ParticleKind::TrialOmen => Self::TrialOmen,
            ParticleKind::Trail => Self::Trail(TrailParticle::default()),
            ParticleKind::BlockCrumble => Self::BlockCrumble(BlockParticle::default()),
        }
    }
}

#[derive(Debug, Clone, AzBuf, Default)]
pub struct BlockParticle {
    #[var]
    pub block_state: i32,
}
// 1.21.4 wire layouts (colors are packed ARGB ints)
#[derive(Debug, Clone, AzBuf, Default)]
pub struct DustParticle {
    pub color: i32,
    pub scale: f32,
}

#[derive(Debug, Clone, AzBuf, Default)]
pub struct DustColorTransitionParticle {
    pub from_color: i32,
    pub to_color: i32,
    pub scale: f32,
}

#[derive(Debug, Clone, AzBuf, Default)]
pub struct ColorParticle {
    pub color: i32,
}

#[derive(Debug, Clone, AzBuf, Default)]
pub struct TrailParticle {
    pub target: azalea_core::position::Vec3,
    pub color: i32,
    #[var]
    pub duration: i32,
}

#[derive(Debug, Clone, AzBuf, Default)]
pub struct ItemParticle {
    pub item: ItemStack,
}

#[derive(Debug, Clone, Default)]
pub enum VibrationParticle {
    #[default]
    None,
    Block { pos: BlockPos, ticks: u32 },
    Entity { id: u32, eye_height: f32, ticks: u32 },
}
impl azalea_buf::AzaleaRead for VibrationParticle {
    fn azalea_read(buf: &mut std::io::Cursor<&[u8]>) -> Result<Self, azalea_buf::BufReadError> {
        use azalea_buf::AzaleaReadVar;
        let kind = u32::azalea_read_var(buf)?;
        Ok(match kind {
            0 => {
                let pos = BlockPos::azalea_read(buf)?;
                VibrationParticle::Block { pos, ticks: u32::azalea_read_var(buf)? }
            }
            _ => {
                let id = u32::azalea_read_var(buf)?;
                let eye_height = f32::azalea_read(buf)?;
                VibrationParticle::Entity { id, eye_height, ticks: u32::azalea_read_var(buf)? }
            }
        })
    }
}
impl azalea_buf::AzaleaWrite for VibrationParticle {
    fn azalea_write(&self, buf: &mut impl std::io::Write) -> Result<(), std::io::Error> {
        use azalea_buf::AzaleaWriteVar;
        match self {
            VibrationParticle::None | VibrationParticle::Block { .. } => {
                let (pos, ticks) = match self { VibrationParticle::Block { pos, ticks } => (*pos, *ticks), _ => (BlockPos::default(), 0) };
                0u32.azalea_write_var(buf)?;
                pos.azalea_write(buf)?;
                ticks.azalea_write_var(buf)
            }
            VibrationParticle::Entity { id, eye_height, ticks } => {
                1u32.azalea_write_var(buf)?;
                id.azalea_write_var(buf)?;
                eye_height.azalea_write(buf)?;
                ticks.azalea_write_var(buf)
            }
        }
    }
}

#[derive(Debug, Clone, AzBuf, Default)]
pub struct SculkChargeParticle {
    pub roll: f32,
}

#[derive(Debug, Clone, AzBuf, Default)]
pub struct ShriekParticle {
    #[var]
    pub delay: i32, // The time in ticks before the particle is displayed
}

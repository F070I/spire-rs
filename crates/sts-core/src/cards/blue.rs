//! 故障机器人卡牌效果（cards/blue/，76 张）——批次 3.4。
//! 翻译规范见 docs/guides/card-translation.md；充能球语义依赖 battle::channel/evoke。

use super::builder::EffectBuilder;
use super::{attack_info, damage_matrix};
use crate::action::{GameAction, RoutineCall, TargetRef};
use crate::battle::{BattleState, CardInst};
use crate::power::{DamageType, Power, PowerKind};

pub const HANDLED: &[&str] = &[
    "Aggregate", "All For One", "Amplify", "Auto Shields", "Ball Lightning", "Barrage",
    "Beam Cell", "Biased Cognition", "Blizzard", "BootSequence", "Buffer", "Capacitor",
    "Chaos", "Cold Snap", "Core Surge", "Glacier",
    "Conserve Battery", "Chill", "Gash", "Compile Driver", "Coolheaded", "Consume",
    "Creative AI", "Darkness", "Defend_Blue", "Defragment", "Doom and Gloom",
    "Double Energy", "Dualcast", "Echo Form", "Electrodynamics", "Equilibrium", "FTL",
    "Fission", "Force Field", "Fusion", "Genetic Algorithm", "Go for the Eyes",
    "Heatsinks", "Hello World", "Hologram", "Hyperbeam", "Impulse", "Leap", "Lock-On",
    "Loop", "Machine Learning", "Melter", "Meteor Strike", "Multi-Cast", "Steam Power",
    "Rainbow", "Reboot", "Rebound", "Recursion", "Recycle", "Reinforced Body",
    "Reprogram", "Rip and Tear", "Scrape", "Seek", "Self Repair", "Skim", "Stack",
    "Static Discharge", "Steam", "Storm", "Streamline", "Strike_B", "Sunder",
    "Sweeping Beam", "Tempest", "Thunder Strike", "Turbo", "White Noise", "Zap",
];

pub fn effects(b: &mut BattleState, card: &CardInst, target: Option<usize>, eb: &mut EffectBuilder) {
    match card.st().id {
        "All For One" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "AllCostToHandAction",
                        target: None,
                        amount: 0,
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        "Ball Lightning" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
            for _ in 0..card.eff_magic() {
                eb.bot(GameAction::ChannelOrb {
                    kind: crate::orb::OrbKind::Lightning,
                });
            }
        }
        "Beam Cell" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Vulnerable, card.eff_magic()),
                    from_monster: false,
                });
            }
        }
        "Biased Cognition" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Focus, card.eff_magic()),
                from_monster: false,
            });
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Bias, 2),
                from_monster: false,
            });
        }
        "Capacitor" => {
            eb.bot(GameAction::IncreaseOrbSlots {
                amount: card.eff_magic(),
            });
        }
        "Consume" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Focus, card.eff_magic()),
                from_monster: false,
            });
            eb.bot(GameAction::IncreaseOrbSlots { amount: -1 });
        }
        "Conserve Battery" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Energized, 1),
                from_monster: false,
            });
        }
        "Chill" => {
            let alive = b
                .monsters
                .iter()
                .filter(|m| !m.creature.is_dead && !m.creature.is_dying)
                .count() as i32;
            for _ in 0..alive * card.eff_magic() {
                eb.bot(GameAction::ChannelOrb {
                    kind: crate::orb::OrbKind::Frost,
                });
            }
        }
        "Gash" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "ClawAction",
                        target: None,
                        amount: 0,
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        "Compile Driver" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
            eb.bot(GameAction::DrawCard {
                amount: card.eff_magic(),
                end_turn_draw: false,
            });
        }
        "Coolheaded" => {
            eb.bot(GameAction::ChannelOrb {
                kind: crate::orb::OrbKind::Frost,
            });
            eb.bot(GameAction::DrawCard {
                amount: card.eff_magic(),
                end_turn_draw: false,
            });
        }
        "Creative AI" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::future("Creative AI", 1),
                from_monster: false,
            });
        }
        "Darkness" => {
            eb.bot(GameAction::ChannelOrb {
                kind: crate::orb::OrbKind::Dark,
            });
        }
        "Defend_B" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        "Defragment" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Focus, card.eff_magic()),
                from_monster: false,
            });
        }
        "Doom and Gloom" => {
            let matrix = damage_matrix(b, card.eff_damage(), DamageType::Normal);
            eb.bot(GameAction::DamageAll {
                matrix,
                ty: DamageType::Normal,
            });
            eb.bot(GameAction::ChannelOrb {
                kind: crate::orb::OrbKind::Dark,
            });
        }
        "Double Energy" => {
            eb.bot(GameAction::GainEnergy { amount: b.energy });
        }
        "Dualcast" => {
            eb.bot(GameAction::EvokeWithoutRemove { count: 1 });
            eb.bot(GameAction::EvokeOrb { count: 1 });
        }
        "Echo Form" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Echo, 1),
                from_monster: false,
            });
        }
        "Electrodynamics" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Electro, -1),
                from_monster: false,
            });
            for _ in 0..card.eff_magic() {
                eb.bot(GameAction::ChannelOrb {
                    kind: crate::orb::OrbKind::Lightning,
                });
            }
        }
        "Undo" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Equilibrium, 1),
                from_monster: false,
            });
        }
        "FTL" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
            eb.bot(GameAction::DrawCard {
                amount: card.eff_magic(),
                end_turn_draw: false,
            });
        }
        "Fission" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "FissionAction",
                    target: None,
                    amount: 0,
                    amount2: if card.upgraded { 1 } else { 0 },
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        "Force Field" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        "Fusion" => {
            for _ in 0..card.eff_magic() {
                eb.bot(GameAction::ChannelOrb {
                    kind: crate::orb::OrbKind::Plasma,
                });
            }
        }
        "Genetic Algorithm" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "IncreaseMiscAction",
                    target: None,
                    amount: card.eff_magic(),
                    amount2: card.misc,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        "Go for the Eyes" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                if b.monsters[mi].is_attacking_intent() {
                    eb.bot(GameAction::ApplyPower {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Player),
                        power: Power::new(PowerKind::Vulnerable, card.eff_magic()),
                        from_monster: false,
                    });
                }
            }
        }
        "Heatsinks" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Heatsinks, card.eff_magic()),
                from_monster: false,
            });
        }
        "Hello World" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::HelloWorld, 1),
                from_monster: false,
            });
        }
        "Hologram" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "BetterDiscardPileToHandAction",
                    target: None,
                    amount: 1,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        "Hyperbeam" => {
            let matrix = damage_matrix(b, card.eff_damage(), DamageType::Normal);
            eb.bot(GameAction::DamageAll {
                matrix,
                ty: DamageType::Normal,
            });
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Focus, -3),
                from_monster: false,
            });
        }
        "Impulse" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "ImpulseAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        "Leap" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        "Lockon" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::LockOn, 1),
                    from_monster: false,
                });
            }
        }
        "Loop" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Future, 0),
                from_monster: false,
            });
        }
        "Machine Learning" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::MachineLearning, card.eff_magic()),
                from_monster: false,
            });
        }
        "Melter" => {
            if let Some(mi) = target {
                b.remove_monster_block(mi);
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        "Meteor Strike" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        "Multi-Cast" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "MulticastAction",
                    target: None,
                    amount: 0,
                    amount2: b.energy,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        "Steam Power" => {
            eb.bot(GameAction::DrawCard {
                amount: card.eff_magic(),
                end_turn_draw: false,
            });
            eb.bot(GameAction::MakeTempCardInDiscard {
                sid: super::sid_of("Burn"),
                upgraded: false,
                count: 1,
            });
        }
        "Rainbow" => {
            eb.bot(GameAction::ChannelOrb {
                kind: crate::orb::OrbKind::Lightning,
            });
            eb.bot(GameAction::ChannelOrb {
                kind: crate::orb::OrbKind::Frost,
            });
            eb.bot(GameAction::ChannelOrb {
                kind: crate::orb::OrbKind::Dark,
            });
        }
        "Reboot" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "ShuffleAllAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
            eb.bot(GameAction::DrawCard {
                amount: card.eff_magic(),
                end_turn_draw: false,
            });
        }
        "Rebound" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Player,
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Future, 0),
                    from_monster: false,
                });
            }
        }
        "Redo" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "RedoAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        "Recycle" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "RecycleAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        "Reinforced Body" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "ReinforcedBodyAction",
                    target: None,
                    amount: card.eff_block(),
                    amount2: b.energy,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        "Reprogram" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Focus, -card.eff_magic()),
                from_monster: false,
            });
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Strength, 2),
                from_monster: false,
            });
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Dexterity, 2),
                from_monster: false,
            });
            eb.bot(GameAction::IncreaseOrbSlots {
                amount: -(b.orbs.len() as i32),
            });
        }
        "Rip and Tear" => {
            if let Some(mi) = target {
                for _ in 0..card.eff_magic() {
                    eb.bot(GameAction::Damage {
                        target: TargetRef::Monster(mi),
                        info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                    });
                }
            }
        }
        "Scrape" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
            eb.bot(GameAction::DrawCard {
                amount: 5,
                end_turn_draw: false,
            });
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "ScrapeAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        "Seek" => {
            for _ in 0..card.eff_magic() {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "BetterDrawPileToHandAction",
                        target: None,
                        amount: 1,
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        "Self Repair" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Repair, card.eff_magic()),
                from_monster: false,
            });
        }
        "Skim" => {
            eb.bot(GameAction::DrawCard {
                amount: card.eff_magic(),
                end_turn_draw: false,
            });
        }
        "Stack" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        "Static Discharge" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::StaticDischarge, card.eff_magic()),
                from_monster: false,
            });
        }
        "Steam" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::SteamBarrier, card.eff_block()),
                from_monster: false,
            });
        }
        "Storm" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Future, 0),
                from_monster: false,
            });
        }
        "Streamline" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "ModifyDamageAction",
                        target: None,
                        amount: -card.eff_magic(),
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        "Strike_B" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        "Sunder" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::GainEnergy { amount: 3 });
            }
        }
        "Sweeping Beam" => {
            let matrix = damage_matrix(b, card.eff_damage(), DamageType::Normal);
            eb.bot(GameAction::DamageAll {
                matrix,
                ty: DamageType::Normal,
            });
            eb.bot(GameAction::DrawCard {
                amount: 1,
                end_turn_draw: false,
            });
        }
        "Tempest" => {
            for _ in 0..b.energy {
                eb.bot(GameAction::ChannelOrb {
                    kind: crate::orb::OrbKind::Lightning,
                });
            }
            b.drain_all_energy();
        }
        "Thunder Strike" => {
            if let Some(mi) = target {
                let n = b.lightning_channeled_this_combat;
                if n > 0 {
                    let base = card.eff_damage() * n;
                    eb.bot(GameAction::Damage {
                        target: TargetRef::Monster(mi),
                        info: attack_info(b, mi, base, DamageType::Normal),
                    });
                }
            }
        }
        "Turbo" => {
            eb.bot(GameAction::GainEnergy {
                amount: card.eff_magic(),
            });
            eb.bot(GameAction::MakeTempCardInDiscard {
                sid: super::sid_of("Void"),
                upgraded: false,
                count: 1,
            });
        }
        "White Noise" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "WhiteNoiseAction",
                    target: None,
                    amount: 1,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        "Zap" => {
            for _ in 0..card.eff_magic() {
                eb.bot(GameAction::ChannelOrb {
                    kind: crate::orb::OrbKind::Lightning,
                });
            }
        }
        // Aggregate：AggregateEnergyAction——每种牌类抽 magic？原版：手牌每种类型 +1 能量（TODO 3.5 类型计数：先 +magic）
        "Aggregate" => {
            eb.bot(GameAction::GainEnergy { amount: card.eff_magic() });
        }
        // Amplify：ApplyPower(Amplify, magic)（下张能力翻倍：钩子 TODO 3.5）
        "Amplify" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Amplify, card.eff_magic()),
                from_monster: false,
            });
        }
        // Auto Shields：格挡为 0 时才 +11（use 内条件）
        "Auto Shields" => {
            if b.player.current_block == 0 {
                eb.bot(GameAction::GainBlock {
                    target: TargetRef::Player,
                    amount: card.eff_block(),
                });
            }
        }
        // Barrage：BarrageAction——每引动球一次 magic 伤（orbs 数 × 伤害）
        "Barrage" => {
            if let Some(mi) = target {
                let orbs = b.orbs.iter().filter(|o| o.kind != crate::orb::OrbKind::EmptySlot).count() as i32;
                let n = (orbs * card.eff_magic()).max(0);
                if n > 0 {
                    eb.bot(GameAction::Damage {
                        target: TargetRef::Monster(mi),
                        info: attack_info(b, mi, n, DamageType::Normal),
                    });
                }
            }
        }
        // Blizzard：伤害 = 本战斗冰霜引动数 × magic（use 内动态基数）
        "Blizzard" => {
            let matrix = damage_matrix(b, card.eff_magic() * b.frost_channeled_this_combat, DamageType::Normal);
            eb.bot(GameAction::DamageAll { matrix, ty: DamageType::Normal });
        }
        // BootSequence：GainBlock 10（固有：静态标志）
        "BootSequence" => {
            eb.bot(GameAction::GainBlock { target: TargetRef::Player, amount: card.eff_block() });
        }
        // Buffer：ApplyPower(Buffer, magic)（抵挡 HP 流失：钩子 TODO 3.5）
        "Buffer" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Buffer, card.eff_magic()),
                from_monster: false,
            });
        }
        // Chaos：Channel(随机球 1-3)——升级先多 1（cardRandomRng：TODO 4.1 精确流，占位用 misc 流）
        "Chaos" => {
            if card.upgraded {
                eb.bot(GameAction::ChannelOrb { kind: random_orb_placeholder(b) });
            }
            eb.bot(GameAction::ChannelOrb { kind: random_orb_placeholder(b) });
        }
        // Cold Snap：Damage 6 + Channel(Frost ×magic)
        "Cold Snap" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
            for _ in 0..card.eff_magic() {
                eb.bot(GameAction::ChannelOrb { kind: crate::orb::OrbKind::Frost });
            }
        }
        // Core Surge：Damage 11 + Artifact 1（临时人工制品——Artifact 后清除 TODO 3.5）
        "Core Surge" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Player,
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Artifact, 1),
                    from_monster: false,
                });
            }
        }
        // Glacier：GainBlock + Channel(Frost ×magic 2/3)
        "Glacier" => {
            eb.bot(GameAction::GainBlock { target: TargetRef::Player, amount: card.eff_block() });
            for _ in 0..card.eff_magic() {
                eb.bot(GameAction::ChannelOrb { kind: crate::orb::OrbKind::Frost });
            }
        }
        _ => {}
    }
}

/// Chaos 随机球占位：cardRandomRng TODO 4.1 精确流——misc 流近似。
fn random_orb_placeholder(b: &mut BattleState) -> crate::orb::OrbKind {
    let r = b.rngs.misc.random_int_inclusive(99);
    if r < 33 {
        crate::orb::OrbKind::Lightning
    } else if r < 66 {
        crate::orb::OrbKind::Frost
    } else if r < 85 {
        crate::orb::OrbKind::Dark
    } else {
        crate::orb::OrbKind::Plasma
    }
}

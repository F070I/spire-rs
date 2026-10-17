        // All For One：Damage + AllCostToHandAction（本战斗弃过的牌 0 费进手——计数 TODO 3.5）
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
        // Ball Lightning：Damage + Channel(Lightning)
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
        // Beam Cell：Damage 3 + Vulnerable 2（apply 切片伤害）
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
        // Biased Cognition：Focus +4 + Bias（每回合 -2：钩子 TODO 3.5）
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
        // Capacitor：球槽 +magic（3）
        "Capacitor" => {
            eb.bot(GameAction::IncreaseOrbSlots {
                amount: card.eff_magic(),
            });
        }
        // Consume：Focus +magic + 球槽 -1（负值 IncreaseOrbSlots）
        "Consume" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Focus, card.eff_magic()),
                from_monster: false,
            });
            eb.bot(GameAction::IncreaseOrbSlots { amount: -1 });
        }
        // Conserve Battery（Charge Battery）：GainBlock + Energized(1)
        "Charge Battery" => {
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
        // Chill：每存活怪 ×magic 支冰霜
        "Chill" => {
            let alive = b.monsters.iter().filter(|m| !m.creature.is_dead && !m.creature.is_dying).count() as i32;
            for _ in 0..alive * card.eff_magic() {
                eb.bot(GameAction::ChannelOrb {
                    kind: crate::orb::OrbKind::Frost,
                });
            }
        }
        // Claw：Damage + 全部 Claw +2（ClawAction 全牌组 Claw 递增——misc 钩子 TODO 3.5）
        "Claw" => {
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
        // Compile Driver：Damage + CompileDriverAction（每 unique stack 抽 1——计数 TODO 3.5，先抽 magic）
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
        // Coolheaded：Channel(Frost) + DrawCard(magic 1/2)
        "Coolheaded" => {
            eb.bot(GameAction::ChannelOrb {
                kind: crate::orb::OrbKind::Frost,
            });
            eb.bot(GameAction::DrawCard {
                amount: card.eff_magic(),
                end_turn_draw: false,
            });
        }
        // Creative AI：ApplyPower(CreativeAI, magic)（每回合加随机能力——TODO 4.1）
        "Creative AI" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Future, 0),
                from_monster: false,
            });
        }
        // Darkness：Channel(Dark)（升级 + DarkImpulse：本回合暗球引动——TODO 3.8 例程）
        "Darkness" => {
            eb.bot(GameAction::ChannelOrb {
                kind: crate::orb::OrbKind::Dark,
            });
        }
        // Defend_Blue：GainBlock（基础卡显式化）
        "Defend_Blue" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        // Defragment：Focus +magic
        "Defragment" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Focus, card.eff_magic()),
                from_monster: false,
            });
        }
        // Doom and Gloom：DamageAll + Channel(Dark)（全体各一暗球——原版每怪 channel，简化为一球 TODO 3.8 对齐）
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
        // Double Energy：DoubleEnergyAction——能量翻倍（钳 999）
        "Double Energy" => {
            eb.bot(GameAction::GainEnergy {
                amount: b.energy,
            });
        }
        // Dualcast：Animate+EvokeWithoutRemove(1) + Animate+Evoke(1)
        "Dualcast" => {
            eb.bot(GameAction::EvokeWithoutRemove { count: 1 });
            eb.bot(GameAction::EvokeOrb { count: 1 });
        }
        // Echo Form：ApplyPower(Echo, 1)（每回合首卡翻倍：钩子 TODO 3.5）
        "Echo Form" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Echo, 1),
                from_monster: false,
            });
        }
        // Electrodynamics：ElectroPower（闪电全体 TODO 3.8）+ Channel(Lightning ×magic)
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
        // Equilibrium：GainBlock + EquilibriumPower(magic=1)（整手保留钩子 TODO 3.5）
        "Equilibrium" => {
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
        // FTL：FTLAction——Damage + 每点本回合已耗能抽 1（energy counter TODO 3.5：先抽 magic）
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
        // Fission：FissionAction——引动全部球（升级=不移除）
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
        // Force Field：GainBlock（force field 递增钩子 TODO 3.5：每打能力 +4）
        "Force Field" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        // Fusion：Channel(Plasma ×magic)
        "Fusion" => {
            for _ in 0..card.eff_magic() {
                eb.bot(GameAction::ChannelOrb {
                    kind: crate::orb::OrbKind::Plasma,
                });
            }
        }
        // Genetic Algorithm：IncreaseMiscAction(uuid, misc, magic) + GainBlock（misc 成长卡）
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
        // Go for the Eyes：Damage + ForTheEyesAction（目标意图攻击 → 自身 Vulnerable？原版：施加目标 Vulnerable）
        "Go for the Eyes" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                // ForTheEyesAction：意图攻击 → 目标 Vulnerable magic
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
        // Heatsinks：ApplyPower(Heatsinks, magic)（抽到能力多抽：钩子 TODO 3.5）
        "Heatsinks" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Heatsinks, card.eff_magic()),
                from_monster: false,
            });
        }
        // Hello World：ApplyPower(HelloWorld, 1)（每回合随机牌：TODO 4.1）
        "Hello World" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::HelloWorld, 1),
                from_monster: false,
            });
        }
        // Hologram：GainBlock + BetterDiscardPileToHandAction(1)（弃牌堆取 1：选择屏 TODO 6）
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
        // Hyperbeam：DamageAll + Focus -3
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
        // Impulse：ImpulseAction——抽 2、下回合少 1 能量（ImpulsePower TODO 3.5）
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
        // Leap：GainBlock
        "Leap" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        // Lock-On：Damage + LockOnPower（球伤 1.5×：钩子 TODO 3.8）
        "Lock-On" => {
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
        // Loop：ApplyPower(Loop, magic)（每回合引动首球：钩子 TODO 3.5）
        "Loop" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Future, 0),
                from_monster: false,
            });
        }
        // Machine Learning：ApplyPower(DrawPower, magic)（手牌上限 +magic：ApplyPower/Remove 已接）
        "Machine Learning" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::MachineLearning, card.eff_magic()),
                from_monster: false,
            });
        }
        // Melter：RemoveAllBlock + Damage（无视并清除格挡后打）
        "Melter" => {
            if let Some(mi) = target {
                b.remove_monster_block(mi);
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Meteor Strike：Damage（3 费；能量耦合 TODO 3.5）
        "Meteor Strike" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Multi-Cast：MulticastAction 例程（X 费：引动 X 次，首球不移除）
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
        // Overclock：DrawCard(2) + Burn 进弃牌堆
        "Overclock" => {
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
        // Rainbow：Channel 闪电+冰霜+暗
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
        // Reboot：ShuffleAll（洗回全部牌堆）+ DrawCard(magic 2/3)
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
        // Rebound：Damage + ReboundPower（下张卡回手：钩子 TODO 3.5）
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
        // Recursion：RedoAction——引动最后（最右）球 + 通道新球
        "Recursion" => {
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
        // Recycle：RecycleAction——消耗手牌一张回等价能量（选择屏 TODO 6）
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
        // Reinforced Body：ReinforcedBodyAction（X 费格挡，保留语义 TODO 3.5）
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
        // Reprogram：Focus -magic + 力量/敏捷 +2 + 球槽清空（原版 lose all orb slots）
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
        // Rip and Tear：2 段 Damage 7
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
        // Scrape：Damage 4 + DrawCard(5) + ScrapeAction（弃非攻击 TODO 6）
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
        // Seek：BetterDrawPileToHandAction(magic 1-3)——抽牌堆取 N 张（选择屏 TODO 6：占位抽顶 N）
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
        // Self Repair：ApplyPower(Repair, magic)（战斗结束回血：钩子 TODO 3.5）
        "Self Repair" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Repair, card.eff_magic()),
                from_monster: false,
            });
        }
        // Skim：DrawCard(magic 3/4)
        "Skim" => {
            eb.bot(GameAction::DrawCard {
                amount: card.eff_magic(),
                end_turn_draw: false,
            });
        }
        // Stack：GainBlock(block) + StackPower（按抽牌堆数成长——calculateCardDamage 动态 TODO 3.5）
        "Stack" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        // Static Discharge：ApplyPower(StaticDischarge, magic)（受击引雷：钩子 TODO 3.5）
        "Static Discharge" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::StaticDischarge, card.eff_magic()),
                from_monster: false,
            });
        }
        // Steam Barrier：GainBlock + SteamBarrierPower（每回合 -1：钩子 TODO 3.5）
        "Steam Barrier" => {
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
        // Storm：ApplyPower(Storm, magic)（出牌引雷：consume 钩子 TODO 3.5——同 ThousandCuts 模式）
        "Storm" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Future, 0),
                from_monster: false,
            });
        }
        // Streamline：Damage + ReduceCostAction(uuid, -1)（misc -1 每次打出）
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
        // Strike_Blue：Damage（基础卡显式化）
        "Strike_Blue" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Sunder：SunderAction——Damage + 命中则 GainEnergy(3)
        "Sunder" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::GainEnergy { amount: 3 });
            }
        }
        // Sweeping Beam：DamageAll + DrawCard 1
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
        // Tempest：TempestAction 例程（X 费：引动 X 道闪电不耗能量段——原版 channel X）
        "Tempest" => {
            for _ in 0..b.energy {
                eb.bot(GameAction::ChannelOrb {
                    kind: crate::orb::OrbKind::Lightning,
                });
            }
            eb.energy = 0;
        }
        // Thunder Strike：Damage = base × 本战斗闪电引动数（计数器已接）
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
        // Turbo：GainEnergy(magic 2) + Void 进弃牌堆
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
        // White Noise：随机能力卡进手 0 费——TODO 4.1（例程占位）
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
        // Zap：Channel(Lightning ×magic 1)（附闪电牌机制 TODO 3.8：产出状态牌进手）
        "Zap" => {
            for _ in 0..card.eff_magic() {
                eb.bot(GameAction::ChannelOrb {
                    kind: crate::orb::OrbKind::Lightning,
                });
            }
        }

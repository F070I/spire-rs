        // A Thousand Cuts：ApplyPower(ThousandCuts, magic)（出牌全体伤：consume 钩子已接）
        "A Thousand Cuts" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::ThousandCuts, card.eff_magic()),
                from_monster: false,
            });
        }
        // After Image：ApplyPower(AfterImage, 1)（出牌 +无实体：onUseCard 钩子 TODO 3.5）
        "After Image" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::AfterImage, 1),
                from_monster: false,
            });
        }
        // Bouncing Flask：随机怪 3 次 ×magic 毒（TODO 4.1 随机目标：首活怪占位）
        "Bouncing Flask" => {
            if let Some(mi) = b.first_alive_monster() {
                for _ in 0..3 {
                    eb.bot(GameAction::ApplyPower {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Player),
                        power: Power::new(PowerKind::Poison, card.eff_magic()),
                        from_monster: false,
                    });
                }
            }
        }
        // Bullet Time：NoDrawPower + ApplyBulletTimeAction（手牌 0 费例程）
        "Bullet Time" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::NoDraw, -1),
                from_monster: false,
            });
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "ApplyBulletTimeAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Burst：ApplyPower(Burst, magic)（下张技能翻倍——onUseCard 钩子 TODO 3.5）
        "Burst" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Burst, card.eff_magic()),
                from_monster: false,
            });
        }
        // Calculated Gamble：CalculatedGambleAction 例程（弃整手抽同数）
        "Calculated Gamble" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "CalculatedGambleAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Catalyst：未升级 ×2 / 升级 ×3（CatalystAction 例程）
        "Catalyst" => {
            if let Some(mi) = target {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "CatalystAction",
                        target: Some(mi),
                        amount: if card.upgraded { 3 } else { 2 },
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Corpse Explosion：Poison(magic) + CorpseExplosionPower（死亡传播钩子 TODO 3.5）
        "Corpse Explosion" => {
            if let Some(mi) = target {
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Poison, card.eff_magic()),
                    from_monster: false,
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::future("Corpse Explosion", 1),
                    from_monster: false,
                });
            }
        }
        // Crippling Poison：全体 Poison(magic) + Weak 2
        "Crippling Poison" => {
            for mi in 0..b.monsters.len() {
                if !b.monsters[mi].creature.is_dying && !b.monsters[mi].creature.is_dead {
                    eb.bot(GameAction::ApplyPower {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Player),
                        power: Power::new(PowerKind::Poison, card.eff_magic()),
                        from_monster: false,
                    });
                    eb.bot(GameAction::ApplyPower {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Player),
                        power: Power::new(PowerKind::Weak, 2),
                        from_monster: false,
                    });
                }
            }
        }
        // Distraction：随机技能进手 0 费——TODO 4.1 池随机（例程占位）
        "Distraction" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "DistractionAction",
                    target: None,
                    amount: 1,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Doppelganger：X 费——下回合抽 X + 能量 X（DoppelgangerAction 例程占位 + 能量语义）
        "Doppelganger" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "DoppelgangerAction",
                    target: None,
                    amount: b.energy,
                    amount2: b.energy,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Eviscerate：3 段伤害（本回合弃牌减费钩子 TODO 3.5）
        "Eviscerate" => {
            if let Some(mi) = target {
                for _ in 0..3 {
                    eb.bot(GameAction::Damage {
                        target: TargetRef::Monster(mi),
                        info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                    });
                }
            }
        }
        // Envenom：ApplyPower(Envenom, 1)（未格挡攻击伤上毒：钩子 TODO 3.5）
        "Envenom" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Envenom, 1),
                from_monster: false,
            });
        }
        // Finisher：DamagePerAttackPlayedAction 例程（每张本回合攻击一段）
        "Finisher" => {
            if let Some(mi) = target {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "FinisherAction",
                        target: Some(mi),
                        amount: card.eff_damage(),
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Flechettes：FlechetteAction 例程（手牌技能数 × amount 伤）
        "Flechettes" => {
            if let Some(mi) = target {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "FlechetteAction",
                        target: Some(mi),
                        amount: 4,
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Glass Knife：2 段伤害 + ModifyDamageAction(uuid, -2)（misc -2 本战斗）
        "Glass Knife" => {
            if let Some(mi) = target {
                let base = card.st().base_damage + card.misc;
                for _ in 0..2 {
                    eb.bot(GameAction::Damage {
                        target: TargetRef::Monster(mi),
                        info: attack_info(b, mi, base, DamageType::Normal),
                    });
                }
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "ModifyDamageAction",
                        target: None,
                        amount: -2,
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Grand Finale：DamageAll（canUse：抽牌堆必须为空——can_use 特例已接）
        "Grand Finale" => {
            let matrix = damage_matrix(b, card.eff_damage(), DamageType::Normal);
            eb.bot(GameAction::DamageAll {
                matrix,
                ty: DamageType::Normal,
            });
        }
        // Heel Hook：HeelHookAction 例程（本回合有弃牌 → 抽 1 回 1 能）
        "Heel Hook" => {
            if let Some(mi) = target {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "HeelHookAction",
                        target: Some(mi),
                        amount: 0,
                        amount2: 0,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Infinite Blades：ApplyPower(InfiniteBlades, 1)（抽牌后 +Shiv：钩子已接）
        "Infinite Blades" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::InfiniteBlades, 1),
                from_monster: false,
            });
        }
        // Leg Sweep：ApplyPower(Weak, magic) + GainBlock
        "Leg Sweep" => {
            if let Some(mi) = target {
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Weak, card.eff_magic()),
                    from_monster: false,
                });
            }
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
        }
        // Malaise：MalaiseAction 例程（X 费：Weak 与力量 -X）
        "Malaise" => {
            if let Some(mi) = target {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "MalaiseAction",
                        target: Some(mi),
                        amount: 0,
                        amount2: b.energy,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Masterful Stab：Damage 12（对中毒目标 1 费：modifyCost 钩子 TODO 3.5）
        "Masterful Stab" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Neutralize：Damage 3 + Weak(magic 1/2)
        "Neutralize" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Weak, card.eff_magic()),
                    from_monster: false,
                });
            }
        }
        // Nightmare（原版 ID "Night Terror"）：NightmareAction——复制手牌（选择屏 TODO 批次 6）
        "Night Terror" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "NightmareAction",
                    target: None,
                    amount: card.eff_magic(),
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Noxious Fumes：ApplyPower(NoxiousFumes, magic)（回合末全体加毒：钩子已接）
        "Noxious Fumes" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::NoxiousFumes, card.eff_magic()),
                from_monster: false,
            });
        }
        // Outmaneuver：Energized(magic 2/3)
        "Outmaneuver" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Energized, card.eff_magic()),
                from_monster: false,
            });
        }
        // Phantasmal Killer：ApplyPower(Phantasmal, 1)（下回合弃整手按技能伤：钩子 TODO 3.5）
        "Phantasmal Killer" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::Phantasmal, 1),
                from_monster: false,
            });
        }
        // PiercingWail：全体敌人 Strength -magic（本回合收回：LoseStrength 配对）
        "PiercingWail" => {
            for mi in 0..b.monsters.len() {
                if !b.monsters[mi].creature.is_dying && !b.monsters[mi].creature.is_dead {
                    eb.bot(GameAction::ApplyPower {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Player),
                        power: Power::new(PowerKind::Strength, -card.eff_magic()),
                        from_monster: false,
                    });
                    eb.bot(GameAction::ApplyPower {
                        target: TargetRef::Monster(mi),
                        source: Some(TargetRef::Player),
                        power: Power::new(PowerKind::LoseStrength, card.eff_magic()),
                        from_monster: false,
                    });
                }
            }
        }
        // Poisoned Stab：Damage 6 + Poison(magic 5/7)
        "Poisoned Stab" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Poison, card.eff_magic()),
                    from_monster: false,
                });
            }
        }
        // Predator：Damage 15 + DrawCardNextTurnPower(magic 2/3)（钩子 TODO 3.5）
        "Predator" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Player,
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::DrawCardNextTurn, card.eff_magic()),
                    from_monster: false,
                });
            }
        }
        // Prepared：DrawCard(magic 1/2) + Discard(magic, 任意张)
        "Prepared" => {
            eb.bot(GameAction::DrawCard {
                amount: card.eff_magic(),
                end_turn_draw: false,
            });
            eb.bot(GameAction::Discard {
                amount: card.eff_magic(),
                is_random: false,
                any_number: true,
            });
        }
        // Quick Slash：Damage 8 + DrawCard 1
        "Quick Slash" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::DrawCard {
                    amount: 1,
                    end_turn_draw: false,
                });
            }
        }
        // Reflex：use() 仅 DrawCard(magic 2)（弃牌触发钩子 TODO 3.5）
        "Reflex" => {
            eb.bot(GameAction::DrawCard {
                amount: card.eff_magic(),
                end_turn_draw: false,
            });
        }
        // Riddle With Holes：5 段伤害
        "Riddle With Holes" => {
            if let Some(mi) = target {
                for _ in 0..5 {
                    eb.bot(GameAction::Damage {
                        target: TargetRef::Monster(mi),
                        info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                    });
                }
            }
        }
        // Setup：SetupAction 例程（手牌置抽牌堆顶；选择屏 TODO 批次 6）
        "Setup" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "SetupAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Skewer：SkewerAction 例程（X 费：每点 4 伤一段）
        "Skewer" => {
            if let Some(mi) = target {
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "SkewerAction",
                        target: Some(mi),
                        amount: card.eff_damage(),
                        amount2: b.energy,
                        card_uuid: card.uuid,
                        sid: card.sid,
                        upgraded: card.upgraded,
                    },
                });
            }
        }
        // Slice：Damage 6
        "Slice" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Storm of Steel：BladeFuryAction 例程（弃整手 → 每张 +1 飞刀）
        "Storm of Steel" => {
            eb.bot(GameAction::CardRoutine {
                call: RoutineCall {
                    id: "BladeFuryAction",
                    target: None,
                    amount: 0,
                    amount2: 0,
                    card_uuid: card.uuid,
                    sid: card.sid,
                    upgraded: card.upgraded,
                },
            });
        }
        // Strike_G：Damage（基础卡显式化）
        "Strike_G" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
            }
        }
        // Sucker Punch：Damage 3 + Weak 1
        "Sucker Punch" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::ApplyPower {
                    target: TargetRef::Monster(mi),
                    source: Some(TargetRef::Player),
                    power: Power::new(PowerKind::Weak, 1),
                    from_monster: false,
                });
            }
        }
        // Survivor：GainBlock + Discard(1, 非随机)（弃牌触发能量 TODO 3.5）
        "Survivor" => {
            eb.bot(GameAction::GainBlock {
                target: TargetRef::Player,
                amount: card.eff_block(),
            });
            eb.bot(GameAction::Discard {
                amount: 1,
                is_random: false,
                any_number: false,
            });
        }
        // Tools of the Trade：ApplyPower(ToolsOfTheTrade, 1)（抽牌后抽 1：钩子已接；弃 1 TODO）
        "Tools of the Trade" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::ToolsOfTheTrade, 1),
                from_monster: false,
            });
        }
        // Tactician：use() 为空——弃牌时 +1 能量（didDiscard 钩子 TODO 3.5）
        "Tactician" => {}
        // Unload：Damage 14 + UnloadAction（本回合不再打攻击——钩子 TODO 3.5）
        "Unload" => {
            if let Some(mi) = target {
                eb.bot(GameAction::Damage {
                    target: TargetRef::Monster(mi),
                    info: attack_info(b, mi, card.eff_damage(), DamageType::Normal),
                });
                eb.bot(GameAction::CardRoutine {
                    call: RoutineCall {
                        id: "UnloadAction",
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
        // Well Laid Plans：ApplyPower(RetainCard, magic)（回合末保留 N 张：钩子 TODO 3.5）
        "Well Laid Plans" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::RetainCard, card.eff_magic()),
                from_monster: false,
            });
        }
        // Wraith Form：ApplyPower(WraithForm, magic)（每回合 +无实体递减：钩子已接；虚无：静态标志）
        "Wraith Form" => {
            eb.bot(GameAction::ApplyPower {
                target: TargetRef::Player,
                source: Some(TargetRef::Player),
                power: Power::new(PowerKind::WraithForm, card.eff_magic()),
                from_monster: false,
            });
        }

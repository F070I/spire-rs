    /// 引动球（channelOrb：找第一个空槽；槽满 → 引动最老再通道，记录 03 §8）。
    pub fn channel_orb(&mut self, kind: crate::orb::OrbKind) {
        let orb = match kind {
            crate::orb::OrbKind::Lightning => crate::orb::Orb { kind, passive_amount: 3, evoke_amount: 8 },
            crate::orb::OrbKind::Frost => crate::orb::Orb { kind, passive_amount: 2, evoke_amount: 5 },
            crate::orb::OrbKind::Dark => crate::orb::Orb { kind, passive_amount: 6, evoke_amount: 6 },
            crate::orb::OrbKind::Plasma => crate::orb::Orb { kind, passive_amount: 1, evoke_amount: 2 },
            crate::orb::OrbKind::EmptySlot => return,
        };
        if self.orbs.len() < self.max_orbs {
            self.orbs.push(orb);
        } else if !self.orbs.is_empty() {
            self.evoke_orb();
            self.orbs.push(orb);
        }
    }

    /// 引动最老的球（evokeOrb：orbs[0] onEvoke → 左移补空槽，记录 03 §8）。
    pub fn evoke_orb(&mut self) {
        if self.orbs.is_empty() {
            return;
        }
        let orb = self.orbs.remove(0);
        let out = crate::orb::evoke_effect(&orb);
        if out.thorns_damage > 0 {
            // 闪电球：THORNS 打随机敌人（Electro 全体分支 TODO 3.8）
            if let Some(mi) = self.first_alive_monster() {
                self.exec_damage(
                    TargetRef::Monster(mi),
                    crate::damage::DamageInfo::pure(false, out.thorns_damage, crate::orb::ORB_DAMAGE_TYPE),
                );
            }
        }
        if out.block > 0 {
            self.player.add_block(out.block);
        }
        if out.dark_damage > 0 {
            // 暗球：打血量最高的敌人
            if let Some(mi) = self.highest_hp_monster() {
                self.exec_damage(
                    TargetRef::Monster(mi),
                    crate::damage::DamageInfo::pure(false, out.dark_damage, crate::orb::ORB_DAMAGE_TYPE),
                );
            }
        }
        if out.energy > 0 {
            self.energy = (self.energy + out.energy).min(999);
        }
        self.orbs.push(crate::orb::Orb {
            kind: crate::orb::OrbKind::EmptySlot,
            passive_amount: 0,
            evoke_amount: 0,
        });
        self.check_battle_over();
    }

    fn first_alive_monster(&self) -> Option<usize> {
        self.monsters
            .iter()
            .position(|m| !m.creature.is_dying && !m.creature.is_dead)
    }

    fn highest_hp_monster(&self) -> Option<usize> {
        self.monsters
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.creature.is_dying && !m.creature.is_dead)
            .max_by_key(|(_, m)| m.creature.current_health)
            .map(|(i, _)| i)
    }

    /// 切换架势（ChangeStanceAction，记录 03 §9：powers→relics→exit→enter；钩子 TODO 3.5/3.6）。
    pub fn change_stance(&mut self, new: crate::stance::StanceKind) {
        if self.player_stance == new {
            return;
        }
        if self.player_stance == crate::stance::StanceKind::Calm {
            // CalmStance.onExitStance：+2 能量
            self.energy = (self.energy + 2).min(999);
        }
        self.player_stance = new;
        if new == crate::stance::StanceKind::Divinity {
            // DivinityStance.onEnterStance：+3 能量
            self.energy = (self.energy + 3).min(999);
        }
    }

    /// 弃牌动作（DiscardAction，记录 09 §5）：随机/全弃；选择屏在批次 6。
    fn exec_discard(&mut self, amount: i32, is_random: bool, _any_number: bool) {
        let n = self.hand.group.len() as i32;
        if is_random {
            for _ in 0..amount.min(n) {
                // 随机弃：cardRandomRng（TODO 4.1 接线；取顶占位）
                if let Some(c) = self.hand.group.pop() {
                    self.discard_pile.add_top(c);
                }
            }
        } else if n <= amount {
            while let Some(c) = self.hand.group.pop() {
                self.discard_pile.add_top(c);
            }
        }
    }


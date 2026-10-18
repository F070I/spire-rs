// 自动生成 —— 勿手改（tools/extract.py）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardType {
    Attack,
    Skill,
    Power,
    Status,
    Curse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardColor {
    Red,
    Green,
    Blue,
    Purple,
    Colorless,
    Curse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardRarity {
    Basic,
    Special,
    Common,
    Uncommon,
    Rare,
    Curse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardTarget {
    Enemy,
    AllEnemy,
    Self_,
    None_,
    SelfAndEnemy,
    All,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelicTier {
    Deprecated,
    Starter,
    Common,
    Uncommon,
    Rare,
    Special,
    Boss,
    Shop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PotionRarity {
    Placeholder,
    Common,
    Uncommon,
    Rare,
}

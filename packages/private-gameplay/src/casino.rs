//! The casino's settings and the maths behind every game. Nothing in here touches the database, so the odds can be unit tested
//! and previewed live in the admin page (the same functions produce the player's tables and the admin's return-to-player figures).

use rand::Rng;
use serde::{Deserialize, Serialize};

pub fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Roulette { pub enabled: bool, pub min_bet: f64, pub max_bet: f64 }
impl Default for Roulette { fn default() -> Self { Self { enabled: true, min_bet: 10.0, max_bet: 5000.0 } } }
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Burst { pub enabled: bool, pub min_bet: f64, pub max_bet: f64, pub survival: f64, pub house_edge: f64, pub max_steps: u32 }
impl Default for Burst { fn default() -> Self { Self { enabled: true, min_bet: 10.0, max_bet: 5000.0, survival: 0.72, house_edge: 0.04, max_steps: 12 } } }
pub fn roulette_red(number: u32) -> bool { matches!(number, 1|3|5|7|9|12|14|16|18|19|21|23|25|27|30|32|34|36) }
pub fn roulette_payout(number: u32, selection: &str, straight: Option<u32>) -> Option<f64> {
    let (won, multiplier) = match selection {
        "straight" => (number == straight.filter(|n| *n <= 36)?, 36.0),
        "red" => (roulette_red(number), 2.0), "black" => (number > 0 && !roulette_red(number), 2.0),
        "odd" => (number > 0 && number % 2 == 1, 2.0), "even" => (number > 0 && number % 2 == 0, 2.0),
        "low" => ((1..=18).contains(&number), 2.0), "high" => ((19..=36).contains(&number), 2.0),
        "first" => ((1..=12).contains(&number), 3.0), "second" => ((13..=24).contains(&number), 3.0), "third" => ((25..=36).contains(&number), 3.0),
        _ => return None,
    }; Some(if won { multiplier } else { 0.0 })
}

// ---- settings ----------------------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Symbol {
    pub id: String,
    pub label: String,
    /// Relative chance of landing on a reel.
    pub weight: f64,
    /// Multiplier of the bet for three in a row.
    pub pay: f64,
    pub color: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Slots {
    pub enabled: bool,
    pub min_bet: f64,
    pub max_bet: f64,
    pub symbols: Vec<Symbol>,
    /// Multiplier for exactly two matching symbols.
    pub pair_pay: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Segment {
    pub label: String,
    /// For the wheel: a multiplier of the bet. For the daily spin: an amount of money.
    pub value: f64,
    pub weight: f64,
    pub color: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Wheel {
    pub enabled: bool,
    pub min_bet: f64,
    pub max_bet: f64,
    pub segments: Vec<Segment>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Daily {
    pub enabled: bool,
    /// Free spins each player gets per UTC day.
    pub spins_per_day: u32,
    pub segments: Vec<Segment>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Plinko {
    pub enabled: bool,
    pub min_bet: f64,
    pub max_bet: f64,
    pub min_rows: u32,
    pub max_rows: u32,
    /// Return to player, 0..1 (so 0.96 keeps a 4% house edge).
    pub rtp: f64,
    pub risks: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Mines {
    pub enabled: bool,
    pub min_bet: f64,
    pub max_bet: f64,
    /// The board is `size` x `size`.
    pub size: u32,
    pub min_mines: u32,
    pub max_mines: u32,
    pub house_edge: f64,
    pub max_multiplier: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Blackjack {
    pub enabled: bool,
    pub min_bet: f64,
    pub max_bet: f64,
    /// What a natural blackjack pays on top of the stake (1.5 is the classic 3 to 2).
    pub blackjack_pay: f64,
    /// Whether the dealer draws on a soft 17 (a 17 that counts an ace as 11).
    pub dealer_hits_soft_17: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Crash {
    pub enabled: bool,
    pub min_bet: f64,
    pub max_bet: f64,
    /// Share of every bet the house keeps over time, whatever multiplier players cash out at.
    pub house_edge: f64,
    pub max_multiplier: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Dice {
    pub enabled: bool,
    pub min_bet: f64,
    pub max_bet: f64,
    pub house_edge: f64,
    /// The player picks their own win chance between these two percentages; a smaller chance pays more.
    pub min_chance: f64,
    pub max_chance: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Coinflip {
    pub enabled: bool,
    pub min_bet: f64,
    pub max_bet: f64,
    /// Multiplier of the bet for calling the coin right. A fair coin would pay 2.
    pub payout: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Double {
    pub enabled: bool,
    /// Chance (0 to 1) that the double comes off. Under 0.5 keeps an edge for the house.
    pub win_chance: f64,
    /// How many doubles in a row a player may take.
    pub max_streak: u32,
    /// How long the offer stays open after the win.
    pub offer_minutes: u32,
}

/// Surprises layered on top of any winning instant round, so results are not just a fixed table: sometimes a win is boosted
/// by a lucky surge, sometimes it is cursed and halved. With the defaults the two cancel out, so the return to players barely moves.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Chaos {
    pub enabled: bool,
    pub surge_chance: f64,
    pub curse_chance: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Bounties {
    pub enabled: bool,
    pub min_amount: f64,
    pub max_amount: f64,
    /// Share of a bounty the house keeps when it is placed.
    pub tax_percent: f64,
    pub expire_days: u32,
    pub max_active_per_player: u32,
    /// The same killer cannot collect on the same victim again within this many minutes.
    pub claim_cooldown_minutes: u32,
    pub allow_anonymous: bool,
    pub allow_cancel: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Betting {
    pub enabled: bool,
    /// `players` lets anyone open a bet; `admins` keeps that to the panel.
    pub creators: String,
    pub metrics: Vec<String>,
    pub windows_minutes: Vec<u32>,
    /// Betting stops this long before the end, so nobody bets on something that has already happened.
    pub lock_minutes: u32,
    pub min_stake: f64,
    pub max_stake: f64,
    pub rake_percent: f64,
    pub max_open_per_player: u32,
    pub max_threshold: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub enabled: bool,
    /// The most a single round can pay out.
    pub max_payout: f64,
    /// A player who has lost this much today is stopped until tomorrow. 0 turns the limit off.
    pub daily_loss_limit: f64,
    /// Wins at least this big are announced in the big-wins ticker.
    pub feed_min_win: f64,
    pub slots: Slots,
    pub wheel: Wheel,
    pub daily: Daily,
    pub plinko: Plinko,
    pub mines: Mines,
    pub blackjack: Blackjack,
    pub crash: Crash,
    pub dice: Dice,
    pub coinflip: Coinflip,
    pub roulette: Roulette,
    pub burst: Burst,
    pub double: Double,
    pub chaos: Chaos,
    pub bounties: Bounties,
    pub betting: Betting,
}

fn sym(id: &str, label: &str, weight: f64, pay: f64, color: &str) -> Symbol {
    Symbol { id: id.into(), label: label.into(), weight, pay, color: color.into() }
}
fn seg(label: &str, value: f64, weight: f64, color: &str) -> Segment {
    Segment { label: label.into(), value, weight, color: color.into() }
}

impl Default for Symbol {
    fn default() -> Self {
        sym("", "", 1.0, 0.0, "#888888")
    }
}
impl Default for Segment {
    fn default() -> Self {
        seg("", 0.0, 1.0, "#888888")
    }
}
impl Default for Slots {
    fn default() -> Self {
        Slots {
            enabled: true,
            min_bet: 10.0,
            max_bet: 5000.0,
            pair_pay: 0.6,
            symbols: vec![
                sym("cherry", "Cherry", 30.0, 7.0, "#ef4444"),
                sym("lemon", "Lemon", 26.0, 10.0, "#facc15"),
                sym("bell", "Bell", 20.0, 17.0, "#f59e0b"),
                sym("clover", "Clover", 12.0, 44.0, "#22c55e"),
                sym("gem", "Gem", 8.0, 87.0, "#38bdf8"),
                sym("seven", "Lucky 7", 4.0, 218.0, "#a855f7"),
            ],
        }
    }
}
impl Default for Wheel {
    fn default() -> Self {
        Wheel {
            enabled: true,
            min_bet: 10.0,
            max_bet: 5000.0,
            segments: vec![
                seg("0x", 0.0, 44.0, "#475569"),
                seg("0.5x", 0.5, 22.0, "#64748b"),
                seg("1x", 1.0, 14.0, "#0ea5e9"),
                seg("1.5x", 1.5, 12.0, "#22c55e"),
                seg("2x", 2.0, 9.0, "#eab308"),
                seg("3x", 3.0, 5.0, "#f97316"),
                seg("5x", 5.0, 2.6, "#ef4444"),
                seg("10x", 10.0, 1.0, "#a855f7"),
                seg("50x", 50.0, 0.1, "#ec4899"),
            ],
        }
    }
}
impl Default for Daily {
    fn default() -> Self {
        Daily {
            enabled: true,
            spins_per_day: 1,
            segments: vec![
                seg("$25", 25.0, 40.0, "#64748b"),
                seg("$50", 50.0, 30.0, "#0ea5e9"),
                seg("$100", 100.0, 18.0, "#22c55e"),
                seg("$250", 250.0, 8.0, "#eab308"),
                seg("$500", 500.0, 3.0, "#f97316"),
                seg("$2,500", 2500.0, 1.0, "#ec4899"),
            ],
        }
    }
}
impl Default for Plinko {
    fn default() -> Self {
        Plinko {
            enabled: true,
            min_bet: 10.0,
            max_bet: 5000.0,
            min_rows: 8,
            max_rows: 16,
            rtp: 0.96,
            risks: vec!["low".into(), "medium".into(), "high".into()],
        }
    }
}
impl Default for Mines {
    fn default() -> Self {
        Mines {
            enabled: true,
            min_bet: 10.0,
            max_bet: 5000.0,
            size: 5,
            min_mines: 1,
            max_mines: 24,
            house_edge: 0.04,
            max_multiplier: 5000.0,
        }
    }
}
impl Default for Blackjack {
    fn default() -> Self {
        Blackjack { enabled: true, min_bet: 10.0, max_bet: 5000.0, blackjack_pay: 1.5, dealer_hits_soft_17: false }
    }
}
impl Default for Crash {
    fn default() -> Self {
        Crash { enabled: true, min_bet: 10.0, max_bet: 5000.0, house_edge: 0.04, max_multiplier: 1000.0 }
    }
}
impl Default for Dice {
    fn default() -> Self {
        Dice { enabled: true, min_bet: 10.0, max_bet: 5000.0, house_edge: 0.03, min_chance: 1.0, max_chance: 95.0 }
    }
}
impl Default for Coinflip {
    fn default() -> Self {
        Coinflip { enabled: true, min_bet: 10.0, max_bet: 5000.0, payout: 1.96 }
    }
}
impl Default for Double {
    fn default() -> Self {
        Double { enabled: true, win_chance: 0.49, max_streak: 5, offer_minutes: 5 }
    }
}
impl Default for Chaos {
    fn default() -> Self {
        Chaos { enabled: true, surge_chance: 0.04, curse_chance: 0.06 }
    }
}
impl Default for Bounties {
    fn default() -> Self {
        Bounties {
            enabled: true,
            min_amount: 100.0,
            max_amount: 1_000_000.0,
            tax_percent: 10.0,
            expire_days: 14,
            max_active_per_player: 5,
            claim_cooldown_minutes: 30,
            allow_anonymous: true,
            allow_cancel: true,
        }
    }
}
impl Default for Betting {
    fn default() -> Self {
        Betting {
            enabled: true,
            creators: "players".into(),
            metrics: vec!["player_kills".into(), "deaths".into(), "mob_kills".into(), "blocks_broken".into()],
            windows_minutes: vec![30, 60, 360, 1440],
            lock_minutes: 5,
            min_stake: 10.0,
            max_stake: 100_000.0,
            rake_percent: 5.0,
            max_open_per_player: 3,
            max_threshold: 10_000,
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Config {
            enabled: true,
            max_payout: 1_000_000.0,
            daily_loss_limit: 0.0,
            feed_min_win: 500.0,
            slots: Slots::default(),
            wheel: Wheel::default(),
            daily: Daily::default(),
            plinko: Plinko::default(),
            mines: Mines::default(),
            blackjack: Blackjack::default(),
            crash: Crash::default(),
            dice: Dice::default(),
            coinflip: Coinflip::default(),
            roulette: Roulette::default(),
            burst: Burst::default(),
            double: Double::default(),
            chaos: Chaos::default(),
            bounties: Bounties::default(),
            betting: Betting::default(),
        }
    }
}

pub const METRICS: &[(&str, &str)] = &[
    ("player_kills", "player kills"),
    ("deaths", "deaths"),
    ("mob_kills", "mob kills"),
    ("blocks_broken", "blocks broken"),
    ("blocks_placed", "blocks placed"),
];
pub const RISKS: &[&str] = &["low", "medium", "high"];

fn money(v: f64, lo: f64, hi: f64) -> f64 {
    if v.is_finite() {
        round2(v.clamp(lo, hi))
    } else {
        lo
    }
}
fn color(c: &str, fallback: &str) -> String {
    let ok = c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|h| h.is_ascii_hexdigit());
    if ok {
        c.to_ascii_lowercase()
    } else {
        fallback.to_string()
    }
}
fn text(s: &str, max: usize, fallback: &str) -> String {
    let t: String = s.trim().chars().filter(|c| !c.is_control()).take(max).collect();
    if t.is_empty() {
        fallback.to_string()
    } else {
        t
    }
}

fn bet_range(min: &mut f64, max: &mut f64) {
    *min = money(*min, 0.01, 1e9);
    *max = money(*max, 0.01, 1e9);
    if *max < *min {
        *max = *min;
    }
}

impl Config {
    /// Clamp everything into a range the games can run on, and repair lists that were emptied. Called on every save.
    pub fn sanitize(&mut self) {
        self.max_payout = money(self.max_payout, 1.0, 1e12);
        self.daily_loss_limit = money(self.daily_loss_limit, 0.0, 1e12);
        self.feed_min_win = money(self.feed_min_win, 0.0, 1e12);

        let s = &mut self.slots;
        bet_range(&mut s.min_bet, &mut s.max_bet);
        s.pair_pay = money(s.pair_pay, 0.0, 100.0);
        s.symbols.truncate(10);
        for (i, y) in s.symbols.iter_mut().enumerate() {
            y.id =
                text(&y.id.to_ascii_lowercase(), 16, &format!("s{i}")).chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
            if y.id.is_empty() {
                y.id = format!("s{i}");
            }
            y.label = text(&y.label, 16, &y.id.clone());
            y.weight = money(y.weight, 0.01, 10_000.0);
            y.pay = money(y.pay, 0.0, 100_000.0);
            y.color = color(&y.color, "#888888");
        }
        let mut seen = std::collections::HashSet::new();
        s.symbols.retain(|y| seen.insert(y.id.clone()));
        if s.symbols.len() < 2 {
            s.symbols = Slots::default().symbols;
        }

        let w = &mut self.wheel;
        bet_range(&mut w.min_bet, &mut w.max_bet);
        clean_segments(&mut w.segments, 1000.0, Wheel::default().segments);
        clean_segments(&mut self.daily.segments, 1e9, Daily::default().segments);
        self.daily.spins_per_day = self.daily.spins_per_day.min(50);

        let p = &mut self.plinko;
        bet_range(&mut p.min_bet, &mut p.max_bet);
        p.min_rows = p.min_rows.clamp(6, 16);
        p.max_rows = p.max_rows.clamp(p.min_rows, 16);
        p.rtp = if p.rtp.is_finite() { p.rtp.clamp(0.5, 0.995) } else { 0.96 };
        p.risks.retain(|r| RISKS.contains(&r.as_str()));
        p.risks.dedup();
        if p.risks.is_empty() {
            p.risks = Plinko::default().risks;
        }

        let m = &mut self.mines;
        bet_range(&mut m.min_bet, &mut m.max_bet);
        m.size = m.size.clamp(3, 7);
        let cells = m.size * m.size;
        m.min_mines = m.min_mines.clamp(1, cells - 1);
        m.max_mines = m.max_mines.clamp(m.min_mines, cells - 1);
        m.house_edge = if m.house_edge.is_finite() { m.house_edge.clamp(0.0, 0.5) } else { 0.04 };
        m.max_multiplier = money(m.max_multiplier, 1.0, 1e7);

        let j = &mut self.blackjack;
        bet_range(&mut j.min_bet, &mut j.max_bet);
        j.blackjack_pay = money(j.blackjack_pay, 1.0, 3.0);

        let c = &mut self.crash;
        bet_range(&mut c.min_bet, &mut c.max_bet);
        c.house_edge = if c.house_edge.is_finite() { c.house_edge.clamp(0.0, 0.5) } else { 0.04 };
        c.max_multiplier = money(c.max_multiplier, 2.0, 100_000.0);

        let d = &mut self.dice;
        bet_range(&mut d.min_bet, &mut d.max_bet);
        d.house_edge = if d.house_edge.is_finite() { d.house_edge.clamp(0.0, 0.5) } else { 0.03 };
        d.min_chance = money(d.min_chance, 0.01, 98.0);
        d.max_chance = money(d.max_chance, d.min_chance, 98.0);

        let f = &mut self.coinflip;
        bet_range(&mut f.min_bet, &mut f.max_bet);
        f.payout = money(f.payout, 1.0, 2.0);

        let x = &mut self.double;
        bet_range(&mut self.roulette.min_bet, &mut self.roulette.max_bet);
        bet_range(&mut self.burst.min_bet, &mut self.burst.max_bet);
        self.burst.survival = if self.burst.survival.is_finite() { self.burst.survival.clamp(0.1, 0.95) } else { 0.72 };
        self.burst.house_edge = if self.burst.house_edge.is_finite() { self.burst.house_edge.clamp(0.0, 0.25) } else { 0.04 };
        self.burst.max_steps = self.burst.max_steps.clamp(1, 20);
        x.win_chance = if x.win_chance.is_finite() { (x.win_chance * 1000.0).round() / 1000.0 } else { 0.49 }.clamp(0.05, 0.95);
        x.max_streak = x.max_streak.clamp(1, 20);
        x.offer_minutes = x.offer_minutes.clamp(1, 60);

        let h = &mut self.chaos;
        h.surge_chance = if h.surge_chance.is_finite() { h.surge_chance.clamp(0.0, 0.5) } else { 0.04 };
        h.curse_chance = if h.curse_chance.is_finite() { h.curse_chance.clamp(0.0, 0.5) } else { 0.06 };

        let b = &mut self.bounties;
        bet_range(&mut b.min_amount, &mut b.max_amount);
        b.tax_percent = money(b.tax_percent, 0.0, 90.0);
        b.expire_days = b.expire_days.min(365);
        b.max_active_per_player = b.max_active_per_player.clamp(1, 100);
        b.claim_cooldown_minutes = b.claim_cooldown_minutes.min(10_080);

        let t = &mut self.betting;
        if t.creators != "admins" {
            t.creators = "players".into();
        }
        t.metrics.retain(|m| METRICS.iter().any(|(k, _)| k == m));
        t.metrics.dedup();
        if t.metrics.is_empty() {
            t.metrics = Betting::default().metrics;
        }
        t.windows_minutes.retain(|w| (5..=20_160).contains(w));
        t.windows_minutes.sort_unstable();
        t.windows_minutes.dedup();
        t.windows_minutes.truncate(8);
        if t.windows_minutes.is_empty() {
            t.windows_minutes = Betting::default().windows_minutes;
        }
        let shortest = t.windows_minutes[0];
        t.lock_minutes = t.lock_minutes.min(shortest.saturating_sub(1));
        bet_range(&mut t.min_stake, &mut t.max_stake);
        t.rake_percent = money(t.rake_percent, 0.0, 50.0);
        t.max_open_per_player = t.max_open_per_player.clamp(1, 50);
        t.max_threshold = t.max_threshold.clamp(1, 1_000_000);
    }
}

fn clean_segments(list: &mut Vec<Segment>, max_value: f64, fallback: Vec<Segment>) {
    list.truncate(16);
    for (i, s) in list.iter_mut().enumerate() {
        s.label = text(&s.label, 12, &format!("#{}", i + 1));
        s.value = money(s.value, 0.0, max_value);
        s.weight = money(s.weight, 0.01, 10_000.0);
        s.color = color(&s.color, "#888888");
    }
    if list.len() < 2 {
        *list = fallback;
    }
}

// ---- weighted choice ---------------------------------------------------------------------------------------------------

pub fn pick_weighted<R: Rng>(weights: &[f64], rng: &mut R) -> usize {
    let total: f64 = weights.iter().sum();
    let mut roll = rng.gen::<f64>() * total;
    for (i, w) in weights.iter().enumerate() {
        if roll < *w {
            return i;
        }
        roll -= *w;
    }
    weights.len().saturating_sub(1)
}

// ---- slots -------------------------------------------------------------------------------------------------------------

pub struct SlotSpin {
    pub reels: [usize; 3],
    pub multiplier: f64,
}

pub fn slots_multiplier(cfg: &Slots, reels: [usize; 3]) -> f64 {
    let [a, b, c] = reels;
    if a == b && b == c {
        cfg.symbols[a].pay
    } else if a == b || b == c || a == c {
        cfg.pair_pay
    } else {
        0.0
    }
}

pub fn slots_spin<R: Rng>(cfg: &Slots, rng: &mut R) -> SlotSpin {
    let weights: Vec<f64> = cfg.symbols.iter().map(|s| s.weight).collect();
    let reels = [pick_weighted(&weights, rng), pick_weighted(&weights, rng), pick_weighted(&weights, rng)];
    SlotSpin { reels, multiplier: slots_multiplier(cfg, reels) }
}

/// (return to player, chance that a spin pays anything), worked out exactly over every combination.
pub fn slots_stats(cfg: &Slots) -> (f64, f64) {
    let total: f64 = cfg.symbols.iter().map(|s| s.weight).sum();
    let (mut rtp, mut hit) = (0.0, 0.0);
    for a in 0..cfg.symbols.len() {
        for b in 0..cfg.symbols.len() {
            for c in 0..cfg.symbols.len() {
                let p = cfg.symbols[a].weight * cfg.symbols[b].weight * cfg.symbols[c].weight / (total * total * total);
                let m = slots_multiplier(cfg, [a, b, c]);
                rtp += p * m;
                if m > 0.0 {
                    hit += p;
                }
            }
        }
    }
    (rtp, hit)
}

// ---- wheels ------------------------------------------------------------------------------------------------------------

/// Expected value of one pick, in the segment's own unit (multiplier of the bet, or money for the daily spin).
pub fn segments_expected(segments: &[Segment]) -> f64 {
    let total: f64 = segments.iter().map(|s| s.weight).sum();
    if total <= 0.0 {
        0.0
    } else {
        segments.iter().map(|s| s.value * s.weight).sum::<f64>() / total
    }
}

// ---- plinko ------------------------------------------------------------------------------------------------------------

/// How steeply the payouts climb toward the edges, and the floor for the middle slots, per risk level.
fn risk_shape(risk: &str) -> (f64, f64) {
    match risk {
        "low" => (2.0, 0.5),
        "high" => (5.0, 0.0),
        _ => (3.2, 0.25),
    }
}

/// Payout multipliers for the slots of a board with `rows` rows (`rows + 1` slots, left to right), scaled so the average
/// return over the binomial landing odds is exactly `rtp`.
pub fn plinko_table(rows: u32, risk: &str, rtp: f64) -> Vec<f64> {
    let n = rows as usize;
    let (power, floor) = risk_shape(risk);
    let odds: Vec<f64> = (0..=n).map(|k| binom(n, k) / 2f64.powi(n as i32)).collect();
    let shape: Vec<f64> = (0..=n).map(|k| ((2.0 * k as f64 - n as f64).abs() / n as f64).powf(power)).collect();
    let weighted: f64 = odds.iter().zip(&shape).map(|(o, s)| o * s).sum();
    let scale = if weighted > 0.0 { (rtp - floor) / weighted } else { 0.0 };
    shape.iter().map(|s| round2(floor + scale * s).max(0.0)).collect()
}

fn binom(n: usize, k: usize) -> f64 {
    let k = k.min(n - k);
    (0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64)
}

pub fn plinko_rtp(table: &[f64]) -> f64 {
    let n = table.len() - 1;
    table.iter().enumerate().map(|(k, m)| binom(n, k) / 2f64.powi(n as i32) * m).sum()
}

/// A ball's fall: at each peg it goes right (1) or left (0). The slot it lands in is the number of rights.
pub fn plinko_drop<R: Rng>(rows: u32, rng: &mut R) -> (Vec<u8>, usize) {
    let path: Vec<u8> = (0..rows).map(|_| rng.gen_range(0..2u8)).collect();
    let slot = path.iter().map(|b| *b as usize).sum();
    (path, slot)
}

// ---- mines -------------------------------------------------------------------------------------------------------------

/// The multiplier after `safe` safe tiles have been turned over on a board of `cells` tiles with `mines` mines: the fair odds
/// of getting that far, less the house edge.
pub fn mines_multiplier(cells: u32, mines: u32, safe: u32, edge: f64, cap: f64) -> f64 {
    if safe == 0 {
        return 1.0;
    }
    let mut m = 1.0 - edge;
    for i in 0..safe {
        let left = cells.saturating_sub(i) as f64;
        let good = cells.saturating_sub(mines).saturating_sub(i) as f64;
        if good <= 0.0 {
            break;
        }
        m *= left / good;
    }
    round2(m.min(cap).max(1.0))
}

pub fn mines_layout<R: Rng>(cells: u32, mines: u32, rng: &mut R) -> Vec<u32> {
    let mut all: Vec<u32> = (0..cells).collect();
    for i in 0..mines.min(cells) as usize {
        let j = rng.gen_range(i..all.len());
        all.swap(i, j);
    }
    let mut out = all[..mines.min(cells) as usize].to_vec();
    out.sort_unstable();
    out
}

// ---- dice ----------------------------------------------------------------------------------------------------------

/// What a roll that wins with probability `chance` percent pays: the fair price, less the house edge.
pub fn dice_multiplier(chance: f64, edge: f64) -> f64 {
    if chance <= 0.0 {
        0.0
    } else {
        ((100.0 * (1.0 - edge) / chance) * 10_000.0).floor() / 10_000.0
    }
}

/// A roll from 0.00 to 99.99. "under" wins below the target number, "over" wins above `100 - chance`.
pub fn dice_roll<R: Rng>(rng: &mut R) -> f64 {
    rng.gen_range(0..10_000u32) as f64 / 100.0
}

pub fn dice_wins(roll: f64, chance: f64, over: bool) -> bool {
    if over {
        roll >= 100.0 - chance
    } else {
        roll < chance
    }
}

// ---- crash ---------------------------------------------------------------------------------------------------------

/// How fast the multiplier climbs while a Crash round runs: it grows by this fraction (compounding) every second.
pub const CRASH_RATE: f64 = 0.12;

/// The multiplier `ms` milliseconds into a round.
pub fn crash_curve(ms: i64, cap: f64) -> f64 {
    let m = (CRASH_RATE * ms.max(0) as f64 / 1000.0).exp();
    m.min(cap)
}

/// Where this round will crash. `P(crash >= x) = (1 - edge) / x`, so every cash-out target has the same return to the player.
pub fn crash_point<R: Rng>(edge: f64, cap: f64, rng: &mut R) -> f64 {
    let u: f64 = 1.0 - rng.gen::<f64>(); // (0, 1]
    ((((1.0 - edge) / u).max(1.0).min(cap)) * 100.0).floor() / 100.0
}

// ---- blackjack -----------------------------------------------------------------------------------------------------

/// Cards are drawn from an endless shoe (each draw is independent), so there is nothing to count. 0..=51: rank = id % 13 + 1.
pub fn bj_draw<R: Rng>(rng: &mut R) -> u8 {
    rng.gen_range(0..52u8)
}

fn bj_value(card: u8) -> u32 {
    match card % 13 + 1 {
        1 => 11,
        r if r >= 10 => 10,
        r => r as u32,
    }
}

/// (best total, whether an ace is still counted as 11).
pub fn bj_total(hand: &[u8]) -> (u32, bool) {
    let mut total: u32 = hand.iter().map(|c| bj_value(*c)).sum();
    let mut soft = hand.iter().filter(|c| **c % 13 == 0).count();
    while total > 21 && soft > 0 {
        total -= 10;
        soft -= 1;
    }
    (total, soft > 0)
}

pub fn bj_natural(hand: &[u8]) -> bool {
    hand.len() == 2 && bj_total(hand).0 == 21
}

/// The dealer draws to 17 (and on a soft 17 when the casino says so).
pub fn bj_dealer_plays<R: Rng>(dealer: &mut Vec<u8>, hit_soft_17: bool, rng: &mut R) {
    loop {
        let (total, soft) = bj_total(dealer);
        if total > 17 || (total == 17 && !(soft && hit_soft_17)) {
            break;
        }
        dealer.push(bj_draw(rng));
    }
}

/// What a finished hand pays as a multiple of the stake: 0 loses, 1 pushes, 2 wins, more for a natural.
pub fn bj_payout(player: &[u8], dealer: &[u8], pay: f64, doubled: bool) -> f64 {
    let (p, d) = (bj_total(player).0, bj_total(dealer).0);
    if p > 21 {
        0.0
    } else if bj_natural(player) && !doubled {
        if bj_natural(dealer) {
            1.0
        } else {
            1.0 + pay
        }
    } else if bj_natural(dealer) || (d <= 21 && d > p) {
        0.0
    } else if d > 21 || p > d {
        2.0
    } else {
        1.0
    }
}

// ---- chaos and double or nothing -----------------------------------------------------------------------------------

/// Lucky-surge multipliers and how likely each is among surges.
const SURGES: [(f64, f64); 3] = [(1.5, 60.0), (2.0, 30.0), (3.0, 10.0)];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Twist {
    None,
    Surge(f64),
    Curse,
}

/// Roll the surprise for a winning round.
pub fn chaos_roll<R: Rng>(cfg: &Chaos, rng: &mut R) -> Twist {
    if !cfg.enabled {
        return Twist::None;
    }
    let r: f64 = rng.gen();
    if r < cfg.surge_chance {
        let weights: Vec<f64> = SURGES.iter().map(|s| s.1).collect();
        Twist::Surge(SURGES[pick_weighted(&weights, rng)].0)
    } else if r < cfg.surge_chance + cfg.curse_chance {
        Twist::Curse
    } else {
        Twist::None
    }
}

/// How much the chaos twists change the average winning payout (1.0 means they cancel out).
pub fn chaos_factor(cfg: &Chaos) -> f64 {
    if !cfg.enabled {
        return 1.0;
    }
    let total: f64 = SURGES.iter().map(|s| s.1).sum();
    let avg_surge: f64 = SURGES.iter().map(|s| s.0 * s.1).sum::<f64>() / total;
    1.0 + cfg.surge_chance * (avg_surge - 1.0) - cfg.curse_chance * 0.5
}

// ---- betting pools -----------------------------------------------------------------------------------------------------

/// What a winning stake pays out of a pool: its share of everything bet, less the house's cut.
pub fn pool_payout(stake: f64, winning_pool: f64, total_pool: f64, rake_percent: f64) -> f64 {
    if winning_pool <= 0.0 {
        return 0.0;
    }
    round2(stake / winning_pool * total_pool * (1.0 - rake_percent / 100.0))
}

/// The multiplier a stake would pay right now if its side won (for showing odds): total pool over the side's pool.
pub fn pool_odds(side_pool: f64, total_pool: f64, rake_percent: f64) -> f64 {
    if side_pool <= 0.0 {
        0.0
    } else {
        round2(total_pool / side_pool * (1.0 - rake_percent / 100.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn defaults_have_a_sane_house_edge() {
        let c = Config::default();
        let (rtp, hit) = slots_stats(&c.slots);
        assert!((0.92..=0.98).contains(&rtp), "slots rtp {rtp}");
        assert!((0.2..0.7).contains(&hit), "slots hit rate {hit}");
        let wheel = segments_expected(&c.wheel.segments);
        assert!((0.92..=0.98).contains(&wheel), "wheel rtp {wheel}");
    }

    #[test]
    fn sanitize_is_idempotent_and_repairs() {
        let mut c = Config::default();
        c.slots.symbols.clear();
        c.wheel.segments.truncate(1);
        c.plinko.min_rows = 99;
        c.mines.size = 1;
        c.betting.metrics = vec!["nonsense".into()];
        c.bounties.tax_percent = f64::NAN;
        c.sanitize();
        assert!(c.slots.symbols.len() >= 2 && c.wheel.segments.len() >= 2);
        assert!(c.plinko.min_rows <= c.plinko.max_rows && c.mines.size == 3 && c.bounties.tax_percent == 0.0);
        assert_eq!(c.betting.metrics, Betting::default().metrics);
        let once = serde_json::to_string(&c).unwrap();
        c.sanitize();
        assert_eq!(once, serde_json::to_string(&c).unwrap());
    }

    #[test]
    fn plinko_tables_hit_the_requested_return() {
        for risk in RISKS {
            for rows in 6..=16 {
                let t = plinko_table(rows, risk, 0.96);
                assert_eq!(t.len(), rows as usize + 1);
                let rtp = plinko_rtp(&t);
                assert!((rtp - 0.96).abs() < 0.01, "{risk} {rows}: {rtp}");
                assert_eq!(t[0], t[rows as usize], "symmetric");
            }
        }
        let high = plinko_table(16, "high", 0.96);
        let low = plinko_table(16, "low", 0.96);
        assert!(high[0] > low[0] && high[8] < low[8]);
    }

    #[test]
    fn plinko_drop_lands_in_range() {
        let mut rng = StdRng::seed_from_u64(7);
        for _ in 0..200 {
            let (path, slot) = plinko_drop(12, &mut rng);
            assert_eq!(path.len(), 12);
            assert!(slot <= 12);
        }
    }

    #[test]
    fn mines_odds_are_fair_minus_edge() {
        // One mine on 25 tiles: the first safe tile has a 24/25 chance, so the fair multiplier is 25/24.
        let m = mines_multiplier(25, 1, 1, 0.0, 1e9);
        assert!((m - 1.04).abs() < 0.01, "{m}");
        assert!(mines_multiplier(25, 3, 5, 0.04, 1e9) > mines_multiplier(25, 3, 4, 0.04, 1e9));
        assert!(mines_multiplier(25, 24, 1, 0.04, 1e9) > 20.0);
        assert_eq!(mines_multiplier(25, 5, 0, 0.04, 1e9), 1.0);
        assert_eq!(mines_multiplier(25, 24, 1, 0.0, 10.0), 10.0, "capped");
    }

    #[test]
    fn mines_layout_is_unique_and_in_range() {
        let mut rng = StdRng::seed_from_u64(1);
        let l = mines_layout(25, 10, &mut rng);
        assert_eq!(l.len(), 10);
        assert!(l.windows(2).all(|w| w[0] < w[1]) && l.iter().all(|t| *t < 25));
    }

    #[test]
    fn simulated_slots_match_the_exact_rtp() {
        let cfg = Slots::default();
        let (exact, _) = slots_stats(&cfg);
        let mut rng = StdRng::seed_from_u64(42);
        let n = 200_000;
        let paid: f64 = (0..n).map(|_| slots_spin(&cfg, &mut rng).multiplier).sum();
        assert!((paid / n as f64 - exact).abs() < 0.06, "{} vs {exact}", paid / n as f64);
    }

    #[test]
    fn pools_pay_out_what_was_bet() {
        // 100 on yes, 300 on no, 5% rake: yes wins and splits 380.
        let paid = pool_payout(100.0, 100.0, 400.0, 5.0);
        assert_eq!(paid, 380.0);
        assert_eq!(pool_payout(50.0, 100.0, 400.0, 5.0), 190.0);
        assert_eq!(pool_payout(10.0, 0.0, 10.0, 5.0), 0.0);
        assert_eq!(pool_odds(100.0, 400.0, 5.0), 3.8);
    }

    #[test]
    fn dice_odds_are_fair_minus_edge() {
        assert!((dice_multiplier(50.0, 0.0) - 2.0).abs() < 1e-9);
        assert!((dice_multiplier(50.0, 0.03) - 1.94).abs() < 1e-3);
        assert!(dice_multiplier(1.0, 0.03) > 90.0);
        let mut rng = StdRng::seed_from_u64(11);
        let n = 200_000;
        let paid: f64 = (0..n).map(|_| if dice_wins(dice_roll(&mut rng), 25.0, false) { dice_multiplier(25.0, 0.03) } else { 0.0 }).sum();
        assert!((paid / n as f64 - 0.97).abs() < 0.03, "{}", paid / n as f64);
        assert!(dice_wins(99.0, 5.0, true) && !dice_wins(94.0, 5.0, true) && dice_wins(4.99, 5.0, false) && !dice_wins(5.0, 5.0, false));
    }

    #[test]
    fn crash_returns_one_minus_edge_for_any_target() {
        let mut rng = StdRng::seed_from_u64(5);
        let n = 200_000;
        let points: Vec<f64> = (0..n).map(|_| crash_point(0.04, 1000.0, &mut rng)).collect();
        assert!(points.iter().all(|p| (1.0..=1000.0).contains(p)));
        for target in [1.5, 2.0, 5.0] {
            let paid: f64 = points.iter().map(|p| if *p >= target { target } else { 0.0 }).sum();
            assert!((paid / n as f64 - 0.96).abs() < 0.04, "target {target}: {}", paid / n as f64);
        }
        assert!(crash_curve(0, 100.0) == 1.0 && crash_curve(10_000, 100.0) > 3.0 && crash_curve(10_000_000, 100.0) == 100.0);
    }

    #[test]
    fn blackjack_totals_and_payouts() {
        // Card ids: ace = 0, king = 12, nine = 8, five = 4, ten = 9.
        assert_eq!(bj_total(&[0, 12]), (21, true));
        assert_eq!(bj_total(&[0, 0, 8]), (21, true));
        assert_eq!(bj_total(&[12, 11, 4]), (25, false));
        assert_eq!(bj_total(&[0, 5]), (17, true));
        assert!(bj_natural(&[0, 12]) && !bj_natural(&[4, 4, 12]));
        assert_eq!(bj_payout(&[0, 12], &[8, 7], 1.5, false), 2.5, "natural pays 3 to 2");
        assert_eq!(bj_payout(&[0, 12], &[0, 12], 1.5, false), 1.0, "both naturals push");
        assert_eq!(bj_payout(&[12, 11, 4], &[8, 7], 1.5, false), 0.0, "bust");
        assert_eq!(bj_payout(&[12, 8], &[12, 11, 4], 1.5, false), 2.0, "dealer bust");
        assert_eq!(bj_payout(&[12, 8], &[12, 8], 1.5, false), 1.0, "push");
        assert_eq!(bj_payout(&[12, 7], &[12, 8], 1.5, false), 0.0);
        assert_eq!(bj_payout(&[0, 12], &[8, 7], 1.5, true), 2.0, "a doubled 21 is a plain win");
    }

    #[test]
    fn dealer_stands_on_17_and_blackjack_house_edge_is_small() {
        let mut rng = StdRng::seed_from_u64(9);
        for _ in 0..2_000 {
            let mut d = vec![bj_draw(&mut rng)];
            bj_dealer_plays(&mut d, false, &mut rng);
            let (t, _) = bj_total(&d);
            assert!(t >= 17, "dealer stopped on {t}");
        }
        // Player stands on 17 or more, otherwise hits: a plain strategy should land near 95 to 100% return.
        let n = 100_000;
        let mut paid = 0.0;
        for _ in 0..n {
            let mut p = vec![bj_draw(&mut rng), bj_draw(&mut rng)];
            let mut d = vec![bj_draw(&mut rng), bj_draw(&mut rng)];
            if !bj_natural(&p) && !bj_natural(&d) {
                while bj_total(&p).0 < 17 {
                    p.push(bj_draw(&mut rng));
                }
                if bj_total(&p).0 <= 21 {
                    bj_dealer_plays(&mut d, false, &mut rng);
                }
            }
            paid += bj_payout(&p, &d, 1.5, false);
        }
        let rtp = paid / n as f64;
        assert!((0.9..1.0).contains(&rtp), "rtp {rtp}");
    }

    #[test]
    fn chaos_cancels_out_by_default_and_follows_the_dials() {
        let c = Chaos::default();
        assert!((chaos_factor(&c) - 1.0).abs() < 0.01, "{}", chaos_factor(&c));
        assert_eq!(chaos_factor(&Chaos { enabled: false, ..c.clone() }), 1.0);
        let mut rng = StdRng::seed_from_u64(2);
        let (mut surges, mut curses) = (0, 0);
        for _ in 0..50_000 {
            match chaos_roll(&c, &mut rng) {
                Twist::Surge(m) => {
                    assert!([1.5, 2.0, 3.0].contains(&m));
                    surges += 1
                }
                Twist::Curse => curses += 1,
                Twist::None => {}
            }
        }
        assert!((1500..2500).contains(&surges) && (2400..3600).contains(&curses), "{surges} {curses}");
        assert_eq!(chaos_roll(&Chaos { enabled: false, ..c }, &mut rng), Twist::None);
    }

    #[test]
    fn new_game_settings_are_clamped() {
        let mut c = Config::default();
        c.double.win_chance = 5.0;
        c.crash.house_edge = f64::NAN;
        c.dice.min_chance = 99.0;
        c.dice.max_chance = 1.0;
        c.coinflip.payout = 9.0;
        c.sanitize();
        assert_eq!(c.double.win_chance, 0.95);
        assert_eq!(c.crash.house_edge, 0.04);
        assert!(c.dice.max_chance >= c.dice.min_chance);
        assert_eq!(c.coinflip.payout, 2.0);
        let once = serde_json::to_string(&c).unwrap();
        c.sanitize();
        assert_eq!(once, serde_json::to_string(&c).unwrap());
    }

    #[test]
    fn weighted_pick_respects_weights() {
        let mut rng = StdRng::seed_from_u64(3);
        let mut hits = [0u32; 2];
        for _ in 0..20_000 {
            hits[pick_weighted(&[1.0, 3.0], &mut rng)] += 1;
        }
        assert!(hits[1] > hits[0] * 2);
    }

    #[test]
    fn roulette_has_single_zero_and_standard_return_for_every_selection() {
        for selection in ["red","black","odd","even","low","high","first","second","third"] {
            assert_eq!(roulette_payout(0,selection,None),Some(0.0));
            let total:f64=(0..=36).map(|n|roulette_payout(n,selection,None).unwrap()).sum();
            assert_eq!(total,36.0,"{selection}: expected return is 36/37");
        }
        for number in 0..=36 {
            assert_eq!((0..=36).map(|n|roulette_payout(n,"straight",Some(number)).unwrap()).sum::<f64>(),36.0);
        }
        assert_eq!(roulette_payout(2,"straight",Some(37)),None);
        assert_eq!(roulette_payout(2,"unknown",None),None);
    }
}

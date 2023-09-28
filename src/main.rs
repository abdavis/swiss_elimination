use std::cell::RefCell;
use std::cmp::{Ord, Ordering};
use std::rc::Rc;

fn main() {
    println!("Hello, world!");
}
type ContestantRef<S> = Rc<RefCell<Contestant<S>>>;
struct SwissElimination<S: Seed, const ALLOWED_LOSSES: usize, const FIRST_MOVE_ADVANTAGE: bool> {
    active_contestants: Vec<Rc<RefCell<Contestant<S>>>>,
    eliminated_contestants: Vec<Rc<RefCell<Contestant<S>>>>,
    round: usize,
}

impl<S: Seed, const ALLOWED_LOSSES: usize, const FIRST_MOVER: bool>
    SwissElimination<S, ALLOWED_LOSSES, FIRST_MOVER>
{
    /// generate a new round of pairings. if any games are in progress, an err variant is returned instead
    fn generate_pairings<'a>(
        &mut self,
    ) -> Result<RoundPairings<'a, S, ALLOWED_LOSSES, FIRST_MOVER>, ()> {
        if self.active_contestants.iter().any(|con| {
            if let Some(game) = con.borrow().games.last() {
                if let GameResult::InProgress = game.game_result {
                    return true;
                }
            }
            false
        }) {
            return Err(());
        }

        self.round += 1;

        let mut survived_contestants = vec![];
        for contestant in self.active_contestants.iter() {
            if self.round - contestant.borrow().win_count() >= ALLOWED_LOSSES {
                self.eliminated_contestants.push(contestant.clone())
            } else {
                survived_contestants.push(contestant)
            }
        }

        self.active_contestants.sort();

        todo!()
    }
}

struct Pairing<S: Seed, const FIRST_MOVE_ADVANTAGE: bool>(ContestantRef<S>, ContestantRef<S>);

struct RoundPairings<'a, S: Seed, const ALLOWED_LOSSES: usize, const FIRST_MOVER: bool> {
    //take a reference to the tornament to prevent new pairings while games are active
    //once all games are completed, tournament should be set to none to allow new pairings
    tournament: Option<&'a mut SwissElimination<S, ALLOWED_LOSSES, FIRST_MOVER>>,
    pairs: Vec<Pairing<S, FIRST_MOVER>>,
}

struct Contestant<S: Seed> {
    seed: S,
    name: String,
    games: Vec<Game<S>>,
}

impl<S: Seed> Contestant<S> {
    fn win_count(&self) -> usize {
        self.games
            .iter()
            .map(|game| match game.game_result {
                GameResult::Win => 1,
                GameResult::Loss => 0,
                GameResult::InProgress => 0,
            })
            .sum()
    }
    fn opponent_win_count(&self) -> usize {
        self.games
            .iter()
            .map(|game| match &game.opponent {
                Some(opponent) => (**opponent).borrow().win_count(),
                None => self.win_count(),
            })
            .sum()
    }
    fn sonneborn_berger(&self) -> usize {
        self.games
            .iter()
            .map(|game| match game.game_result {
                GameResult::Win => match &game.opponent {
                    Some(opponent) => (**opponent).borrow().win_count(),
                    None => self.win_count(),
                },
                _ => 0,
            })
            .sum()
    }
}

struct BracketContestant<S: Seed>(Rc<RefCell<Contestant<S>>>);
impl<S: Seed> Ord for BracketContestant<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (*self.0)
            .borrow()
            .opponent_win_count()
            .cmp(&(*other.0).borrow().opponent_win_count())
        {
            Ordering::Equal => match (*self.0)
                .borrow()
                .sonneborn_berger()
                .cmp(&(*other.0).borrow().sonneborn_berger())
            {
                Ordering::Equal => (*self.0).borrow().seed.cmp(&(*other.0).borrow().seed),
                order => order,
            },
            order => order,
        }
    }
}

impl<S: Seed> PartialOrd for BracketContestant<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<S: Seed> Eq for BracketContestant<S> {}
impl<S: Seed> PartialEq for BracketContestant<S> {
    fn eq(&self, other: &Self) -> bool {
        (*self.0).borrow().opponent_win_count() == (*other.0).borrow().opponent_win_count()
            && (*self.0).borrow().sonneborn_berger() == (*other.0).borrow().sonneborn_berger()
            && (*self.0).borrow().seed == (*other.0).borrow().seed
    }
}

/// represents pairing criteria that must be fulfilled if possible.
/// when any of these criteria are greater than zero, the progam should try
/// reorganizing which player recives a bye, adding extra down floaters, etc
/// these criteria are considered globally, as opposed to only for the current bracket.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct StrongPairingCriteria {
    bye_repeats: usize,
    max_pairing_repeats: usize,
    pairing_repeats: usize,
    absolute_preference_violations: usize,
}

impl std::ops::Add for StrongPairingCriteria {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            bye_repeats: self.bye_repeats + rhs.bye_repeats,
            max_pairing_repeats: self.max_pairing_repeats.max(rhs.max_pairing_repeats),
            pairing_repeats: self.pairing_repeats + rhs.pairing_repeats,
            absolute_preference_violations: self.absolute_preference_violations
                + rhs.absolute_preference_violations,
        }
    }
}

/// weaker pairing criteria. these criteria should be fulfilled as well as possible
/// for the current bracket, but aren't as vital to the pairing process.
/// most of these criteria are only considred for the current bracket. the exceptions are
/// next_unpaired_floaters and next_sum_score_paired_floaters, which are only considred
/// between two brackets at a time.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct WeakPairingCriteria {
    outgoing_floaters: usize,
    unpaired_floaters: usize,
    sum_score_paired_floaters: Reverse<usize>,
    next_unpaired_floaters: usize,
    next_sum_score_paired_floaters: Reverse<usize>,
    strong_preference_violations: usize,
    weak_preference_violations: usize,
}

struct Game<S: Seed> {
    game_result: GameResult,
    advantage: Option<FirstMoverAdvantage>,
    opponent: Option<Rc<RefCell<Contestant<S>>>>,
}
enum GameResult {
    Win,
    Loss,
    InProgress,
}

enum FirstMoverAdvantage {
    First,
    Last,
}

impl<S: Seed> Ord for Contestant<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.win_count().cmp(&other.win_count()) {
            Ordering::Equal => match self.opponent_win_count().cmp(&other.opponent_win_count()) {
                Ordering::Equal => match self.sonneborn_berger().cmp(&other.sonneborn_berger()) {
                    Ordering::Equal => self.seed.cmp(&other.seed),
                    order => order,
                },
                order => order,
            },
            order => order,
        }
    }
}

impl<S: Seed> PartialOrd for Contestant<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S: Seed> Eq for Contestant<S> {}

impl<S: Seed> PartialEq for Contestant<S> {
    fn eq(&self, other: &Self) -> bool {
        self.win_count() == other.win_count()
            && self.opponent_win_count() == other.opponent_win_count()
            && self.sonneborn_berger() == other.sonneborn_berger()
            && self.seed == other.seed
    }
}

trait Seed: Ord {}

#[derive(Ord, PartialOrd, PartialEq, Eq)]
struct Elo {
    elo: Option<usize>,
    rand: usize,
}
impl Seed for Elo {}

use std::cmp::Reverse;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Rank {
    rank: Option<Reverse<usize>>,
    rand: usize,
}
impl Seed for Rank {}

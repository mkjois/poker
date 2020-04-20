use std::cmp::Ordering;
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::io;

use rand::Rng;

fn main() {
    println!("\nPoker hand builder!");

    let card_repr: HashMap<_, _> = [
        ("2d", Card::_2d), ("2c", Card::_2c), ("2h", Card::_2h), ("2s", Card::_2s),
        ("3d", Card::_3d), ("3c", Card::_3c), ("3h", Card::_3h), ("3s", Card::_3s),
        ("4d", Card::_4d), ("4c", Card::_4c), ("4h", Card::_4h), ("4s", Card::_4s),
        ("5d", Card::_5d), ("5c", Card::_5c), ("5h", Card::_5h), ("5s", Card::_5s),
        ("6d", Card::_6d), ("6c", Card::_6c), ("6h", Card::_6h), ("6s", Card::_6s),
        ("7d", Card::_7d), ("7c", Card::_7c), ("7h", Card::_7h), ("7s", Card::_7s),
        ("8d", Card::_8d), ("8c", Card::_8c), ("8h", Card::_8h), ("8s", Card::_8s),
        ("9d", Card::_9d), ("9c", Card::_9c), ("9h", Card::_9h), ("9s", Card::_9s),
        ("Td", Card::_Td), ("Tc", Card::_Tc), ("Th", Card::_Th), ("Ts", Card::_Ts),
        ("Jd", Card::_Jd), ("Jc", Card::_Jc), ("Jh", Card::_Jh), ("Js", Card::_Js),
        ("Qd", Card::_Qd), ("Qc", Card::_Qc), ("Qh", Card::_Qh), ("Qs", Card::_Qs),
        ("Kd", Card::_Kd), ("Kc", Card::_Kc), ("Kh", Card::_Kh), ("Ks", Card::_Ks),
        ("Ad", Card::_Ad), ("Ac", Card::_Ac), ("Ah", Card::_Ah), ("As", Card::_As),
    ].iter()
        .map(|(s, card)| ((*s).to_owned(), card.clone()))
        .collect();

    let streets = [("preflop", 2), ("flop", 3), ("turn", 1), ("river", 1)];
    let mut building_hand = BuildingHand::new();

    for &(street, ncards) in streets.iter() {
        loop {
            let mut input = String::new();
            println!("\nAdd {} {} card(s):", ncards, street);
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read input");

            let cards = match normalize_input_cards(&input, ncards) {
                Ok(Some(cards)) => cards,
                Ok(None) => {
                    println!();
                    return;
                },
                Err(e) => {
                    println!("{}", e.msg);
                    continue;
                },
            };

            println!("Cards input: {}", cards.join(" "));
            let cards: Vec<_> = cards.iter()
                .map(|card| card_repr.get(card).expect("Bug! Unknown card mapping"))
                .collect();

            let mut street_hand = building_hand.clone();
            let mut errors = 0;

            for card in cards {
                errors += match street_hand.add_card(card, true) {
                    Some(e) => {
                        println!("{}", e.msg);
                        1
                    },
                    None => 0,
                }
            }

            if errors > 0 {
                continue;
            }

            building_hand = street_hand;
            break;
        }

        println!("Building hand: {:#018x}", building_hand.0);
    }

    println!();
}

// TODO: play nice with UTF-8?
fn normalize_input_cards(cards: &str, n_expected: usize) -> Result<Option<Vec<String>>, AppError> {

    let cards = cards.trim();
    let mut current_rank = '0';
    let mut normalized = Vec::new();

    match cards.to_lowercase().as_str() {
        "done" | "exit" | "quit" => return Ok(None),
        _ => (),
    }

    for c in cards.chars() {
        if !c.is_ascii() {
            return Err(AppError::of(format!("Invalid input, non-ASCII: {}", cards)));
        }

        if normalized.len() > n_expected {
            return Err(AppError::of(format!("Invalid cards, {} required, {}+ given: {}", n_expected, normalized.len(), cards)));
        }

        let c = c.to_ascii_uppercase();
        match c {
            'A' | 'K' | 'Q' | 'J' | 'T' | '9' | '8' | '7' | '6' | '5' | '4' | '3' | '2' => {
                if current_rank == '0' {
                    current_rank = c;
                } else {
                    return Err(AppError::of(format!("Invalid cards, rank without suit: {}", cards)));
                }
            },

            'D' | 'C' | 'H' | 'S' => {
                if current_rank == '0' {
                    return Err(AppError::of(format!("Invalid cards, suit without rank: {}", cards)));
                } else {
                    let mut card = String::new();
                    card.push(current_rank);
                    card.push(c.to_ascii_lowercase());
                    normalized.push(card);
                    current_rank = '0';
                }
            },

            ' ' => (),
            _ => return Err(AppError::of(format!("Invalid input: {}", cards))),
        }
    }

    if current_rank != '0' {
        Err(AppError::of(format!("Invalid cards, rank without suit: {}", cards)))
    } else if normalized.len() != n_expected {
        Err(AppError::of(format!("Invalid cards, {} required, {} given: {}", n_expected, normalized.len(), cards)))
    } else {
        Ok(Some(normalized))
    }
}

#[derive(Debug, Clone)]
struct AppError {
    msg: String,
}

impl AppError {
    fn of(msg: String) -> AppError {
        AppError { msg }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[ERROR] {}", self.msg)
    }
}

impl error::Error for AppError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

/* |............|....................................................|
 * - 12 bits: TBD (8 bits: number of cards in hand?)
 * - 52 bits: card bit positions, see enum Card below
 */
#[derive(Debug, Clone, Eq, PartialEq)]
struct BuildingHand(u64);

/* |....|....|....|....................................................|
 * -  4 bits: made hand classifier, see enum Order below
 * -  4 bits: top card rank, e.g. top card rank of made hand, rank of pair/trip/quad, rank of boat
 * -  4 bits: 2nd card rank, e.g. 2nd card rank of made hand, 2nd of two pairs, filler of boat
 * - 52 bits: card bit positions, see enum Card below
 */
#[derive(Debug, Clone)]
struct RealizedHand(u64);

/* |....|..|......|....|....|....|....|....|
 * -   4 bits: made hand classifier, see enum Order below
 * -   2 bits: suit for flush hands, see enum Suit below
 * -   6 bits: TBD
 * - 5x4 bits: card ranks in descending order from msb to lsb
 */
#[derive(Debug, Clone)]
struct ShowdownHand(u32);

impl BuildingHand {
    fn new() -> BuildingHand {
        BuildingHand(0)
    }

    fn add_card(&mut self, card: &Card, err_on_duplicate: bool) -> Option<AppError> {
        let bit = card.clone() as u64;
        if err_on_duplicate && self.0 & bit != 0 {
            Some(AppError::of(format!("Duplicate card: {}", card.clone())))
        } else {
            self.0 |= bit;
            None
        }
    }

    fn to_realized_hand(&self) -> RealizedHand {
        RealizedHand(0) // TODO
    }

    fn to_showdown_hand(&self) -> ShowdownHand {
        self.to_realized_hand().to_showdown_hand()
    }
}

impl RealizedHand {
    fn to_building_hand(&self) -> BuildingHand {
        BuildingHand(0) // TODO
    }

    fn to_showdown_hand(&self) -> ShowdownHand {
        ShowdownHand(0) // TODO
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Eq, PartialEq)]
enum Order {
    None, High, Pair, Twop, Trip, Strt, Flsh, Boat, Quad, Stfl
}

#[repr(u64)]
#[derive(Debug, Clone, Eq, PartialEq)]
enum Card {
    _2d = 0x0000000000000001, _2c = 0x0000000000000002, _2h = 0x0000000000000004, _2s = 0x0000000000000008,
    _3d = 0x0000000000000010, _3c = 0x0000000000000020, _3h = 0x0000000000000040, _3s = 0x0000000000000080,
    _4d = 0x0000000000000100, _4c = 0x0000000000000200, _4h = 0x0000000000000400, _4s = 0x0000000000000800,
    _5d = 0x0000000000001000, _5c = 0x0000000000002000, _5h = 0x0000000000004000, _5s = 0x0000000000008000,
    _6d = 0x0000000000010000, _6c = 0x0000000000020000, _6h = 0x0000000000040000, _6s = 0x0000000000080000,
    _7d = 0x0000000000100000, _7c = 0x0000000000200000, _7h = 0x0000000000400000, _7s = 0x0000000000800000,
    _8d = 0x0000000001000000, _8c = 0x0000000002000000, _8h = 0x0000000004000000, _8s = 0x0000000008000000,
    _9d = 0x0000000010000000, _9c = 0x0000000020000000, _9h = 0x0000000040000000, _9s = 0x0000000080000000,
    _Td = 0x0000000100000000, _Tc = 0x0000000200000000, _Th = 0x0000000400000000, _Ts = 0x0000000800000000,
    _Jd = 0x0000001000000000, _Jc = 0x0000002000000000, _Jh = 0x0000004000000000, _Js = 0x0000008000000000,
    _Qd = 0x0000010000000000, _Qc = 0x0000020000000000, _Qh = 0x0000040000000000, _Qs = 0x0000080000000000,
    _Kd = 0x0000100000000000, _Kc = 0x0000200000000000, _Kh = 0x0000400000000000, _Ks = 0x0000800000000000,
    _Ad = 0x0001000000000000, _Ac = 0x0002000000000000, _Ah = 0x0004000000000000, _As = 0x0008000000000000,
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            Card::_2d => "2d", Card::_2c => "2c", Card::_2h => "2h", Card::_2s => "2s",
            Card::_3d => "3d", Card::_3c => "3c", Card::_3h => "3h", Card::_3s => "3s",
            Card::_4d => "4d", Card::_4c => "4c", Card::_4h => "4h", Card::_4s => "4s",
            Card::_5d => "5d", Card::_5c => "5c", Card::_5h => "5h", Card::_5s => "5s",
            Card::_6d => "6d", Card::_6c => "6c", Card::_6h => "6h", Card::_6s => "6s",
            Card::_7d => "7d", Card::_7c => "7c", Card::_7h => "7h", Card::_7s => "7s",
            Card::_8d => "8d", Card::_8c => "8c", Card::_8h => "8h", Card::_8s => "8s",
            Card::_9d => "9d", Card::_9c => "9c", Card::_9h => "9h", Card::_9s => "9s",
            Card::_Td => "Td", Card::_Tc => "Tc", Card::_Th => "Th", Card::_Ts => "Ts",
            Card::_Jd => "Jd", Card::_Jc => "Jc", Card::_Jh => "Jh", Card::_Js => "Js",
            Card::_Qd => "Qd", Card::_Qc => "Qc", Card::_Qh => "Qh", Card::_Qs => "Qs",
            Card::_Kd => "Kd", Card::_Kc => "Kc", Card::_Kh => "Kh", Card::_Ks => "Ks",
            Card::_Ad => "Ad", Card::_Ac => "Ac", Card::_Ah => "Ah", Card::_As => "As",
        })
    }
}

/*
#[repr(u8)]
#[derive(Debug, Clone, Eq, PartialEq)]
enum Rank {
    _2 = 1, _3, _4, _5, _6, _7, _8, _9, _T, _J, _Q, _K, _A
}

#[repr(u8)]
#[derive(Debug, Clone, Eq, PartialEq)]
enum Suit {
    _D, _C, _H, _S
}

impl Card {
    fn from(rank: Rank, suit: Suit) -> Card {
        match (rank, suit) {
            (Rank::_2, Suit::_D) => Card::_2d, (Rank::_2, Suit::_C) => Card::_2c,
            (Rank::_2, Suit::_H) => Card::_2h, (Rank::_2, Suit::_S) => Card::_2s,
            (Rank::_3, Suit::_D) => Card::_3d, (Rank::_3, Suit::_C) => Card::_3c,
            (Rank::_3, Suit::_H) => Card::_3h, (Rank::_3, Suit::_S) => Card::_3s,
            (Rank::_4, Suit::_D) => Card::_4d, (Rank::_4, Suit::_C) => Card::_4c,
            (Rank::_4, Suit::_H) => Card::_4h, (Rank::_4, Suit::_S) => Card::_4s,
            (Rank::_5, Suit::_D) => Card::_5d, (Rank::_5, Suit::_C) => Card::_5c,
            (Rank::_5, Suit::_H) => Card::_5h, (Rank::_5, Suit::_S) => Card::_5s,
            (Rank::_6, Suit::_D) => Card::_6d, (Rank::_6, Suit::_C) => Card::_6c,
            (Rank::_6, Suit::_H) => Card::_6h, (Rank::_6, Suit::_S) => Card::_6s,
            (Rank::_7, Suit::_D) => Card::_7d, (Rank::_7, Suit::_C) => Card::_7c,
            (Rank::_7, Suit::_H) => Card::_7h, (Rank::_7, Suit::_S) => Card::_7s,
            (Rank::_8, Suit::_D) => Card::_8d, (Rank::_8, Suit::_C) => Card::_8c,
            (Rank::_8, Suit::_H) => Card::_8h, (Rank::_8, Suit::_S) => Card::_8s,
            (Rank::_9, Suit::_D) => Card::_9d, (Rank::_9, Suit::_C) => Card::_9c,
            (Rank::_9, Suit::_H) => Card::_9h, (Rank::_9, Suit::_S) => Card::_9s,
            (Rank::_T, Suit::_D) => Card::_Td, (Rank::_T, Suit::_C) => Card::_Tc,
            (Rank::_T, Suit::_H) => Card::_Th, (Rank::_T, Suit::_S) => Card::_Ts,
            (Rank::_J, Suit::_D) => Card::_Jd, (Rank::_J, Suit::_C) => Card::_Jc,
            (Rank::_J, Suit::_H) => Card::_Jh, (Rank::_J, Suit::_S) => Card::_Js,
            (Rank::_Q, Suit::_D) => Card::_Qd, (Rank::_Q, Suit::_C) => Card::_Qc,
            (Rank::_Q, Suit::_H) => Card::_Qh, (Rank::_Q, Suit::_S) => Card::_Qs,
            (Rank::_K, Suit::_D) => Card::_Kd, (Rank::_K, Suit::_C) => Card::_Kc,
            (Rank::_K, Suit::_H) => Card::_Kh, (Rank::_K, Suit::_S) => Card::_Ks,
            (Rank::_A, Suit::_D) => Card::_Ad, (Rank::_A, Suit::_C) => Card::_Ac,
            (Rank::_A, Suit::_H) => Card::_Ah, (Rank::_A, Suit::_S) => Card::_As,
        }
    }

    fn rank(self) -> Rank {
        match msb(self as u64).unwrap() / 4 {
             0 => Rank::_2,
             1 => Rank::_3,
             2 => Rank::_4,
             3 => Rank::_5,
             4 => Rank::_6,
             5 => Rank::_7,
             6 => Rank::_8,
             7 => Rank::_9,
             8 => Rank::_T,
             9 => Rank::_J,
            10 => Rank::_Q,
            11 => Rank::_K,
            12 => Rank::_A,
            _ => panic!("Bug! Unknown card"),
        }
    }

    fn suit(self) -> Suit {
        match lsb(self as u64).unwrap() % 4 {
            0 => Suit::_D,
            1 => Suit::_C,
            2 => Suit::_H,
            3 => Suit::_S,
            _ => panic!("Bug! Unknown card"),
        }
    }
}

fn msb(x: u64) -> Option<u8> {
    if x == 0 { return None; }

    let mut b = 0;
    let mut y = x;
    if y & 0xffffffff00000000 != 0 { b += 32; y >>= 32; }
    if y & 0x00000000ffff0000 != 0 { b += 16; y >>= 16; }
    if y & 0x000000000000ff00 != 0 { b +=  8; y >>=  8; }
    if y & 0x00000000000000f0 != 0 { b +=  4; y >>=  4; }
    if y & 0x000000000000000c != 0 { b +=  2; y >>=  2; }
    if y & 0x0000000000000002 != 0 { b +=  1; }
    Some(b)
}

fn lsb(x: u64) -> Option<u8> {
    if x == 0 { return None; }

    let mut b = 0;
    let mut y = x;
    if y & 0xffffffff == 0 { b += 32; y >>= 32; }
    if y & 0x0000ffff == 0 { b += 16; y >>= 16; }
    if y & 0x000000ff == 0 { b +=  8; y >>=  8; }
    if y & 0x0000000f == 0 { b +=  4; y >>=  4; }
    if y & 0x00000003 == 0 { b +=  2; y >>=  2; }
    if y & 0x00000001 == 0 { b +=  1; }
    Some(b)
}
*/

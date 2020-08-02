use std::cmp::Ordering;
use std::error;
use std::fmt;

// TODO: play nice with UTF-8?
pub fn normalize_input_cards(cards: &str, n_expected: usize) -> Result<Option<Vec<String>>, Error> {

    let cards = cards.trim();
    let mut current_rank = '0';
    let mut normalized = Vec::new();

    match cards.to_lowercase().as_str() {
        "done" | "exit" | "quit" => return Ok(None),
        _ => (),
    }

    for c in cards.chars() {
        if !c.is_ascii() {
            return Err(Error::of(format!("Invalid input, non-ASCII: {}", cards)));
        }

        if normalized.len() > n_expected {
            return Err(Error::of(format!("Invalid cards, {} required, {}+ given: {}", n_expected, normalized.len(), cards)));
        }

        let c = c.to_ascii_uppercase();
        match c {
            'A' | 'K' | 'Q' | 'J' | 'T' | '9' | '8' | '7' | '6' | '5' | '4' | '3' | '2' => {
                if current_rank == '0' {
                    current_rank = c;
                } else {
                    return Err(Error::of(format!("Invalid cards, rank without suit: {}", cards)));
                }
            },

            'D' | 'C' | 'H' | 'S' => {
                if current_rank == '0' {
                    return Err(Error::of(format!("Invalid cards, suit without rank: {}", cards)));
                } else {
                    let mut card = String::new();
                    card.push(current_rank);
                    card.push(c.to_ascii_lowercase());
                    normalized.push(card);
                    current_rank = '0';
                }
            },

            ' ' => (),
            _ => return Err(Error::of(format!("Invalid input: {}", cards))),
        }
    }

    if current_rank != '0' {
        Err(Error::of(format!("Invalid cards, rank without suit: {}", cards)))
    } else if normalized.len() != n_expected {
        Err(Error::of(format!("Invalid cards, {} required, {} given: {}", n_expected, normalized.len(), cards)))
    } else {
        Ok(Some(normalized))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Error {
    pub msg: String,
}

impl Error {
    fn of(msg: String) -> Error {
        Error { msg }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[ERROR] {}", self.msg)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

/* |............|....................................................|
 * - 12 bits: TBD (8 bits: number of cards in hand?)
 * - 52 bits: card bit positions, see enum Card below
 */
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BuildingHand(u64);

/* |....|....|....|....................................................|
 * -  4 bits: made hand classifier, see enum Order below
 * -  4 bits: top card rank, e.g. top card rank of made hand, rank of pair/trip/quad, rank of boat
 * -  4 bits: 2nd card rank, e.g. 2nd card rank of made hand, 2nd of two pairs, filler of boat
 * - 52 bits: card bit positions, see enum Card below
 */
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RealizedHand(u64);

/* |....|..|......|....|....|....|....|....|
 * -   4 bits: made hand classifier, see enum Order below
 * -   2 bits: suit for flush hands, see enum Suit below
 * -   6 bits: TBD
 * - 5x4 bits: card ranks in descending order from msb to lsb
 */
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShowdownHand(u32);

impl BuildingHand {
    pub fn new() -> BuildingHand {
        BuildingHand(0)
    }

    pub fn add_card(&mut self, card: &Card, err_on_duplicate: bool) -> Option<Error> {
        let bit = card.clone() as u64;
        if err_on_duplicate && self.0 & bit != 0 {
            Some(Error::of(format!("Duplicate card: {}", card.clone())))
        } else {
            self.0 |= bit;
            None
            // TODO: add hole cards or num cards, and update comment on representation
        }
    }

    fn to_realized_hand(&self) -> RealizedHand {
        if self.0 & 0x000fffffffffffff == 0 {
            return self.new_realized_hand(Order::None, 0, 0);
        }

        let mask_base = 0x000f000000000000;
        let mut straight_flush_counter = 0u8;
        let mut straight_flush_buffer = 0b1111u8;
        let mut straight_counter = 0u8;
        let mut best_straight = Option::<u64>::None;
        let mut best_flush = Option::<u64>::None;
        let mut best_quads = Option::<u64>::None;
        let mut best_trips = Option::<u64>::None;
        let mut best_pair1 = Option::<u64>::None;
        let mut best_pair2 = Option::<u64>::None;
        let mut best_ranks = [0u64; 5];
        let mut best_ranks_idx = 0usize;

        // extra index i=13 for wheel evaluation
        for i in 0u8..14u8 {

            let mask_shift = i % 13 << 2;
            let mask_comp = 48 - mask_shift;
            let mask = mask_base >> mask_shift;
            let rank = 1 + (mask_comp >> 2) as u64 % 13;
            let quartet = ((self.0 & mask) >> mask_comp) as u8;

            straight_flush_buffer &= quartet;
            if straight_flush_buffer > 0 {
                straight_flush_counter += 1;
                if straight_flush_counter == 5 {
                    let top_rank = 1 + (rank + 3) % 13;
                    return self.new_realized_hand(Order::Stfl, top_rank, top_rank - 1);
                }
            } else {
                straight_flush_counter = 0;
                straight_flush_buffer = 0b1111;
            }

            if quartet != 0 {
                if best_ranks_idx < 5 {
                    best_ranks[best_ranks_idx] = rank;
                    best_ranks_idx += 1;
                }

                straight_counter += 1;
                if let (5, None) = (straight_counter, best_straight) {
                    let top_rank = 1 + (rank + 3) % 13;
                    best_straight = Some(top_rank)
                }
            } else {
                straight_counter = 0;
            }

            // wheel evaluation no longer needed at this point
            if i == 13 {
                continue;
            }

            match quartet {
                0b1111 => match best_quads {
                    None => best_quads = Some(rank),
                    Some(_) => match best_trips {
                        None => best_trips = Some(rank),
                        Some(_) => match best_pair1 {
                            None => best_pair1 = Some(rank),
                            Some(_) => if let None = best_pair2 {
                                best_pair2 = Some(rank);
                            },
                        },
                    },
                },

                0b1110 | 0b1101 | 0b1011 | 0b0111 => match best_trips {
                    None => best_trips = Some(rank),
                    Some(_) => match best_pair1 {
                        None => best_pair1 = Some(rank),
                        Some(_) => if let None = best_pair2 {
                            best_pair2 = Some(rank);
                        },
                    },
                },

                0b1100 | 0b1010 | 0b1001 | 0b0110 | 0b0101 | 0b0011 => match best_pair1 {
                    None => best_pair1 = Some(rank),
                    Some(_) => if let None = best_pair2 {
                        best_pair2 = Some(rank);
                    },
                },
                _ => {},
            }
        }

        if let Some(rank1) = best_quads {
            return self.new_realized_hand(Order::Quad, rank1, self.find_rank2(&best_ranks, rank1));
        }

        if let (Some(rank1), Some(rank2)) = (best_trips, best_pair1) {
            return self.new_realized_hand(Order::Boat, rank1, rank2);
        }

        // flush algorithm, based on Kernighan bit counting
        for (mask, offset) in &SUIT_MASKS {

            // normalize bit positions for easy ordering comparison
            let suited_group = (self.0 & (mask.clone() as u64)) >> offset;
            let mut suited_cards = suited_group;

            // take away 3 cards, ensure flush would have at least 2 left
            for _ in 0u8..3u8 {
                if suited_cards == 0 {
                    break; // avoid underflow
                }
                suited_cards = suited_cards & (suited_cards - 1);
            }

            // if at most 4 cards of one suit, skip to next suit
            if suited_cards == 0 || suited_cards & (suited_cards - 1) == 0 {
                continue;
            }

            best_flush = Some(match best_flush {
                None => suited_group,
                Some(other) => if suited_group > other { suited_group } else { other },
            });
        }

        if let Some(suited_group) = best_flush {
            let ms_bit = msb(suited_group).unwrap();
            let suited_group = suited_group & !(1u64 << ms_bit);
            let rank1 = 1 + (ms_bit >> 2) as u64;
            let rank2 = 1 + (msb(suited_group).unwrap() >> 2) as u64;
            return self.new_realized_hand(Order::Flsh, rank1, rank2);
        }

        if let Some(rank1) = best_straight {
            return self.new_realized_hand(Order::Strt, rank1, rank1 - 1);
        }

        if let Some(rank1) = best_trips {
            return self.new_realized_hand(Order::Trip, rank1, self.find_rank2(&best_ranks, rank1));
        }

        if let Some(rank1) = best_pair1 {
            return match best_pair2 {
                Some(rank2) => self.new_realized_hand(Order::Twop, rank1, rank2),
                None => self.new_realized_hand(Order::Pair, rank1, self.find_rank2(&best_ranks, rank1)),
            }
        }

        let rank1 = self.find_rank2(&best_ranks, 0);
        return self.new_realized_hand(Order::High, rank1, self.find_rank2(&best_ranks, rank1));
    }

    fn new_realized_hand(&self, order: Order, rank1: u64, rank2: u64) -> RealizedHand {
        RealizedHand((order as u64) << 60 | rank1 << 56 | rank2 << 52 | (self.0 & 0x000fffffffffffff))
    }

    fn find_rank2(&self, best_ranks: &[u64; 5], rank1: u64) -> u64 {
        for &rank2 in best_ranks {
            if rank2 != rank1 {
                return rank2;
            }
        }
        0
    }

    fn to_showdown_hand(&self) -> ShowdownHand {
        self.to_realized_hand().to_showdown_hand()
    }
}

impl RealizedHand {
    fn to_building_hand(&self) -> BuildingHand {
        BuildingHand(self.0 & 0x000fffffffffffff)
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
pub enum Card {
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
    _Md = 0x0001111111111111, _Mc = 0x0002222222222222, _Mh = 0x0004444444444444, _Ms = 0x0008888888888888,
}

const SUIT_MASKS: [(Card, u8); 4] = [(Card::_Md, 0), (Card::_Mc, 1), (Card::_Mh, 2), (Card::_Ms, 3)];

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
            Card::_Md =>  "d", Card::_Mc =>  "c", Card::_Mh =>  "h", Card::_Ms =>  "s",
        })
    }
}

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

    fn rank(&self) -> Rank {
        match msb(self.clone() as u64).expect("Bug! Unknown card") / 4 {
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

    fn suit(&self) -> Suit {
        match lsb(self.clone() as u64).expect("Bug! Unknown card") % 4 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msb_with_all_cases() {
        assert_eq!(None, msb(0));
        assert_eq!(Some(0), msb(1));
        assert_eq!(Some(1), msb(2));
        assert_eq!(Some(1), msb(3));
        assert_eq!(Some(2), msb(4));
        assert_eq!(Some(2), msb(5));
        assert_eq!(Some(2), msb(6));
        assert_eq!(Some(2), msb(7));
        assert_eq!(Some(3), msb(8));
        assert_eq!(Some(3), msb(9));
        assert_eq!(Some(63), msb(u64::MAX));
        assert_eq!(Some(63), msb(u64::MAX - 1));
        assert_eq!(Some(62), msb(u64::MAX / 2));
        assert_eq!(Some(63), msb(u64::MAX / 2 + 1));
    }

    #[test]
    fn lsb_with_all_cases() {
        assert_eq!(None, lsb(0));
        assert_eq!(Some(0), lsb(1));
        assert_eq!(Some(1), lsb(2));
        assert_eq!(Some(0), lsb(3));
        assert_eq!(Some(2), lsb(4));
        assert_eq!(Some(0), lsb(5));
        assert_eq!(Some(1), lsb(6));
        assert_eq!(Some(0), lsb(7));
        assert_eq!(Some(3), lsb(8));
        assert_eq!(Some(0), lsb(9));
        assert_eq!(Some(0), lsb(u64::MAX));
        assert_eq!(Some(1), lsb(u64::MAX - 1));
        assert_eq!(Some(0), lsb(u64::MAX - 2));
        assert_eq!(Some(2), lsb(u64::MAX - 3));
        assert_eq!(Some(0), lsb(u64::MAX - 4));
        assert_eq!(Some(1), lsb(u64::MAX - 5));
        assert_eq!(Some(0), lsb(u64::MAX - 6));
        assert_eq!(Some(3), lsb(u64::MAX - 7));
        assert_eq!(Some(0), lsb(u64::MAX - 8));
    }

    #[test]
    fn normalize_input_cards_with_quit() {
        assert_eq!(Ok(None), normalize_input_cards("done", 0));
        assert_eq!(Ok(None), normalize_input_cards("exit", 0));
        assert_eq!(Ok(None), normalize_input_cards("quit", 0));
        assert_eq!(Ok(None), normalize_input_cards("DONE", 0));
        assert_eq!(Ok(None), normalize_input_cards("EXIT", 0));
        assert_eq!(Ok(None), normalize_input_cards("QUIT", 0));
    }

    #[test]
    fn normalize_input_cards_with_invalid_input() {
        assert_eq!(Ok(None), normalize_input_cards("done", 0));
        assert_eq!(Ok(None), normalize_input_cards("EXIT", 0));
        assert_eq!(Ok(None), normalize_input_cards("qUiT", 0));
    }

    #[test]
    fn building_hand_to_realized_hand_with_straight_flush() {

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_As, true);
        hand.add_card(&Card::_Ts, true);
        hand.add_card(&Card::_Qs, true);
        hand.add_card(&Card::_Js, true);
        hand.add_card(&Card::_Ks, true);
        assert_eq!(hand.new_realized_hand(Order::Stfl, Rank::_A as u64, Rank::_K as u64), hand.to_realized_hand());

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_8c, true);
        hand.add_card(&Card::_5d, true);
        hand.add_card(&Card::_2d, true);
        hand.add_card(&Card::_Ad, true);
        hand.add_card(&Card::_Jh, true);
        hand.add_card(&Card::_4d, true);
        hand.add_card(&Card::_3d, true);
        assert_eq!(hand.new_realized_hand(Order::Stfl, Rank::_5 as u64, Rank::_4 as u64), hand.to_realized_hand());
    }

    #[test]
    fn building_hand_to_realized_hand_with_quads() {

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_Qd, true);
        hand.add_card(&Card::_Qc, true);
        hand.add_card(&Card::_3h, true);
        hand.add_card(&Card::_Qh, true);
        hand.add_card(&Card::_Qs, true);
        assert_eq!(hand.new_realized_hand(Order::Quad, Rank::_Q as u64, Rank::_3 as u64), hand.to_realized_hand());

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_7d, true);
        hand.add_card(&Card::_7c, true);
        hand.add_card(&Card::_Jd, true);
        hand.add_card(&Card::_Jc, true);
        hand.add_card(&Card::_7h, true);
        hand.add_card(&Card::_7s, true);
        hand.add_card(&Card::_Jh, true);
        hand.add_card(&Card::_Js, true);
        hand.add_card(&Card::_9s, true);
        assert_eq!(hand.new_realized_hand(Order::Quad, Rank::_J as u64, Rank::_9 as u64), hand.to_realized_hand());
    }

    #[test]
    fn building_hand_to_realized_hand_with_full_house() {
        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_Kd, true);
        hand.add_card(&Card::_8c, true);
        hand.add_card(&Card::_9h, true);
        hand.add_card(&Card::_8s, true);
        hand.add_card(&Card::_Qd, true);
        hand.add_card(&Card::_8d, true);
        hand.add_card(&Card::_Qh, true);
        assert_eq!(hand.new_realized_hand(Order::Boat, Rank::_8 as u64, Rank::_Q as u64), hand.to_realized_hand());
    }

    #[test]
    fn building_hand_to_realized_hand_with_flush() {

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_6c, true);
        hand.add_card(&Card::_Td, true);
        hand.add_card(&Card::_3c, true);
        hand.add_card(&Card::_4s, true);
        hand.add_card(&Card::_Ac, true);
        hand.add_card(&Card::_2c, true);
        hand.add_card(&Card::_5c, true);
        assert_eq!(hand.new_realized_hand(Order::Flsh, Rank::_A as u64, Rank::_6 as u64), hand.to_realized_hand());

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_Td, true);
        hand.add_card(&Card::_Kc, true);
        hand.add_card(&Card::_3c, true);
        hand.add_card(&Card::_5d, true);
        hand.add_card(&Card::_9c, true);
        hand.add_card(&Card::_Kd, true);
        hand.add_card(&Card::_Jc, true);
        hand.add_card(&Card::_3d, true);
        hand.add_card(&Card::_9d, true);
        hand.add_card(&Card::_5c, true);
        assert_eq!(hand.new_realized_hand(Order::Flsh, Rank::_K as u64, Rank::_J as u64), hand.to_realized_hand());

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_Tc, true);
        hand.add_card(&Card::_Kd, true);
        hand.add_card(&Card::_3d, true);
        hand.add_card(&Card::_5c, true);
        hand.add_card(&Card::_9d, true);
        hand.add_card(&Card::_Kc, true);
        hand.add_card(&Card::_Jd, true);
        hand.add_card(&Card::_3c, true);
        hand.add_card(&Card::_9c, true);
        hand.add_card(&Card::_5d, true);
        assert_eq!(hand.new_realized_hand(Order::Flsh, Rank::_K as u64, Rank::_J as u64), hand.to_realized_hand());

    }

    #[test]
    fn building_hand_to_realized_hand_with_straight() {

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_8h, true);
        hand.add_card(&Card::_Td, true);
        hand.add_card(&Card::_9h, true);
        hand.add_card(&Card::_8s, true);
        hand.add_card(&Card::_Ad, true);
        hand.add_card(&Card::_Jh, true);
        hand.add_card(&Card::_Qh, true);
        assert_eq!(hand.new_realized_hand(Order::Strt, Rank::_Q as u64, Rank::_J as u64), hand.to_realized_hand());

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_8h, true);
        hand.add_card(&Card::_Td, true);
        hand.add_card(&Card::_9h, true);
        hand.add_card(&Card::_6s, true);
        hand.add_card(&Card::_7d, true);
        hand.add_card(&Card::_Jh, true);
        hand.add_card(&Card::_Qh, true);
        assert_eq!(hand.new_realized_hand(Order::Strt, Rank::_Q as u64, Rank::_J as u64), hand.to_realized_hand());

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_Ac, true);
        hand.add_card(&Card::_Td, true);
        hand.add_card(&Card::_3c, true);
        hand.add_card(&Card::_4s, true);
        hand.add_card(&Card::_Ad, true);
        hand.add_card(&Card::_2c, true);
        hand.add_card(&Card::_5c, true);
        assert_eq!(hand.new_realized_hand(Order::Strt, Rank::_5 as u64, Rank::_4 as u64), hand.to_realized_hand());

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_6c, true);
        hand.add_card(&Card::_Td, true);
        hand.add_card(&Card::_3c, true);
        hand.add_card(&Card::_4s, true);
        hand.add_card(&Card::_Ad, true);
        hand.add_card(&Card::_2c, true);
        hand.add_card(&Card::_5c, true);
        assert_eq!(hand.new_realized_hand(Order::Strt, Rank::_6 as u64, Rank::_5 as u64), hand.to_realized_hand());

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_Jh, true);
        hand.add_card(&Card::_Th, true);
        hand.add_card(&Card::_9d, true);
        hand.add_card(&Card::_8d, true);
        hand.add_card(&Card::_7h, true);
        hand.add_card(&Card::_5d, true);
        hand.add_card(&Card::_2d, true);
        assert_eq!(hand.new_realized_hand(Order::Strt, Rank::_J as u64, Rank::_T as u64), hand.to_realized_hand());
    }

    #[test]
    fn building_hand_to_realized_hand_with_trips() {
        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_Kd, true);
        hand.add_card(&Card::_8c, true);
        hand.add_card(&Card::_9h, true);
        hand.add_card(&Card::_8s, true);
        hand.add_card(&Card::_Ad, true);
        hand.add_card(&Card::_8d, true);
        hand.add_card(&Card::_Qh, true);
        assert_eq!(hand.new_realized_hand(Order::Trip, Rank::_8 as u64, Rank::_A as u64), hand.to_realized_hand());
    }

    #[test]
    fn building_hand_to_realized_hand_with_two_pair() {

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_Kd, true);
        hand.add_card(&Card::_8c, true);
        hand.add_card(&Card::_9h, true);
        hand.add_card(&Card::_Js, true);
        hand.add_card(&Card::_Qd, true);
        hand.add_card(&Card::_8d, true);
        hand.add_card(&Card::_Qh, true);
        assert_eq!(hand.new_realized_hand(Order::Twop, Rank::_Q as u64, Rank::_8 as u64), hand.to_realized_hand());

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_As, true);
        hand.add_card(&Card::_Ac, true);
        hand.add_card(&Card::_6c, true);
        hand.add_card(&Card::_5c, true);
        hand.add_card(&Card::_6d, true);
        hand.add_card(&Card::_Jh, true);
        hand.add_card(&Card::_Jc, true);
        assert_eq!(hand.new_realized_hand(Order::Twop, Rank::_A as u64, Rank::_J as u64), hand.to_realized_hand());
    }

    #[test]
    fn building_hand_to_realized_hand_with_one_pair() {

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_Kd, true);
        hand.add_card(&Card::_8c, true);
        hand.add_card(&Card::_9h, true);
        hand.add_card(&Card::_Js, true);
        hand.add_card(&Card::_Ad, true);
        hand.add_card(&Card::_8d, true);
        hand.add_card(&Card::_Qh, true);
        assert_eq!(hand.new_realized_hand(Order::Pair, Rank::_8 as u64, Rank::_A as u64), hand.to_realized_hand());

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_9h, true);
        hand.add_card(&Card::_6c, true);
        hand.add_card(&Card::_4s, true);
        hand.add_card(&Card::_Jd, true);
        hand.add_card(&Card::_3d, true);
        hand.add_card(&Card::_2c, true);
        hand.add_card(&Card::_2d, true);
        assert_eq!(hand.new_realized_hand(Order::Pair, Rank::_2 as u64, Rank::_J as u64), hand.to_realized_hand());
    }

    #[test]
    fn building_hand_to_realized_hand_with_nothing() {

        let hand = BuildingHand::new();
        assert_eq!(hand.new_realized_hand(Order::None, 0, 0), hand.to_realized_hand());

        let mut hand = BuildingHand::new();
        hand.add_card(&Card::_Kd, true);
        hand.add_card(&Card::_8c, true);
        hand.add_card(&Card::_9h, true);
        hand.add_card(&Card::_Js, true);
        hand.add_card(&Card::_Ad, true);
        hand.add_card(&Card::_7d, true);
        hand.add_card(&Card::_Qh, true);
        assert_eq!(hand.new_realized_hand(Order::High, Rank::_A as u64, Rank::_K as u64), hand.to_realized_hand());
    }
}

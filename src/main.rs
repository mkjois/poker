use std::collections::HashMap;
use std::io;

use poker;

fn main() {
    println!("\nPoker hand builder!");

    //println!("\nThread city baby!");
    //return;

    let card_repr: HashMap<_, _> = [
        ("2d", poker::Card::_2d), ("2c", poker::Card::_2c), ("2h", poker::Card::_2h), ("2s", poker::Card::_2s),
        ("3d", poker::Card::_3d), ("3c", poker::Card::_3c), ("3h", poker::Card::_3h), ("3s", poker::Card::_3s),
        ("4d", poker::Card::_4d), ("4c", poker::Card::_4c), ("4h", poker::Card::_4h), ("4s", poker::Card::_4s),
        ("5d", poker::Card::_5d), ("5c", poker::Card::_5c), ("5h", poker::Card::_5h), ("5s", poker::Card::_5s),
        ("6d", poker::Card::_6d), ("6c", poker::Card::_6c), ("6h", poker::Card::_6h), ("6s", poker::Card::_6s),
        ("7d", poker::Card::_7d), ("7c", poker::Card::_7c), ("7h", poker::Card::_7h), ("7s", poker::Card::_7s),
        ("8d", poker::Card::_8d), ("8c", poker::Card::_8c), ("8h", poker::Card::_8h), ("8s", poker::Card::_8s),
        ("9d", poker::Card::_9d), ("9c", poker::Card::_9c), ("9h", poker::Card::_9h), ("9s", poker::Card::_9s),
        ("Td", poker::Card::_Td), ("Tc", poker::Card::_Tc), ("Th", poker::Card::_Th), ("Ts", poker::Card::_Ts),
        ("Jd", poker::Card::_Jd), ("Jc", poker::Card::_Jc), ("Jh", poker::Card::_Jh), ("Js", poker::Card::_Js),
        ("Qd", poker::Card::_Qd), ("Qc", poker::Card::_Qc), ("Qh", poker::Card::_Qh), ("Qs", poker::Card::_Qs),
        ("Kd", poker::Card::_Kd), ("Kc", poker::Card::_Kc), ("Kh", poker::Card::_Kh), ("Ks", poker::Card::_Ks),
        ("Ad", poker::Card::_Ad), ("Ac", poker::Card::_Ac), ("Ah", poker::Card::_Ah), ("As", poker::Card::_As),
    ].iter()
        .map(|(s, card)| ((*s).to_owned(), card.clone()))
        .collect();

    let streets = [("preflop", 2), ("flop", 3), ("turn", 1), ("river", 1)];
    let mut building_hand = poker::BuildingHand::new();

    for &(street, ncards) in streets.iter() {
        loop {
            let mut input = String::new();
            println!("\nAdd {} {} card(s):", ncards, street);
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read input");

            let cards = match poker::normalize_input_cards(&input, ncards) {
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
    }

    println!();
}

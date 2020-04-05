#!/usr/bin/env python3

import argparse
import sys


def parse_args(argv):
    '''Parse a list of program arguments. Do not include the program name as the first element.'''

    parser = argparse.ArgumentParser(description='TODO')
    #parser.add_argument('abc', type=str, choices=['asdf'], help='TODO')
    parser.add_argument('--def', type=int, required=False, help='TODO')
    parser.add_argument('-g', '--ghi', type=argparse.FileType(mode='r', encoding='UTF-8'), metavar='G', help='TODO')
    #parser.add_argument('xyz', nargs=argparse.REMAINDER, help='All remaining arguments to be parsed as one positional argument')

    group = parser.add_argument_group(title='const options', description='TODO')
    group.add_argument('-j', '--jkl', nargs='?', const='c', default='d', metavar='J', help='TODO')
    group.add_argument('-m', '--mno', dest='const', action='store_const', const=1, default=0, help='TODO')

    mutex_group = parser.add_mutually_exclusive_group(required=False)
    mutex_group.add_argument('-q', '--quiet', action='store_true', help='TODO')
    mutex_group.add_argument('-v', '--verbose', action='count', default=0, help='TODO')
    return parser.parse_args(argv)


RANKS = ('2', '3', '4', '5', '6', '7', '8', '9', 'T', 'J', 'Q', 'K', 'A')
SUITS = ('d', 'c', 'h', 's')
SUIT_ALIASES = {
    'd': 'd', '♦': 'd', '♢': 'd',
    'c': 'c', '♣': 'c', '♧': 'c',
    'h': 'h', '♥': 'h', '♡': 'h',
    's': 's', '♠': 's', '♤': 's',
}

CARD_STR_TO_REPR = dict()

k = 0
for rank in RANKS:
    for suit in SUITS:
        CARD_STR_TO_REPR[f'{rank}{suit}'] = k
        k += 1


def card_str_repr(card_int):
    suit = card_int & 0b11
    rank = (card_int & 0b111100) >> 2
    return f'{RANKS[rank]}{SUITS[suit]}'

def card_int_repr(card_str):
    rank = card_str[0].upper()
    suit = card_str[1].lower()
    return CARD_STR_TO_REPR[f'{rank}{SUIT_ALIASES[suit]}']


for suit in SUITS:
    for rank in RANKS:
        card_str = f'{rank}{suit}'
        assert card_str == card_str_repr(card_int_repr(card_str))

for i in range(52):
    assert i == card_int_repr(card_str_repr(i))


def card_suit(card_int):
    return card_int & 0b11


def card_rank(card_int):
    return card_int >> 2 & 0b1111


class Hand:

    def __init__(self, *hole_cards):
        self.hole = tuple(sorted(map(card_int_repr, hole_cards), reverse=True))
        self.board = []

        hole_set = set()
        for card in self.hole:
            if card in hole_set:
                raise Exception(f'Duplicate card: {card_str_repr(card)}')
            hole_set.add(card)

        # TODO: SortedList
        self.by_rank = list(self.hole)
        self.by_suit = sorted(self.by_rank, key=card_suit, reverse=True)

    def __str__(self):
        return ''.join(map(card_str_repr, self.hole))

    def add(self, card):
        card_int = card_int_repr(card)
        if card_int in self.by_rank:
            raise Exception(f'Duplicate card: {card_str_repr(card)}')

        self.board.append(card_int)
        self.by_rank.append(card_int)
        self.by_rank.sort(reverse=True)
        self.by_suit = sorted(self.by_rank, key=card_suit, reverse=True)

    def eval(self):
        if len(self.hole) == 0:
            return None

        straight_flush, best_flush = None, None
        for i in range(4, len(self.by_suit)):
            curr, back3, back4 = self.by_suit[i], self.by_suit[i-3], self.by_suit[i-4]
            curr_rank,  curr_suit  = card_rank(curr),  card_suit(curr)
            back3_rank, back3_suit = card_rank(back3), card_suit(back3)
            back4_rank, back4_suit = card_rank(back4), card_suit(back4)

            if curr_suit != back4_suit:
                continue

            tmp_straight_flush = None
            if back4_rank - curr_rank == 4:
                tmp_straight_flush = self.by_suit[i-4:i+1]
            elif curr_rank == 0b0000 and back3_rank == 0b0011 and back4_rank == 0b1100:
                tmp_straight_flush = self.by_suit[i-3:i+1]
                tmp_straight_flush.append(back4)

            if tmp_straight_flush:
                if not straight_flush or card_rank(tmp_straight_flush[0]) > card_rank(straight_flush[0]):
                    straight_flush = tmp_straight_flush
                continue

        if straight_flush:
            return f'SF_{SUITS[card_suit(straight_flush[0])].upper()} {"".join(map(lambda card: card_str_repr(card)[0], straight_flush))}'

        straight = [self.by_rank[0]]
        flushes = [[self.by_suit[0]]]
        quads, trips, pairs = [], [], []
        high = straight[0]

        for i in range(1, len(self.by_rank)):

            curr, prev = self.by_suit[i], self.by_suit[i-1]
            if card_suit(curr) != card_suit(prev):
                if len(flushes[-1]) < 5:
                    flushes[-1].clear()
                else:
                    flushes.append([])
            flushes[-1].append(curr)

            curr, prev = self.by_rank[i], self.by_rank[i-1]
            curr_rank, prev_rank = card_rank(curr), card_rank(prev)

            if prev_rank - curr_rank == 0:
                if trips and card_rank(trips[-1]) == curr_rank:
                    trips.pop()
                    quads.append(curr)
                elif pairs and card_rank(pairs[-1]) == curr_rank:
                    pairs.pop()
                    trips.append(curr)
                else:
                    pairs.append(curr)
            elif prev_rank - curr_rank != 1:
                straight.clear()
            straight.append(curr)

        non_kickers = set(map(card_rank, quads)) | set(map(card_rank, trips)) | set(map(card_rank, pairs))
        kickers = tuple(card for card in self.by_rank if card_rank(card) not in non_kickers)

        if quads:
            return f'QUAD {card_str_repr(quads[0])[0] * 4}{card_str_repr(kickers[0])[0]}'

        if trips:
            filler = None
            if len(trips) > 1:
                if pairs:
                    second_set, first_pair = trips[1], pairs[0]
                    filler = second_set if card_rank(second_set) > card_rank(first_pair) else first_pair
                else:
                    filler = trips[1]
            elif pairs:
                filler = pairs[0]
            if filler:
                return f'BOAT {card_str_repr(trips[0])[0] * 3}{card_str_repr(filler)[0] * 2}'
            else:
                return f'TRIP {card_str_repr(trips[0])[0] * 3}{"".join(map(lambda card: card_str_repr(card)[0], kickers[:2]))}'

        if pairs:
            if len(pairs) > 1:
                return f'TWOP {card_str_repr(pairs[0])[0] * 2}{card_str_repr(pairs[1])[0] * 2}{card_str_repr(kickers[0])[0]}'
            else:
                return f'PAIR {card_str_repr(pairs[0])[0] * 2}{"".join(map(lambda card: card_str_repr(card)[0], kickers[:3]))}'

        return f'HIGH {"".join(map(lambda card: card_str_repr(card)[0], kickers[:5]))}'


if __name__ == '__main__':
    args = parse_args(sys.argv[1:])

    print(Hand('kd', '8c', '9h', 'Js', 'ad', '7d', 'Qh').eval())
    print(Hand('kd', '8c', '9h', 'Js', 'ad', '8d', 'Qh').eval())
    print(Hand('kd', '8c', '9h', 'Js', 'Qd', '8d', 'Qh').eval())
    print(Hand('kd', '8c', '9h', '8s', 'ad', '8d', 'Qh').eval())
    print(Hand('kd', '8c', '9h', '8s', 'Qd', '8d', 'Qh').eval())
    print(Hand('8h', '8c', '9h', '8s', 'ad', '8d', 'Qh').eval())
    print(Hand('8h', 'Th', '9h', '8s', 'ad', 'Jh', 'Qh').eval())
    print(Hand('8h', 'Td', '9h', '8s', 'ad', 'Jh', 'Qh').eval())
    print(Hand('Ac', 'Td', '3c', '4c', 'ad', '2c', '5c').eval())
    print(Hand('Ac', 'Td', '3c', '4s', 'ad', '2c', '5c').eval())

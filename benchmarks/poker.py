#!/usr/bin/env python3

import argparse
import functools
import itertools
import math
import os
import random
import sys
import time


# Note: currently does nothing; just an artifact of how I generate Python script skeleton code
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
RANK_INDICES = dict()

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
    RANK_INDICES[rank] = k >> 2
    for suit in SUITS:
        CARD_STR_TO_REPR[f'{rank}{suit}'] = k
        k += 1


def card_suit(card_int):
    return card_int & 0b11

def card_rank(card_int):
    return card_int >> 2

def card_int_repr(card_str):
    return CARD_STR_TO_REPR[f'{card_str[0].upper()}{SUIT_ALIASES[card_str[1].lower()]}']

def card_str_repr(card_int):
    return f'{RANKS[card_rank(card_int)]}{SUITS[card_suit(card_int)]}'

def rank_str_sort(rank_str):
    return ''.join(sorted(rank_str, key=RANK_INDICES.get, reverse=True))


for suit in SUITS:
    for rank in RANKS:
        card_str = f'{rank}{suit}'
        assert card_str == card_str_repr(card_int_repr(card_str))

for i in range(52):
    assert i == card_int_repr(card_str_repr(i))


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

    def lookup(self, suited_table, offsuit_table, debug=False):
        best_ranks, best_class, best_total_rank = None, -1, 9999

        for hand in itertools.combinations(self.by_rank, 5):
            suits = map(card_suit, hand)
            same_suit = functools.reduce(lambda s1, s2: s2 if s1 == s2 else -1, suits, next(suits))
            ranks_key = ''.join(map(lambda card: card_str_repr(card)[0], hand))
            hand_class, total_rank = (offsuit_table if same_suit == -1 else suited_table)[ranks_key]
            if total_rank < best_total_rank:
                best_ranks, best_class, best_total_rank = ranks_key, hand_class, total_rank

        return best_ranks, best_class

    def eval(self, debug=False):
        if len(self.hole) == 0:
            return None

        straight_flush, best_flush = None, None
        if debug:
            print('card str by suit:', ''.join(map(card_str_repr, self.by_suit)))
            print('card int by suit:', self.by_suit)

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
            elif back3_rank - curr_rank == 3:
                for j in range(i-4, -1, -1):
                    top = self.by_suit[j]
                    if card_suit(top) != curr_suit:
                        break
                    if card_rank(top) - curr_rank == 12:
                        tmp_straight_flush = self.by_suit[i-3:i+1]
                        tmp_straight_flush.append(top)
                        break

            if tmp_straight_flush:
                if not straight_flush or card_rank(tmp_straight_flush[0]) > card_rank(straight_flush[0]):
                    straight_flush = tmp_straight_flush
                continue

            curr_flush = self.by_suit[i-4:i+1]
            if not best_flush or tuple(map(card_rank, curr_flush)) > tuple(map(card_rank, best_flush)):
                best_flush = curr_flush

        if straight_flush:
            return rank_str_sort("".join(map(lambda card: card_str_repr(card)[0], straight_flush))), 8

        straight, best_straight = [self.by_rank[0]], None
        quads, trips, pairs = [], [], []
        high = straight[0]
        if debug:
            print('card str by rank:', ''.join(map(card_str_repr, self.by_rank)))
            print('card int by rank:', self.by_rank)

        for i in range(1, len(self.by_rank)):
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
            elif prev_rank - curr_rank == 1:
                straight.append(curr)
                if len(straight) == 5 and not best_straight:
                    best_straight = straight[:]
            else:
                straight.clear()
                straight.append(curr)

        non_kicks = set(map(card_rank, quads)) | set(map(card_rank, trips)) | set(map(card_rank, pairs))
        kicks = tuple(card for card in self.by_rank if card_rank(card) not in non_kicks)

        if quads:
            candidates = [kicks[0]] if kicks else []
            if len(quads) > 1:
                candidates.append(quads[1])
            if trips:
                candidates.append(trips[0])
            if pairs:
                candidates.append(pairs[0])
            return rank_str_sort(f'{card_str_repr(quads[0])[0] * 4}{card_str_repr(max(candidates))[0]}'), 7

        if trips:
            candidates = []
            if len(trips) > 1:
                candidates.append(trips[1])
            if pairs:
                candidates.append(pairs[0])
            if candidates:
                return rank_str_sort(f'{card_str_repr(trips[0])[0] * 3}{card_str_repr(max(candidates))[0] * 2}'), 6

        if best_flush:
            return rank_str_sort("".join(map(lambda card: card_str_repr(card)[0], best_flush))), 5

        if best_straight:
            return rank_str_sort("".join(map(lambda card: card_str_repr(card)[0], best_straight[:5]))), 4
        elif len(straight) == 4 and card_rank(self.by_rank[0]) - card_rank(straight[-1]) == 12:
            return rank_str_sort(f'{"".join(map(lambda card: card_str_repr(card)[0], straight))}{card_str_repr(self.by_rank[0])[0]}'), 4

        if trips:
            return rank_str_sort(f'{card_str_repr(trips[0])[0] * 3}{"".join(map(lambda card: card_str_repr(card)[0], kicks[:2]))}'), 3

        if pairs:
            if len(pairs) > 1:
                candidates = [kicks[0]] if kicks else []
                if len(pairs) > 2:
                    candidates.append(pairs[2])
                return rank_str_sort(f'{card_str_repr(pairs[0])[0] * 2}{card_str_repr(pairs[1])[0] * 2}{card_str_repr(max(candidates))[0]}'), 2
            else:
                return rank_str_sort(f'{card_str_repr(pairs[0])[0] * 2}{"".join(map(lambda card: card_str_repr(card)[0], kicks[:3]))}'), 1

        return rank_str_sort("".join(map(lambda card: card_str_repr(card)[0], kicks[:5]))), 0


class ClassificationTester:

    def __init__(self, lookup_table_suited, lookup_table_offsuit):
        self.suited_table = lookup_table_suited
        self.offsuit_table = lookup_table_offsuit

    def eq_class(self, hand, debug=False):
        eval_ranks, eval_class = hand.eval(debug)
        lookup_ranks, lookup_class = hand.lookup(self.suited_table, self.offsuit_table, debug)
        if debug:
            print('results:', ''.join(map(card_str_repr, hand.by_rank)), eval_ranks, eval_class, lookup_ranks, lookup_class)
        return eval_ranks == lookup_ranks and eval_class == lookup_class


def n_choose_k(n, k):
    return math.factorial(n) // math.factorial(k) // math.factorial(n-k)


if __name__ == '__main__':
    args = parse_args(sys.argv[1:])

    suited_table  = dict()
    offsuit_table = dict()
    with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'data/5-card-distinct-ranks.csv')) as f:
        for total_rank, line in enumerate(f):
            ranks_key, hand_class = line.split(',')
            if ranks_key in offsuit_table:
                suited_table[ranks_key]  = offsuit_table[ranks_key]
            offsuit_table[ranks_key] = int(hand_class), total_rank

    # Basic test cases
    tester = ClassificationTester(suited_table, offsuit_table)
    assert tester.eq_class(Hand('kd', '8c', '9h', 'Js', 'ad', '7d', 'Qh'))
    assert tester.eq_class(Hand('kd', '8c', '9h', 'Js', 'ad', '8d', 'Qh'))
    assert tester.eq_class(Hand('kd', '8c', '9h', 'Js', 'Qd', '8d', 'Qh'))
    assert tester.eq_class(Hand('kd', '8c', '9h', '8s', 'ad', '8d', 'Qh'))
    assert tester.eq_class(Hand('kd', '8c', '9h', '8s', 'Qd', '8d', 'Qh'))
    assert tester.eq_class(Hand('8h', '8c', '9h', '8s', 'ad', '8d', 'Qh'))
    assert tester.eq_class(Hand('8h', 'Th', '9h', '8s', 'ad', 'Jh', 'Qh'))
    assert tester.eq_class(Hand('8h', 'Td', '9h', '8s', 'ad', 'Jh', 'Qh'))
    assert tester.eq_class(Hand('Ac', 'Td', '3c', '4c', 'ad', '2c', '5c'))
    assert tester.eq_class(Hand('Ac', 'Td', '3c', '4s', 'ad', '2c', '5c'))
    assert tester.eq_class(Hand('6c', 'Td', '3c', '4s', 'ad', '2c', '5c'))
    assert tester.eq_class(Hand('6c', 'Td', '3c', '4s', 'ac', '2c', '5c'))

    # Regression test cases found at random
    assert tester.eq_class(Hand('As', 'Ac', 'Jh', 'Jc', '6c', '6d'))
    assert tester.eq_class(Hand('ac', 'jd', '9h', '4s', '3d', '2c', '2d'))
    assert tester.eq_class(Hand('jh', 'th', '9d', '8d', '7h', '5d', '2d'))
    assert tester.eq_class(Hand('Kh', 'kc', 'qh', 'qd', 'th', 'td', '4c'))
    assert tester.eq_class(Hand('Ad', 'Ks', 'Kd', 'Tc', '6s', '6h', '4h', '3s', '3c'))
    assert tester.eq_class(Hand('Kc', 'Kd', '9s', '8h', '8c', '7h', '7c', '4s', '4d'))
    assert tester.eq_class(Hand('Ah', 'Ks', 'Kh', 'Qc', '8d', '6d', '5h', '4h', '3h', '3d', '2h'))
    assert tester.eq_class(Hand('Kh', 'Kd', 'Th', 'Td', '9s', '6h', '6d', '5d', '4h', '3h', '2d'))
    assert tester.eq_class(Hand('Ad', 'Kh', 'Qc', 'Jd', 'Ts', 'Th', '8s', '7c', '7d', '6c', '5c', '4s'))
    assert tester.eq_class(Hand('Ah', 'Ac', 'Ad', 'Td', '9s', '9h', '6s', '6h', '6c', '6d', '4h', '3s', '2h'))
    assert tester.eq_class(Hand('As', 'Ac', 'Js', 'Th', 'Tc', '9d', '8s', '7h', '7d', '5d', '2s', '2h', '2c', '2d'))

    sampling_config = {n: max(1, n_choose_k(40, 5) // n_choose_k(n, 5)) for n in range(5, 53)}
    sampling_config = sorted(sampling_config.items())
    iter_digits = 1 + int(math.log10(max(sampling_config, key=lambda p: p[1])[1]))

    for ncards, iterations in sampling_config:
        micros_eval, micros_lookup = 0, 0

        for _ in range(iterations):
            hand = random.sample(range(52), ncards)
            hand = tuple(map(card_str_repr, hand))
            hand = Hand(*hand)

            try:
                start = time.time()
                lookup_ranks, lookup_class = hand.lookup(suited_table, offsuit_table)
                end = time.time()
                micros_lookup += 1e6 * (end - start)

                start = time.time()
                eval_ranks, eval_class = hand.eval()
                end = time.time()
                micros_eval += 1e6 * (end - start)

            except Exception:
                print('[ERROR]', ''.join(map(card_str_repr, hand.by_rank)))
                continue

            if eval_ranks != lookup_ranks or eval_class != lookup_class:
                print('[ERROR]', ''.join(map(card_str_repr, hand.by_rank)), eval_ranks, eval_class, lookup_ranks, lookup_class)

        print(f'{ncards : >2} cards lookup(), {iterations: >{iter_digits}} iters,  avg time (us): {micros_lookup / iterations : >12.3f}')
        print(f'{ncards : >2} cards   eval(), {iterations: >{iter_digits}} iters,  avg time (us): {micros_eval   / iterations : >12.3f}')

#!/bin/bash

set -e # stop on error

function player1() {
    http --check --session=test1 "$@"
}
function player1_quiet() {
    player1 --quiet "$@"
}
function player1_expect_error() {
    expectation="$1"
    shift
    if player1 --quiet --quiet "$@"; then
        echo "  missing expected error ($expectation)!"
        false
    else
        echo "  got expected error ($expectation)"
    fi
}

function player2() {
    http --check --session=test2 "$@"
}
function player2_quiet() {
    player2 --quiet "$@"
}
function player2_expect_error() {
    expectation="$1"
    shift
    if player2 --quiet --quiet "$@"; then
        echo "  missing expected error ($expectation)!"
        false
    else
        echo "  got expected error ($expectation)"
    fi
}

echo "log in"
player1_quiet POST :3000/login name="John Doe"
echo "whoami"
player1_quiet :3000/login
echo "log out"
player1_quiet DELETE :3000/login 
player1_expect_error "logged out" :3000/login

echo "log in again"
player1_quiet POST :3000/login name="Jane Doe"

echo "list tables (should be empty)"
test `player1 :3000/table | jq length` -eq 0
echo "create table"
TABLE=`player1 POST :3000/table`
echo "list tables (should be empty)"
test `player1 :3000/table | jq length` -eq 1

#echo "creating multiple tables without others should be forbidden:"
#player1 POST :3000/table
#player1 :3000/table

echo "try to join own table"
player1_expect_error "already at table" POST :3000/table/${TABLE}
test `player1 :3000/table | jq length` -eq 1

echo "log in second player"
player2_quiet POST :3000/login name="Crabby"
test `player2 :3000/table | jq length` -eq 0
echo "create unused table"
player2_quiet POST :3000/table
test `player2 :3000/table | jq length` -eq 1
test `player1 :3000/table | jq length` -eq 1
echo "join table of first player"
player2_quiet POST :3000/table/${TABLE}
test `player2 :3000/table | jq length` -eq 2

echo "verify that game status is still inactive"
test `player1 :3000/table/${TABLE}/game | jq .active` == false
echo "start game"
player1_quiet POST :3000/table/${TABLE}/game
echo "test that game status is now active (for both players at table)"
test `player1 :3000/table/${TABLE}/game | jq .active` == true
test `player2 :3000/table/${TABLE}/game | jq .active` == true
echo "cannot start game twice"
player2_expect_error "starting game twice" POST :3000/table/${TABLE}/game

echo "play first hand with varying card indices"
player2_quiet                                      POST :3000/table/${TABLE}/game/play card_index:=1
player2_expect_error "playing when not one's turn" POST :3000/table/${TABLE}/game/play card_index:=0
player1_quiet                                      POST :3000/table/${TABLE}/game/play card_index:=3
player1_expect_error "playing when not one's turn" POST :3000/table/${TABLE}/game/play card_index:=0
player2_quiet                                      POST :3000/table/${TABLE}/game/play card_index:=2
player1_quiet                                      POST :3000/table/${TABLE}/game/play card_index:=1
player2_expect_error "too high card_index"         POST :3000/table/${TABLE}/game/play card_index:=3
player2_quiet                                      POST :3000/table/${TABLE}/game/play card_index:=1
player1_quiet                                      POST :3000/table/${TABLE}/game/play card_index:=0
player2_quiet                                      POST :3000/table/${TABLE}/game/play card_index:=0
player1_quiet                                      POST :3000/table/${TABLE}/game/play card_index:=0

echo "players have 48 cards in total; 8 have been played already, 2*20 to go..."
for i in `seq 19`; do
    player2_quiet POST :3000/table/${TABLE}/game/play card_index:=0
    player1_quiet POST :3000/table/${TABLE}/game/play card_index:=0
done

player2_quiet POST :3000/table/${TABLE}/game/play card_index:=0
echo "test that game has not yet ended according to status"
test `player1 :3000/table/${TABLE}/game | jq .ended` == false
echo "trying to finish just a little too early"
player1_expect_error "game not finished yet" DELETE :3000/table/${TABLE}/game
player1_quiet POST :3000/table/${TABLE}/game/play card_index:=0

echo "test that game status shows that game has now ended"
test `player1 :3000/table/${TABLE}/game | jq .ended` == true

echo "finishing game"
player1_quiet DELETE :3000/table/${TABLE}/game

test `player1 :3000/table/${TABLE}/game | jq .active` == false

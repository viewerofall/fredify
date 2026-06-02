// 17 — Rock Paper Scissors, written in JavaScript, compiled to native.
//
//   fred examples/17_rock_paper_scissors.js rps && ./rps
//
// Shows the JS frontend reaching real compiler builtins: input_key() for raw
// single-key input and Math.random() for the CPU. Arrays are int-only, so the
// move names come from a helper rather than a string array.

function nameOf(move) {
  if (move === 0) {
    return "Rock";
  } else if (move === 1) {
    return "Paper";
  } else {
    return "Scissors";
  }
}

function playRound(player) {
  const cpu = Math.random() % 3;
  console.log(`You: ${nameOf(player)}   CPU: ${nameOf(cpu)}`);

  // (3 + player - cpu) % 3  ->  0 tie, 1 win, 2 lose
  const result = (3 + player - cpu) % 3;
  if (result === 0) {
    console.log("Tie!");
    return 0;
  } else if (result === 1) {
    console.log("You win!");
    return 1;
  } else {
    console.log("You lose!");
    return 0;
  }
}

let wins = 0;
let rounds = 0;
console.log("Rock/Paper/Scissors — press r, p, s to play, q to quit.");

while (true) {
  const k = input_key();
  let move = -1;
  if (k === 114) {
    move = 0;
  } else if (k === 112) {
    move = 1;
  } else if (k === 115) {
    move = 2;
  }

  if (k === 113) {
    break;
  }

  if (move === -1) {
    console.log("Press r, p, s, or q.");
  } else {
    wins = wins + playRound(move);
    rounds = rounds + 1;
  }
}

console.log(`You won ${wins} of ${rounds} rounds. Bye!`);

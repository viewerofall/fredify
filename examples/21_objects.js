// 21 — JavaScript objects + floats, compiled natively (no Node).
//
//   fred examples/21_objects.js out && ./out
//
// Object literals, member access, member assignment and float math all transpile
// to fred's boxed-container + double runtime.

const account = { owner: "abyss", balance: 100.50, active: true };
console.log("owner", account.owner);
console.log("balance", account.balance);

account.balance += 49.50;
console.log("after deposit", account.balance);

// function params are int64_t today, so pass integers
function makeVec(x, y) {
  return { x: x, y: y };
}

const v = makeVec(3, 4);
const mag = Math.sqrt(v.x * v.x + v.y * v.y);
console.log("vector", v.x, v.y, "magnitude", mag);

// shorthand properties { x, y }
const x = 10;
const y = 20;
const p = { x, y };
console.log("point", p.x, p.y);

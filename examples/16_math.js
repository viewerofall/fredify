// 16 — Complex math in real JavaScript, compiled to a native binary.
//
//   fred examples/16_math.js out && ./out
//
// Modern JS (let/const, arrow fns, for, recursion, template literals, .map /
// .reduce) goes through the hand-written JS->fred transpiler, then to C.
// NOTE: fred numbers are integers, so this sticks to integer math.

// classic recursion
function factorial(n) {
  if (n <= 1) {
    return 1;
  }
  return n * factorial(n - 1);
}

// Euclid's GCD
function gcd(a, b) {
  while (b !== 0) {
    const t = b;
    b = a % b;
    a = t;
  }
  return a;
}

// trial division, no sqrt needed (i*i <= n)
function isPrime(n) {
  if (n < 2) {
    return false;
  }
  for (let i = 2; i * i <= n; i++) {
    if (n % i === 0) {
      return false;
    }
  }
  return true;
}

// count primes up to a limit
function countPrimes(limit) {
  let count = 0;
  for (let i = 2; i <= limit; i++) {
    if (isPrime(i)) {
      count = count + 1;
    }
  }
  return count;
}

console.log(`5! = ${factorial(5)}`);
console.log(`10! = ${factorial(10)}`);
console.log(`gcd(48, 36) = ${gcd(48, 36)}`);
console.log(`2^10 = ${Math.pow(2, 10)}`);
console.log(`sqrt(144) = ${Math.sqrt(144)}`);
console.log(`primes below 100: ${countPrimes(100)}`);

// array higher-order functions
const nums = [3, 1, 4, 1, 5, 9, 2, 6];
const squares = nums.map(x => x * x);
const total = squares.reduce((acc, x) => acc + x, 0);
const biggest = nums.reduce((acc, x) => (x > acc ? x : acc), 0);

console.log(`sum of squares = ${total}`);
console.log(`max element = ${biggest}`);

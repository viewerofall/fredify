-- 18 — Primes, factorials and powers in real Lua, compiled to a native binary.
--
--   fred examples/18_primes.lua out && ./out
--
-- This is genuine Lua syntax (function/end, then, local, ~=, .., ^), parsed by
-- the built-in Lua->fred transpiler (no external interpreter). fred numbers are
-- integers, so this sticks to integer math.

local function isPrime(n)
  if n < 2 then
    return false
  end
  local i = 2
  while i * i <= n do
    if n % i == 0 then
      return false
    end
    i = i + 1
  end
  return true
end

local function factorial(n)
  if n <= 1 then
    return 1
  end
  return n * factorial(n - 1)
end

print("primes up to 30:")
for n = 2, 30 do
  if isPrime(n) then
    print("  " .. n)
  end
end

print("10! = " .. factorial(10))
print("2^16 = " .. 2 ^ 16)

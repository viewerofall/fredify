-- Complex test
local function fibonacci(n)
  if n <= 1 then
    return n
  end
  return fibonacci(n - 1) + fibonacci(n - 2)
end

local result = fibonacci(10)
print(result)

local obj = {x = 5, y = 10}
print(obj.x)
print(obj.y)

local arr = {1, 2, 3, 4, 5}
local sum = 0
for i = 1, 5 do
  sum = sum + arr[i]
end
print(sum)

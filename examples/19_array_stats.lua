-- 19 — Array stats in real Lua: ipairs, the # length operator, table.sort.
--
--   fred examples/19_array_stats.lua out && ./out
--
-- Uses `for _, v in ipairs(t)` value-iteration on purpose: Lua is 1-based and
-- fred/C is 0-based, so iterating values (rather than `t[i]`) keeps results
-- correct. Arrays are int-only (a fred limit).

local nums = {12, 7, 23, 5, 18, 9, 1}

local sum = 0
local max = 0
local count = 0

for _, v in ipairs(nums) do
  sum = sum + v
  if v > max then
    max = v
  end
  count = count + 1
end

print("count   = " .. count)
print("sum     = " .. sum)
print("max     = " .. max)
print("# nums  = " .. #nums)
print("average = " .. sum / count)

table.sort(nums)
print("sorted, joined: " .. table.concat(nums, ", "))

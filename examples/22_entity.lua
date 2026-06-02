-- 22 — Lua key/value tables (dicts) + floats, compiled to a native binary.
--
--   fred examples/22_entity.lua out && ./out
--
-- Real Lua table-as-record syntax (name = value), field access and float math,
-- parsed by the built-in Lua->fred transpiler (no external interpreter).

local entity = {name = "abyss", hp = 100, speed = 3.5, alive = true}
print("name: " .. entity.name)
print("speed: " .. entity.speed)

entity.hp = entity.hp - 30
print("hp after hit: " .. entity.hp)

if entity.hp > 50 then
  print(entity.name .. " is still alive")
end

-- a function returning a table (params are int64_t today, so pass integers)
local function makeVec(x, y)
  return {x = x, y = y}
end

local v = makeVec(6, 8)
local mag = math.sqrt(v.x * v.x + v.y * v.y)
print("magnitude: " .. mag)

-- nested tables
local world = {name = "veil", size = {w = 256, h = 256}}
print("world: " .. world.name .. " " .. world.size.w .. "x" .. world.size.h)

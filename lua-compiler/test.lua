-- Test lua to fred compiler
local msg = "Hello Fred"
print(msg)

function greet(name)
  return "Hi " .. name
end

print(greet("World"))

for i = 1, 5 do
  print(i)
end

local x = 10
if x > 5 then
  print("x is big")
else
  print("x is small")
end

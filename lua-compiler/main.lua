#!/usr/bin/env lua

local Lexer = require("lexer")
local Parser = require("parser")
local Codegen = require("codegen")

local function compile(source)
  local lexer = Lexer.new(source)
  local tokens = Lexer.tokenize(lexer)

  local parser = Parser.new(tokens)
  local ast = Parser.parse(parser)

  local codegen = Codegen.new()
  local fred = Codegen.generate(codegen, ast)

  return fred
end

local function main()
  if #arg < 1 then
    print("Usage: lua main.lua <input.lua> [output.fred]")
    os.exit(1)
  end

  local inputFile = arg[1]
  local outputFile = arg[2] or inputFile:gsub("%.lua$", ".fred")

  local f = io.open(inputFile, "r")
  if not f then
    print("Error: Cannot read file " .. inputFile)
    os.exit(1)
  end
  local source = f:read("*a")
  f:close()

  local fred = compile(source)

  local out = io.open(outputFile, "w")
  if not out then
    print("Error: Cannot write to " .. outputFile)
    os.exit(1)
  end
  out:write(fred)
  out:close()

  print("Compiled: " .. inputFile .. " -> " .. outputFile)
end

main()

-- Tokenizes Lua source code
local Lexer = {}

local keywords = {
  ["and"] = true, ["break"] = true, ["do"] = true, ["else"] = true, ["elseif"] = true,
  ["end"] = true, ["false"] = true, ["for"] = true, ["function"] = true, ["if"] = true,
  ["in"] = true, ["local"] = true, ["nil"] = true, ["not"] = true, ["or"] = true,
  ["repeat"] = true, ["return"] = true, ["then"] = true, ["true"] = true, ["until"] = true,
  ["while"] = true
}

function Lexer.new(source)
  return {
    source = source,
    pos = 1,
    line = 1,
    col = 1,
    tokens = {}
  }
end

local function peek(lex, offset)
  offset = offset or 0
  return lex.source:sub(lex.pos + offset, lex.pos + offset)
end

local function advance(lex, n)
  n = n or 1
  for _ = 1, n do
    if peek(lex) == "\n" then
      lex.line = lex.line + 1
      lex.col = 1
    else
      lex.col = lex.col + 1
    end
    lex.pos = lex.pos + 1
  end
end

local function skipWhitespace(lex)
  while peek(lex):match("%s") do
    advance(lex)
  end
  -- Skip line comments
  if peek(lex) == "-" and peek(lex, 1) == "-" then
    advance(lex, 2)
    while peek(lex) ~= "\n" and peek(lex) ~= "" do
      advance(lex)
    end
    skipWhitespace(lex)
  end
end

local function readString(lex, quote)
  advance(lex) -- skip opening quote
  local str = ""
  while peek(lex) ~= quote and peek(lex) ~= "" do
    if peek(lex) == "\\" then
      advance(lex)
      local c = peek(lex)
      if c == "n" then str = str .. "\n"
      elseif c == "t" then str = str .. "\t"
      elseif c == "r" then str = str .. "\r"
      else str = str .. c end
      advance(lex)
    else
      str = str .. peek(lex)
      advance(lex)
    end
  end
  advance(lex) -- skip closing quote
  return str
end

local function readNumber(lex)
  local num = ""
  while peek(lex):match("[0-9.]") do
    num = num .. peek(lex)
    advance(lex)
  end
  return tonumber(num)
end

local function readIdentifier(lex)
  local id = ""
  while peek(lex):match("[%w_]") do
    id = id .. peek(lex)
    advance(lex)
  end
  return id
end

function Lexer.tokenize(lex)
  while lex.pos <= #lex.source do
    skipWhitespace(lex)
    if lex.pos > #lex.source then break end

    local c = peek(lex)

    if c == '"' or c == "'" then
      local str = readString(lex, c)
      table.insert(lex.tokens, {type = "STRING", value = str})
    elseif c:match("[0-9]") then
      local num = readNumber(lex)
      table.insert(lex.tokens, {type = "NUMBER", value = num})
    elseif c:match("[%w_]") then
      local id = readIdentifier(lex)
      if keywords[id] then
        table.insert(lex.tokens, {type = id:upper(), value = id})
      else
        table.insert(lex.tokens, {type = "ID", value = id})
      end
    elseif c == "=" and peek(lex, 1) == "=" then
      table.insert(lex.tokens, {type = "EQ", value = "=="})
      advance(lex, 2)
    elseif c == "~" and peek(lex, 1) == "=" then
      table.insert(lex.tokens, {type = "NE", value = "~="})
      advance(lex, 2)
    elseif c == "<" and peek(lex, 1) == "=" then
      table.insert(lex.tokens, {type = "LE", value = "<="})
      advance(lex, 2)
    elseif c == ">" and peek(lex, 1) == "=" then
      table.insert(lex.tokens, {type = "GE", value = ">="})
      advance(lex, 2)
    elseif c == "." and peek(lex, 1) == "." then
      table.insert(lex.tokens, {type = "CONCAT", value = ".."})
      advance(lex, 2)
    elseif c == ":" and peek(lex, 1) == ":" then
      advance(lex, 2)
      local label = ""
      while peek(lex) ~= ":" or peek(lex, 1) ~= ":" do
        label = label .. peek(lex)
        advance(lex)
      end
      advance(lex, 2)
      table.insert(lex.tokens, {type = "LABEL", value = label})
    else
      local ops = {
        ["("] = "LPAREN", [")"] = "RPAREN",
        ["{"] = "LBRACE", ["}"] = "RBRACE",
        ["["] = "LBRACKET", ["]"] = "RBRACKET",
        [","] = "COMMA", [";"] = "SEMI",
        [":"] = "COLON", ["."] = "DOT",
        ["="] = "ASSIGN", ["+"] = "PLUS", ["-"] = "MINUS",
        ["*"] = "MUL", ["/"] = "DIV", ["%"] = "MOD",
        ["#"] = "LEN", ["<"] = "LT", [">"] = "GT"
      }
      if ops[c] then
        table.insert(lex.tokens, {type = ops[c], value = c})
        advance(lex)
      else
        advance(lex)
      end
    end
  end

  table.insert(lex.tokens, {type = "EOF", value = nil})
  return lex.tokens
end

return Lexer

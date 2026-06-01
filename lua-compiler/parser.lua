-- Builds AST from Lua tokens
local Parser = {}

function Parser.new(tokens)
  return {
    tokens = tokens,
    pos = 1
  }
end

local function peek(p, offset)
  offset = offset or 0
  return p.tokens[p.pos + offset] or {type = "EOF"}
end

local function advance(p)
  p.pos = p.pos + 1
end

local function expect(p, typ)
  local tok = peek(p)
  if tok.type ~= typ then
    error("Expected " .. typ .. " but got " .. tok.type)
  end
  advance(p)
  return tok
end

local function match(p, ...)
  local types = {...}
  for _, typ in ipairs(types) do
    if peek(p).type == typ then
      return true
    end
  end
  return false
end

local function parseExpression(p)
  return parseOr(p)
end

function parseOr(p)
  local left = parseAnd(p)
  while peek(p).type == "OR" do
    advance(p)
    local right = parseAnd(p)
    left = {type = "BinOp", op = "or", left = left, right = right}
  end
  return left
end

function parseAnd(p)
  local left = parseComparison(p)
  while peek(p).type == "AND" do
    advance(p)
    local right = parseComparison(p)
    left = {type = "BinOp", op = "and", left = left, right = right}
  end
  return left
end

function parseComparison(p)
  local left = parseConcat(p)
  while match(p, "EQ", "NE", "LT", "LE", "GT", "GE") do
    local op = peek(p).value
    advance(p)
    local right = parseConcat(p)
    left = {type = "BinOp", op = op, left = left, right = right}
  end
  return left
end

function parseConcat(p)
  local left = parseAddSub(p)
  while peek(p).type == "CONCAT" do
    advance(p)
    local right = parseAddSub(p)
    left = {type = "BinOp", op = "..", left = left, right = right}
  end
  return left
end

function parseAddSub(p)
  local left = parseMulDiv(p)
  while match(p, "PLUS", "MINUS") do
    local op = peek(p).value
    advance(p)
    local right = parseMulDiv(p)
    left = {type = "BinOp", op = op, left = left, right = right}
  end
  return left
end

function parseMulDiv(p)
  local left = parseUnary(p)
  while match(p, "MUL", "DIV", "MOD") do
    local op = peek(p).value
    advance(p)
    local right = parseUnary(p)
    left = {type = "BinOp", op = op, left = left, right = right}
  end
  return left
end

function parseUnary(p)
  if match(p, "NOT", "MINUS", "LEN") then
    local op = peek(p).value
    advance(p)
    local expr = parseUnary(p)
    return {type = "UnOp", op = op, expr = expr}
  end
  return parseCall(p)
end

function parseCall(p)
  local expr = parsePrimary(p)
  while true do
    if peek(p).type == "LPAREN" then
      advance(p)
      local args = {}
      if peek(p).type ~= "RPAREN" then
        table.insert(args, parseExpression(p))
        while peek(p).type == "COMMA" do
          advance(p)
          if peek(p).type ~= "RPAREN" then
            table.insert(args, parseExpression(p))
          end
        end
      end
      expect(p, "RPAREN")
      expr = {type = "Call", func = expr, args = args}
    elseif peek(p).type == "DOT" then
      advance(p)
      local field = expect(p, "ID").value
      expr = {type = "Index", obj = expr, field = field}
    elseif peek(p).type == "LBRACKET" then
      advance(p)
      local idx = parseExpression(p)
      expect(p, "RBRACKET")
      expr = {type = "Index", obj = expr, index = idx}
    elseif peek(p).type == "COLON" then
      advance(p)
      local method = expect(p, "ID").value
      advance(p) -- LPAREN
      local args = {}
      if peek(p).type ~= "RPAREN" then
        table.insert(args, parseExpression(p))
        while peek(p).type == "COMMA" do
          advance(p)
          table.insert(args, parseExpression(p))
        end
      end
      expect(p, "RPAREN")
      expr = {type = "MethodCall", obj = expr, method = method, args = args}
    else
      break
    end
  end
  return expr
end

function parsePrimary(p)
  local tok = peek(p)

  if tok.type == "NUMBER" then
    advance(p)
    return {type = "Number", value = tok.value}
  elseif tok.type == "STRING" then
    advance(p)
    return {type = "String", value = tok.value}
  elseif tok.type == "TRUE" then
    advance(p)
    return {type = "Bool", value = true}
  elseif tok.type == "FALSE" then
    advance(p)
    return {type = "Bool", value = false}
  elseif tok.type == "NIL" then
    advance(p)
    return {type = "Nil"}
  elseif tok.type == "ID" then
    advance(p)
    return {type = "Id", name = tok.value}
  elseif tok.type == "LPAREN" then
    advance(p)
    -- Check if this is a parenthesized function (CASTL pattern)
    if peek(p).type == "FUNCTION" then
      local func = parseFunction(p)
      expect(p, "RPAREN")
      return func
    else
      local expr = parseExpression(p)
      expect(p, "RPAREN")
      return expr
    end
  elseif tok.type == "LBRACE" then
    return parseTable(p)
  elseif tok.type == "FUNCTION" then
    return parseFunction(p)
  elseif tok.type == "DO" then
    advance(p)
    local stmts = {}
    while peek(p).type ~= "EOF" and peek(p).type ~= "END" do
      table.insert(stmts, parseStatement(p))
    end
    expect(p, "END")
    if #stmts == 1 and stmts[1].type == "Return" then
      return stmts[1].values[1] or {type = "Nil"}
    end
    return {type = "Block", stmts = stmts}
  else
    error("Unexpected token: " .. tok.type)
  end
end

function parseTable(p)
  expect(p, "LBRACE")
  local fields = {}
  while peek(p).type ~= "RBRACE" and peek(p).type ~= "EOF" do
    if peek(p).type == "ID" and peek(p, 1).type == "ASSIGN" then
      local key = expect(p, "ID").value
      expect(p, "ASSIGN")
      local val = parseExpression(p)
      table.insert(fields, {type = "field", key = key, value = val})
    elseif peek(p).type == "LBRACKET" then
      advance(p)
      local key = parseExpression(p)
      expect(p, "RBRACKET")
      expect(p, "ASSIGN")
      local val = parseExpression(p)
      table.insert(fields, {type = "field", key = key, value = val})
    else
      local val = parseExpression(p)
      table.insert(fields, {type = "field", value = val})
    end
    if peek(p).type == "COMMA" then
      advance(p)
    end
  end
  expect(p, "RBRACE")
  return {type = "Table", fields = fields}
end

function parseFunction(p)
  expect(p, "FUNCTION")
  local params = {}
  expect(p, "LPAREN")
  if peek(p).type ~= "RPAREN" then
    local firstParam = expect(p, "ID").value
    -- Skip CASTL's "this" parameter
    if firstParam ~= "this" then
      table.insert(params, firstParam)
    end
    while peek(p).type == "COMMA" do
      advance(p)
      if peek(p).type ~= "RPAREN" then
        local param = expect(p, "ID").value
        if param ~= "this" then
          table.insert(params, param)
        end
      end
    end
  end
  expect(p, "RPAREN")
  local body = parseBlock(p)
  expect(p, "END")
  return {type = "Function", params = params, body = body}
end

function parseBlock(p)
  local stmts = {}
  while peek(p).type ~= "EOF" and peek(p).type ~= "END" and
        peek(p).type ~= "ELSE" and peek(p).type ~= "ELSEIF" and
        peek(p).type ~= "UNTIL" do
    table.insert(stmts, parseStatement(p))
    if peek(p).type == "SEMI" then
      advance(p)
    end
  end
  return stmts
end

function parseStatement(p)
  local tok = peek(p)

  if tok.type == "LABEL" then
    advance(p)
    return {type = "Label", name = tok.value}

  elseif tok.type == "FUNCTION" then
    advance(p)
    local name = expect(p, "ID").value
    expect(p, "LPAREN")
    local params = {}
    if peek(p).type ~= "RPAREN" then
      table.insert(params, expect(p, "ID").value)
      while peek(p).type == "COMMA" do
        advance(p)
        table.insert(params, expect(p, "ID").value)
      end
    end
    expect(p, "RPAREN")
    local body = parseBlock(p)
    expect(p, "END")
    return {type = "FuncDef", name = name, params = params, body = body}

  elseif tok.type == "LOCAL" then
    advance(p)
    if peek(p).type == "FUNCTION" then
      advance(p)
      local name = expect(p, "ID").value
      expect(p, "LPAREN")
      local params = {}
      if peek(p).type ~= "RPAREN" then
        table.insert(params, expect(p, "ID").value)
        while peek(p).type == "COMMA" do
          advance(p)
          table.insert(params, expect(p, "ID").value)
        end
      end
      expect(p, "RPAREN")
      local body = parseBlock(p)
      expect(p, "END")
      return {type = "LocalFuncDef", name = name, params = params, body = body}
    else
      local name = expect(p, "ID").value
      -- Skip multi-var locals (CASTL's local a, b, c;)
      while peek(p).type == "COMMA" do
        advance(p)
        expect(p, "ID")
      end
      local expr = nil
      if peek(p).type == "ASSIGN" then
        advance(p)
        expr = parseExpression(p)
      end
      return {type = "Local", name = name, value = expr}
    end

  elseif tok.type == "IF" then
    advance(p)
    local cond = parseExpression(p)
    expect(p, "THEN")
    local body = parseBlock(p)
    local elifs = {}
    while peek(p).type == "ELSEIF" do
      advance(p)
      local econd = parseExpression(p)
      expect(p, "THEN")
      local ebody = parseBlock(p)
      table.insert(elifs, {cond = econd, body = ebody})
    end
    local els = nil
    if peek(p).type == "ELSE" then
      advance(p)
      els = parseBlock(p)
    end
    expect(p, "END")
    return {type = "If", cond = cond, body = body, elifs = elifs, els = els}

  elseif tok.type == "WHILE" then
    advance(p)
    local cond = parseExpression(p)
    expect(p, "DO")
    local body = parseBlock(p)
    expect(p, "END")
    return {type = "While", cond = cond, body = body}

  elseif tok.type == "FOR" then
    advance(p)
    local name = expect(p, "ID").value
    expect(p, "ASSIGN")
    local start = parseExpression(p)
    expect(p, "COMMA")
    local finish = parseExpression(p)
    local step = nil
    if peek(p).type == "COMMA" then
      advance(p)
      step = parseExpression(p)
    end
    expect(p, "DO")
    local body = parseBlock(p)
    expect(p, "END")
    return {type = "For", name = name, start = start, finish = finish, step = step, body = body}

  elseif tok.type == "RETURN" then
    advance(p)
    local values = {}
    if peek(p).type ~= "EOF" and peek(p).type ~= "END" then
      table.insert(values, parseExpression(p))
      while peek(p).type == "COMMA" do
        advance(p)
        table.insert(values, parseExpression(p))
      end
    end
    return {type = "Return", values = values}

  else
    local expr = parseExpression(p)
    if peek(p).type == "ASSIGN" then
      advance(p)
      local val = parseExpression(p)
      -- If assigning an anonymous function to a variable, treat as function def
      if val.type == "Function" and expr.type == "Id" then
        return {type = "FuncDef", name = expr.name, params = val.params, body = val.body}
      end
      return {type = "Assign", target = expr, value = val}
    else
      return {type = "Expr", expr = expr}
    end
  end
end

function Parser.parse(p)
  return parseBlock(p)
end

return Parser

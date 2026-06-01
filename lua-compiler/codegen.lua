-- Converts AST to .fred format
local Codegen = {}

function Codegen.new()
  return {
    output = {},
    indent = 0,
    locals = {}
  }
end

local function emit(cg, line)
  table.insert(cg.output, string.rep("  ", cg.indent) .. line)
end

local function genExpr(cg, expr)
  if expr.type == "Number" then
    return tostring(expr.value)
  elseif expr.type == "String" then
    return '"' .. expr.value:gsub('"', '\\"') .. '"'
  elseif expr.type == "Bool" then
    return expr.value and "true" or "false"
  elseif expr.type == "Nil" then
    return "nil"
  elseif expr.type == "Id" then
    return expr.name
  elseif expr.type == "BinOp" then
    local left = genExpr(cg, expr.left)
    local right = genExpr(cg, expr.right)
    local op = expr.op
    if op == ".." then op = "+" end
    return left .. " " .. op .. " " .. right
  elseif expr.type == "UnOp" then
    local e = genExpr(cg, expr.expr)
    if expr.op == "not" then return "!" .. e
    elseif expr.op == "-" then return "-" .. e
    elseif expr.op == "#" then return "len(" .. e .. ")" end
  elseif expr.type == "Call" then
    local func = genExpr(cg, expr.func)
    local args = {}
    local funcName = expr.func.name or ""

    -- CASTL helper function mapping
    local helpers = {
      _add = "+", _addStr1 = "+", _addStr2 = "+", _addNum2 = "+",
      _sub = "-", _mul = "*", _div = "/", _mod = "%",
      _lt = "<", _le = "<=", _gt = ">", _ge = ">=", _eq = "==", _ne = "~=",
      _and = "and", _or = "or", _not = "not",
      _inc = "++", _dec = "--"
    }

    if helpers[funcName] then
      -- Map binary helper to operator
      for _, arg in ipairs(expr.args) do
        if arg.type ~= "Id" or arg.name ~= "_ENV" then
          table.insert(args, genExpr(cg, arg))
        end
      end
      if #args == 1 then
        return args[1]
      elseif #args == 2 then
        return args[1] .. " " .. helpers[funcName] .. " " .. args[2]
      else
        return func .. "(" .. table.concat(args, ", ") .. ")"
      end
    else
      -- Regular function call - skip _ENV argument if present
      for _, arg in ipairs(expr.args) do
        if arg.type ~= "Id" or arg.name ~= "_ENV" then
          table.insert(args, genExpr(cg, arg))
        end
      end
      return func .. "(" .. table.concat(args, ", ") .. ")"
    end
  elseif expr.type == "Index" then
    local obj = genExpr(cg, expr.obj)
    if expr.field then
      return obj .. "." .. expr.field
    else
      local idx = genExpr(cg, expr.index)
      return obj .. "[" .. idx .. "]"
    end
  elseif expr.type == "Table" then
    local fields = {}
    for _, field in ipairs(expr.fields) do
      if field.key then
        local key = type(field.key) == "string" and field.key or genExpr(cg, field.key)
        table.insert(fields, key .. ": " .. genExpr(cg, field.value))
      else
        table.insert(fields, genExpr(cg, field.value))
      end
    end
    return "{ " .. table.concat(fields, ", ") .. " }"
  elseif expr.type == "Function" then
    local params = table.concat(expr.params, ", ")
    emit(cg, "fn(" .. params .. ") {")
    cg.indent = cg.indent + 1
    for _, stmt in ipairs(expr.body) do
      genStmt(cg, stmt)
    end
    cg.indent = cg.indent - 1
    return "function"
  end
  return ""
end

function genStmt(cg, stmt)
  if stmt.type == "Label" then
    -- Skip labels

  elseif stmt.type == "FuncDef" or stmt.type == "LocalFuncDef" then
    local params = table.concat(stmt.params, ", ")
    emit(cg, "fn " .. stmt.name .. "(" .. params .. ") {")
    cg.indent = cg.indent + 1
    for _, s in ipairs(stmt.body) do
      genStmt(cg, s)
    end
    cg.indent = cg.indent - 1
    emit(cg, "}")

  elseif stmt.type == "Local" then
    local val = stmt.value and (" = " .. genExpr(cg, stmt.value)) or ""
    emit(cg, "let " .. stmt.name .. val)

  elseif stmt.type == "Assign" then
    local target = genExpr(cg, stmt.target)
    local val = genExpr(cg, stmt.value)
    emit(cg, target .. " = " .. val)

  elseif stmt.type == "Expr" then
    emit(cg, genExpr(cg, stmt.expr))

  elseif stmt.type == "If" then
    emit(cg, "if (" .. genExpr(cg, stmt.cond) .. ") {")
    cg.indent = cg.indent + 1
    for _, s in ipairs(stmt.body) do
      genStmt(cg, s)
    end
    cg.indent = cg.indent - 1

    for _, elif in ipairs(stmt.elifs or {}) do
      emit(cg, "} else if (" .. genExpr(cg, elif.cond) .. ") {")
      cg.indent = cg.indent + 1
      for _, s in ipairs(elif.body) do
        genStmt(cg, s)
      end
      cg.indent = cg.indent - 1
    end

    if stmt.els then
      emit(cg, "} else {")
      cg.indent = cg.indent + 1
      for _, s in ipairs(stmt.els) do
        genStmt(cg, s)
      end
      cg.indent = cg.indent - 1
    end

    emit(cg, "}")

  elseif stmt.type == "While" then
    emit(cg, "while (" .. genExpr(cg, stmt.cond) .. ") {")
    cg.indent = cg.indent + 1
    for _, s in ipairs(stmt.body) do
      genStmt(cg, s)
    end
    cg.indent = cg.indent - 1
    emit(cg, "}")

  elseif stmt.type == "For" then
    local step = stmt.step and (", " .. genExpr(cg, stmt.step)) or ""
    emit(cg, "loop " .. stmt.name .. " from " .. genExpr(cg, stmt.start) .. " to " .. genExpr(cg, stmt.finish) .. step .. " {")
    cg.indent = cg.indent + 1
    for _, s in ipairs(stmt.body) do
      genStmt(cg, s)
    end
    cg.indent = cg.indent - 1
    emit(cg, "}")

  elseif stmt.type == "Return" then
    if #stmt.values == 0 then
      emit(cg, "return")
    else
      local vals = {}
      for _, val in ipairs(stmt.values) do
        table.insert(vals, genExpr(cg, val))
      end
      emit(cg, "return " .. table.concat(vals, ", "))
    end
  end
end

function Codegen.generate(cg, ast)
  for _, stmt in ipairs(ast) do
    genStmt(cg, stmt)
  end
  return table.concat(cg.output, "\n")
end

return Codegen

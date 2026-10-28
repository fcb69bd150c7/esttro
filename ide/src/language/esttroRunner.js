class EsttroLexer {
  constructor(source) {
    this.source = source;
    this.cursor = 0;
    this.line = 1;
    this.col = 1;
    this.diagnostics = [];
  }

  tokenize() {
    const tokens = [];
    while (this.cursor < this.source.length) {
      const startLine = this.line;
      const startCol = this.col;
      const ch = this.peek();

      if (ch === ' ' || ch === '\t' || ch === '\r') {
        this.advance();
        continue;
      }

      if (ch === '\n') {
        this.advance();
        this.line++;
        this.col = 1;
        continue;
      }

      if (ch === '/' && this.peekNext() === '/') {
        while (this.cursor < this.source.length && this.peek() !== '\n') {
          this.advance();
        }
        continue;
      }
      if (ch === '-' && this.peekNext() === '-') {
        while (this.cursor < this.source.length && this.peek() !== '\n') {
          this.advance();
        }
        continue;
      }
      if (ch === '/' && this.peekNext() === '*') {
        this.advance();
        this.advance();
        while (this.cursor < this.source.length && !(this.peek() === '*' && this.peekNext() === '/')) {
          if (this.peek() === '\n') {
            this.line++;
            this.col = 0;
          }
          this.advance();
        }
        if (this.cursor < this.source.length) {
          this.advance();
          this.advance();
        }
        continue;
      }

      if (ch === '"') {
        tokens.push(this.lexString(startLine, startCol));
        continue;
      }

      if (this.isDigit(ch)) {
        tokens.push(this.lexNumber(startLine, startCol));
        continue;
      }

      if (this.isAlpha(ch) || ch === '_') {
        tokens.push(this.lexIdentifier(startLine, startCol));
        continue;
      }

      if (ch === '-' && this.peekNext() === '>') {
        this.advance(); this.advance();
        tokens.push({ type: 'ARROW', value: '->', line: startLine, col: startCol });
        continue;
      }

      const twoCharOps = {
        '+=': 'PLUS_EQ', '-=': 'MINUS_EQ', '*=': 'STAR_EQ', '/=': 'SLASH_EQ', '%=': 'PERCENT_EQ',
        '==': 'EQ_EQ', '!=': 'NOT_EQ', '<=': 'LT_EQ', '>=': 'GT_EQ',
        '&&': 'AND_AND', '||': 'OR_OR'
      };
      const two = ch + (this.peekNext() || '');
      if (twoCharOps[two]) {
        this.advance(); this.advance();
        tokens.push({ type: twoCharOps[two], value: two, line: startLine, col: startCol });
        continue;
      }

      const singleOps = {
        '+': 'PLUS', '-': 'MINUS', '*': 'STAR', '/': 'SLASH', '%': 'PERCENT',
        '=': 'EQ', '<': 'LT', '>': 'GT', '!': 'BANG',
        '(': 'LPAREN', ')': 'RPAREN', '{': 'LBRACE', '}': 'RBRACE',
        '[': 'LBRACKET', ']': 'RBRACKET', ',': 'COMMA', ':': 'COLON', ';': 'SEMICOLON'
      };
      if (singleOps[ch]) {
        this.advance();
        tokens.push({ type: singleOps[ch], value: ch, line: startLine, col: startCol });
        continue;
      }

      this.diagnostics.push({
        line: startLine,
        col: startCol,
        message: `Unexpected character '${ch}'`,
        severity: 'error',
      });
      this.advance();
    }

    tokens.push({ type: 'EOF', value: '', line: this.line, col: this.col });
    return tokens;
  }

  lexString(startLine, startCol) {
    this.advance(); // "
    let raw = '';
    while (this.cursor < this.source.length && this.peek() !== '"') {
      if (this.peek() === '\\') {
        this.advance();
        const esc = this.advance();
        if (esc === 'n') raw += '\n';
        else if (esc === 't') raw += '\t';
        else if (esc === 'r') raw += '\r';
        else if (esc === '\\') raw += '\\';
        else if (esc === '"') raw += '"';
        else raw += esc;
      } else {
        if (this.peek() === '\n') {
          this.line++;
          this.col = 0;
        }
        raw += this.advance();
      }
    }
    if (this.peek() === '"') {
      this.advance();
    } else {
      this.diagnostics.push({
        line: startLine,
        col: startCol,
        message: 'Unterminated string literal',
        severity: 'error',
      });
    }
    return { type: 'STRING', value: raw, line: startLine, col: startCol };
  }

  lexNumber(startLine, startCol) {
    let numStr = '';
    while (this.cursor < this.source.length && this.isDigit(this.peek())) {
      numStr += this.advance();
    }
    if (this.peek() === '.' && this.isDigit(this.peekNext())) {
      numStr += this.advance();
      while (this.cursor < this.source.length && this.isDigit(this.peek())) {
        numStr += this.advance();
      }
      return { type: 'DOUBLE_LIT', value: parseFloat(numStr), line: startLine, col: startCol };
    }
    return { type: 'INT_LIT', value: parseInt(numStr, 10), line: startLine, col: startCol };
  }

  lexIdentifier(startLine, startCol) {
    let ident = '';
    while (this.cursor < this.source.length && (this.isAlphaNum(this.peek()) || this.peek() === '_')) {
      ident += this.advance();
    }
    const keywords = [
      'function', 'let', 'const', 'if', 'else', 'while', 'for',
      'return', 'break', 'continue', 'true', 'false',
      'int', 'float', 'double', 'boolean', 'string', 'void'
    ];
    if (keywords.includes(ident)) {
      return { type: ident.toUpperCase(), value: ident, line: startLine, col: startCol };
    }
    return { type: 'IDENT', value: ident, line: startLine, col: startCol };
  }

  peek() { return this.source[this.cursor]; }
  peekNext() { return this.cursor + 1 < this.source.length ? this.source[this.cursor + 1] : null; }
  advance() {
    const ch = this.source[this.cursor++];
    this.col++;
    return ch;
  }
  isDigit(ch) { return ch >= '0' && ch <= '9'; }
  isAlpha(ch) { return (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z'); }
  isAlphaNum(ch) { return this.isAlpha(ch) || this.isDigit(ch); }
}

class EsttroParser {
  constructor(tokens, diagnostics) {
    this.tokens = tokens;
    this.cursor = 0;
    this.diagnostics = diagnostics;
  }

  peek() { return this.tokens[this.cursor] || { type: 'EOF', line: 1, col: 1 }; }
  previous() { return this.tokens[this.cursor - 1]; }
  isAtEnd() { return this.peek().type === 'EOF'; }

  advance() {
    if (!this.isAtEnd()) this.cursor++;
    return this.previous();
  }

  check(type) {
    if (this.isAtEnd()) return false;
    return this.peek().type === type;
  }

  match(...types) {
    for (const t of types) {
      if (this.check(t)) {
        this.advance();
        return true;
      }
    }
    return false;
  }

  consume(type, message) {
    if (this.check(type)) return this.advance();
    const tok = this.peek();
    this.diagnostics.push({
      line: tok.line,
      col: tok.col,
      message,
      severity: 'error',
    });
    return null;
  }

  parse() {
    const statements = [];
    while (!this.isAtEnd()) {
      try {
        const stmt = this.declaration();
        if (stmt) statements.push(stmt);
      } catch (err) {
        this.synchronize();
      }
    }
    return statements;
  }

  synchronize() {
    this.advance();
    while (!this.isAtEnd()) {
      if (this.previous().type === 'SEMICOLON') return;
      switch (this.peek().type) {
        case 'FUNCTION':
        case 'LET':
        case 'CONST':
        case 'FOR':
        case 'IF':
        case 'WHILE':
        case 'RETURN':
          return;
      }
      this.advance();
    }
  }

  declaration() {
    if (this.match('FUNCTION')) return this.functionDeclaration();
    if (this.match('LET')) return this.letDeclaration();
    if (this.match('CONST')) return this.constDeclaration();
    return this.statement();
  }

  functionDeclaration() {
    const nameTok = this.consume('IDENT', "Expected function name after 'function'");
    this.consume('LPAREN', "Expected '(' after function name");
    const params = [];
    if (!this.check('RPAREN')) {
      do {
        const pName = this.consume('IDENT', "Expected parameter name");
        this.consume('COLON', "Expected ':' after parameter name");
        const pType = this.parseType();
        params.push({ name: pName ? pName.value : '?', type: pType });
      } while (this.match('COMMA'));
    }
    this.consume('RPAREN', "Expected ')' after parameters");

    let retType = 'void';
    if (this.match('ARROW')) {
      retType = this.parseType();
    }

    this.consume('LBRACE', "Expected '{' before function body");
    const body = this.block();
    return {
      type: 'FunctionDecl',
      name: nameTok ? nameTok.value : '?',
      params,
      retType,
      body,
      line: nameTok ? nameTok.line : 1,
    };
  }

  parseType() {
    const t = this.advance();
    return t.value;
  }

  letDeclaration() {
    const nameTok = this.consume('IDENT', "Expected variable name after 'let'");
    let typeAnnot = null;
    if (this.match('COLON')) {
      typeAnnot = this.parseType();
    }
    this.consume('EQ', "Expected '=' after variable name");
    const init = this.expression();
    this.match('SEMICOLON');
    return {
      type: 'LetDecl',
      name: nameTok ? nameTok.value : '?',
      typeAnnot,
      init,
      line: nameTok ? nameTok.line : 1,
    };
  }

  constDeclaration() {
    const nameTok = this.consume('IDENT', "Expected constant name after 'const'");
    this.consume('EQ', "Expected '=' after constant name");
    const init = this.expression();
    this.match('SEMICOLON');
    return {
      type: 'ConstDecl',
      name: nameTok ? nameTok.value : '?',
      init,
      line: nameTok ? nameTok.line : 1,
    };
  }

  statement() {
    if (this.match('IF')) return this.ifStatement();
    if (this.match('WHILE')) return this.whileStatement();
    if (this.match('FOR')) return this.forStatement();
    if (this.match('RETURN')) return this.returnStatement();
    if (this.match('BREAK')) {
      const line = this.previous().line;
      this.match('SEMICOLON');
      return { type: 'Break', line };
    }
    if (this.match('CONTINUE')) {
      const line = this.previous().line;
      this.match('SEMICOLON');
      return { type: 'Continue', line };
    }
    if (this.match('LBRACE')) return { type: 'Block', statements: this.block() };

    return this.expressionStatement();
  }

  ifStatement() {
    const condition = this.expression();
    this.consume('LBRACE', "Expected '{' after if condition");
    const thenBranch = this.block();
    let elseBranch = null;
    if (this.match('ELSE')) {
      if (this.match('IF')) {
        elseBranch = [this.ifStatement()];
      } else {
        this.consume('LBRACE', "Expected '{' after else");
        elseBranch = this.block();
      }
    }
    return { type: 'If', condition, thenBranch, elseBranch };
  }

  whileStatement() {
    const condition = this.expression();
    this.consume('LBRACE', "Expected '{' after while condition");
    const body = this.block();
    return { type: 'While', condition, body };
  }

  forStatement() {
    const varName = this.consume('IDENT', "Expected loop variable name");
    this.consume('EQ', "Expected '=' in for loop");
    const start = this.expression();
    this.consume('COMMA', "Expected ',' between range expressions");
    const end = this.expression();
    this.consume('LBRACE', "Expected '{' after for loop range");
    const body = this.block();
    return {
      type: 'For',
      variable: varName ? varName.value : '?',
      start,
      end,
      body,
    };
  }

  returnStatement() {
    const line = this.previous().line;
    let value = null;
    if (!this.check('SEMICOLON') && !this.check('RBRACE')) {
      value = this.expression();
    }
    this.match('SEMICOLON');
    return { type: 'Return', value, line };
  }

  block() {
    const stmts = [];
    while (!this.check('RBRACE') && !this.isAtEnd()) {
      const s = this.declaration();
      if (s) stmts.push(s);
    }
    this.consume('RBRACE', "Expected '}' after block");
    return stmts;
  }

  expressionStatement() {
    const expr = this.expression();
    this.match('SEMICOLON');
    return { type: 'ExprStmt', expr };
  }

  expression() {
    return this.assignment();
  }

  assignment() {
    const expr = this.logicalOr();
    if (this.match('EQ', 'PLUS_EQ', 'MINUS_EQ', 'STAR_EQ', 'SLASH_EQ', 'PERCENT_EQ')) {
      const op = this.previous();
      const value = this.assignment();
      if (expr.type === 'Variable') {
        return {
          type: 'Assignment',
          name: expr.name,
          op: op.value,
          value,
          line: op.line,
        };
      }
      this.diagnostics.push({
        line: op.line,
        col: op.col,
        message: 'Invalid assignment target',
        severity: 'error',
      });
    }
    return expr;
  }

  logicalOr() {
    let expr = this.logicalAnd();
    while (this.match('OR_OR')) {
      const op = this.previous().value;
      const right = this.logicalAnd();
      expr = { type: 'Binary', op, left: expr, right };
    }
    return expr;
  }

  logicalAnd() {
    let expr = this.equality();
    while (this.match('AND_AND')) {
      const op = this.previous().value;
      const right = this.equality();
      expr = { type: 'Binary', op, left: expr, right };
    }
    return expr;
  }

  equality() {
    let expr = this.comparison();
    while (this.match('EQ_EQ', 'NOT_EQ')) {
      const op = this.previous().value;
      const right = this.comparison();
      expr = { type: 'Binary', op, left: expr, right };
    }
    return expr;
  }

  comparison() {
    let expr = this.term();
    while (this.match('LT', 'LT_EQ', 'GT', 'GT_EQ')) {
      const op = this.previous().value;
      const right = this.term();
      expr = { type: 'Binary', op, left: expr, right };
    }
    return expr;
  }

  term() {
    let expr = this.factor();
    while (this.match('PLUS', 'MINUS')) {
      const op = this.previous().value;
      const right = this.factor();
      expr = { type: 'Binary', op, left: expr, right };
    }
    return expr;
  }

  factor() {
    let expr = this.unary();
    while (this.match('STAR', 'SLASH', 'PERCENT')) {
      const op = this.previous().value;
      const right = this.unary();
      expr = { type: 'Binary', op, left: expr, right };
    }
    return expr;
  }

  unary() {
    if (this.match('BANG', 'MINUS')) {
      const op = this.previous().value;
      const right = this.unary();
      return { type: 'Unary', op, right };
    }
    return this.call();
  }

  call() {
    let expr = this.primary();
    while (true) {
      if (this.match('LPAREN')) {
        const args = [];
        if (!this.check('RPAREN')) {
          do {
            args.push(this.expression());
          } while (this.match('COMMA'));
        }
        this.consume('RPAREN', "Expected ')' after arguments");
        expr = { type: 'Call', callee: expr, args, line: expr.line || 1 };
      } else {
        break;
      }
    }
    return expr;
  }

  primary() {
    if (this.match('FALSE')) return { type: 'Literal', value: false, valType: 'boolean' };
    if (this.match('TRUE')) return { type: 'Literal', value: true, valType: 'boolean' };
    if (this.match('INT_LIT')) return { type: 'Literal', value: this.previous().value, valType: 'int' };
    if (this.match('DOUBLE_LIT')) return { type: 'Literal', value: this.previous().value, valType: 'double' };
    if (this.match('STRING')) return { type: 'StringLiteral', raw: this.previous().value };

    if (this.match('IDENT')) {
      return { type: 'Variable', name: this.previous().value, line: this.previous().line };
    }

    if (this.match('LPAREN')) {
      const expr = this.expression();
      this.consume('RPAREN', "Expected ')' after expression");
      return expr;
    }

    const tok = this.peek();
    this.diagnostics.push({
      line: tok.line,
      col: tok.col,
      message: `Unexpected token '${tok.value || tok.type}'`,
      severity: 'error',
    });
    this.advance();
    return { type: 'Literal', value: null, valType: 'void' };
  }
}

class EsttroInterpreter {
  constructor(outputCallback) {
    this.outputCallback = outputCallback;
    this.globals = new Map();
    this.functions = new Map();
    this.environment = this.globals;
    this.initBuiltins();
  }

  initBuiltins() {
    const builtins = [
      'print',
      'assert',
      'toInt',
      'toFloat',
      'toDouble',
      'toString',
      'toBoolean',
    ];
    for (const b of builtins) {
      this.globals.set(b, `<builtin ${b}>`);
    }
  }

  run(statements) {
    for (const stmt of statements) {
      if (stmt.type === 'FunctionDecl') {
        this.functions.set(stmt.name, stmt);
      }
    }

    for (const stmt of statements) {
      if (stmt.type !== 'FunctionDecl') {
        const res = this.executeStatement(stmt, this.globals);
        if (res && res.type === 'return') {
          break;
        }
      }
    }
  }

  executeStatement(stmt, env) {
    switch (stmt.type) {
      case 'ExprStmt':
        this.evaluate(stmt.expr, env);
        break;
      case 'LetDecl':
      case 'ConstDecl': {
        const val = this.evaluate(stmt.init, env);
        env.set(stmt.name, val);
        break;
      }
      case 'Block': {
        const blockEnv = new Map(env);
        for (const s of stmt.statements) {
          const res = this.executeStatement(s, blockEnv);
          if (res && (res.type === 'return' || res.type === 'break' || res.type === 'continue')) {
            return res;
          }
        }
        for (const [k, v] of blockEnv.entries()) {
          if (env.has(k)) env.set(k, v);
        }
        break;
      }
      case 'If': {
        const cond = this.evaluate(stmt.condition, env);
        if (cond) {
          const ifEnv = new Map(env);
          for (const s of stmt.thenBranch) {
            const res = this.executeStatement(s, ifEnv);
            if (res && (res.type === 'return' || res.type === 'break' || res.type === 'continue')) return res;
          }
          for (const [k, v] of ifEnv.entries()) if (env.has(k)) env.set(k, v);
        } else if (stmt.elseBranch) {
          const elseEnv = new Map(env);
          for (const s of stmt.elseBranch) {
            const res = this.executeStatement(s, elseEnv);
            if (res && (res.type === 'return' || res.type === 'break' || res.type === 'continue')) return res;
          }
          for (const [k, v] of elseEnv.entries()) if (env.has(k)) env.set(k, v);
        }
        break;
      }
      case 'While': {
        while (this.evaluate(stmt.condition, env)) {
          const loopEnv = new Map(env);
          let shouldBreak = false;
          for (const s of stmt.body) {
            const res = this.executeStatement(s, loopEnv);
            if (res) {
              if (res.type === 'break') { shouldBreak = true; break; }
              if (res.type === 'continue') break;
              if (res.type === 'return') return res;
            }
          }
          for (const [k, v] of loopEnv.entries()) if (env.has(k)) env.set(k, v);
          if (shouldBreak) break;
        }
        break;
      }
      case 'For': {
        const startVal = this.evaluate(stmt.start, env);
        const endVal = this.evaluate(stmt.end, env);
        for (let i = startVal; i <= endVal; i++) {
          const loopEnv = new Map(env);
          loopEnv.set(stmt.variable, i);
          let shouldBreak = false;
          for (const s of stmt.body) {
            const res = this.executeStatement(s, loopEnv);
            if (res) {
              if (res.type === 'break') { shouldBreak = true; break; }
              if (res.type === 'continue') break;
              if (res.type === 'return') return res;
            }
          }
          for (const [k, v] of loopEnv.entries()) {
            if (k !== stmt.variable && env.has(k)) env.set(k, v);
          }
          if (shouldBreak) break;
        }
        break;
      }
      case 'Return':
        return { type: 'return', value: stmt.value ? this.evaluate(stmt.value, env) : null };
      case 'Break':
        return { type: 'break' };
      case 'Continue':
        return { type: 'continue' };
    }
    return null;
  }

  evaluate(expr, env) {
    if (!expr) return null;
    switch (expr.type) {
      case 'Literal':
        return expr.value;
      case 'StringLiteral':
        return this.interpolateString(expr.raw, env);
      case 'Variable':
        if (env.has(expr.name)) return env.get(expr.name);
        if (this.globals.has(expr.name)) return this.globals.get(expr.name);
        if (this.functions.has(expr.name)) return `<fn ${expr.name}>`;
        throw new Error(`Undeclared identifier '${expr.name}'`);
      case 'Assignment': {
        const val = this.evaluate(expr.value, env);
        let curr = env.has(expr.name) ? env.get(expr.name) : this.globals.get(expr.name);
        let updated = val;
        if (expr.op === '+=') updated = curr + val;
        else if (expr.op === '-=') updated = curr - val;
        else if (expr.op === '*=') updated = curr * val;
        else if (expr.op === '/=') updated = Math.floor(curr / val);
        else if (expr.op === '%=') updated = curr % val;

        if (env.has(expr.name)) env.set(expr.name, updated);
        else this.globals.set(expr.name, updated);
        return updated;
      }
      case 'Binary': {
        const left = this.evaluate(expr.left, env);
        const right = this.evaluate(expr.right, env);
        switch (expr.op) {
          case '+': return left + right;
          case '-': return left - right;
          case '*': return left * right;
          case '/': return typeof left === 'number' && Number.isInteger(left) && Number.isInteger(right) ? Math.floor(left / right) : left / right;
          case '%': return left % right;
          case '==': return left === right;
          case '!=': return left !== right;
          case '<': return left < right;
          case '<=': return left <= right;
          case '>': return left > right;
          case '>=': return left >= right;
          case '&&': return left && right;
          case '||': return left || right;
        }
        break;
      }
      case 'Unary': {
        const val = this.evaluate(expr.right, env);
        if (expr.op === '-') return -val;
        if (expr.op === '!') return !val;
        break;
      }
      case 'Call': {
        const funcName = expr.callee.name;
        if (funcName === 'print') {
          const evaluatedArgs = expr.args.map((a) => this.evaluate(a, env));
          const text = evaluatedArgs.map((v) => (v === null || v === undefined ? '' : String(v))).join(' ') + '\n';
          this.outputCallback(text);
          return null;
        }
        if (funcName === 'assert') {
          const evaluatedArgs = expr.args.map((a) => this.evaluate(a, env));
          if (!evaluatedArgs[0]) {
            throw new Error(evaluatedArgs[1] || 'assertion failed');
          }
          return null;
        }
        if (funcName === 'toInt') {
          const v = this.evaluate(expr.args[0], env);
          return parseInt(v, 10);
        }
        if (funcName === 'toFloat' || funcName === 'toDouble') {
          const v = this.evaluate(expr.args[0], env);
          return parseFloat(v);
        }
        if (funcName === 'toString') {
          const v = this.evaluate(expr.args[0], env);
          return String(v);
        }
        if (funcName === 'toBoolean') {
          const v = this.evaluate(expr.args[0], env);
          return Boolean(v);
        }

        const fn = this.functions.get(funcName);
        if (!fn) throw new Error(`Function '${funcName}' is not defined`);

        const callEnv = new Map(this.globals);
        for (let i = 0; i < fn.params.length; i++) {
          const argVal = i < expr.args.length ? this.evaluate(expr.args[i], env) : null;
          callEnv.set(fn.params[i].name, argVal);
        }

        for (const s of fn.body) {
          const res = this.executeStatement(s, callEnv);
          if (res && res.type === 'return') return res.value;
        }
        return null;
      }
    }
    return null;
  }

  interpolateString(raw, env) {
    return raw.replace(/\{([^}]+)\}/g, (match, exprStr) => {
      const trimmed = exprStr.trim();
      try {
        const lexer = new EsttroLexer(trimmed);
        const tokens = lexer.tokenize();
        const parser = new EsttroParser(tokens, []);
        const expr = parser.expression();
        const val = this.evaluate(expr, env);
        return String(val);
      } catch (err) {
        return match;
      }
    });
  }
}

export function validateEsttro(source) {
  const lexer = new EsttroLexer(source);
  const tokens = lexer.tokenize();
  const diagnostics = [...lexer.diagnostics];
  const parser = new EsttroParser(tokens, diagnostics);
  parser.parse();
  return { diagnostics };
}

export function executeEsttro(source, onOutput) {
  const startTime = Date.now();
  let buffer = '';
  const out = (text) => {
    buffer += text;
    if (onOutput) onOutput(text);
  };

  try {
    const lexer = new EsttroLexer(source);
    const tokens = lexer.tokenize();
    const diagnostics = [...lexer.diagnostics];
    const parser = new EsttroParser(tokens, diagnostics);
    const ast = parser.parse();

    if (diagnostics.length > 0) {
      const durationMs = Date.now() - startTime;
      return {
        output: buffer,
        success: false,
        exitCode: 1,
        durationMs,
        diagnostics,
      };
    }

    const interp = new EsttroInterpreter(out);
    interp.run(ast);
    const durationMs = Date.now() - startTime;

    return {
      output: buffer,
      success: true,
      exitCode: 0,
      durationMs,
      diagnostics: [],
    };
  } catch (err) {
    const durationMs = Date.now() - startTime;
    out(`\nRuntime error: ${err.message}\n`);
    return {
      output: buffer,
      success: false,
      exitCode: 2,
      durationMs,
      diagnostics: [{ line: 1, col: 1, message: err.message, severity: 'error' }],
    };
  }
}

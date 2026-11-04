// A function declaration that is an `if` branch is in its own block scope (Annex B.3.3).

// A later lexical declaration prevents the Annex B var binding.
// The function's own `f` is the function.
function laterLet() {
  if (true) function f() {
    return f;
  }
  let f = 1;
  return f;
}

// An earlier lexical declaration prevents it too, without sharing a symbol with the function.
function earlierLet() {
  let f = 1;
  if (true) function f() {}
  return f;
}

// So does a lexical declaration in the enclosing block. `return f` is a global reference.
function enclosingBlockLet() {
  {
    let f;
    if (true) function f() {}
  }
  return f;
}

// So does a parameter. The function's own parameter defaults still see the function.
function parameter(f) {
  if (true) function f(a = f) {}
  return f;
}

// Each branch has its own scope. Both write the same var binding.
function bothBranches(c) {
  if (c) function f() {} else function f() {}
  return f;
}

// The alternate cannot see the consequent's function.
function crossBranch(c) {
  if (c) function f() {} else function g() {
    return f;
  }
  let f;
}

// A function in the branch function's body writes a var binding in that body.
function nested() {
  if (true) function f() {
    if (true) function f() {}
    return f;
  }
  let f;
}

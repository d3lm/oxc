// TypeScript has no Annex B. A function declaration that is an `if` branch is bound in the
// scope enclosing the `if` statement, not in the branch's own scope.
if (x) function f() {}
f();

function g(c: boolean) {
  if (c) function h() {} else function h() {}
  return h;
}

function f(make, a) {
  using resource = make();
  if (a) function g() {}
  let g = 1;
  return g;
}

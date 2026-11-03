function f(make, a) {
  try {
    var _usingCtx = babelHelpers.usingCtx();
    const resource = _usingCtx.u(make());
    if (a) function g() {}
    let g = 1;
    return g;
  } catch (_) {
    _usingCtx.e = _;
  } finally {
    _usingCtx.d();
  }
}

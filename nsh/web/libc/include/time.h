#ifndef NSVIM_WASM_TIME_H_
#define NSVIM_WASM_TIME_H_

// The web build's C library has no clock: tree-sitter's parse timeouts stay off.
typedef long long clock_t;

#define CLOCKS_PER_SEC 1000000

clock_t clock(void);

#endif // NSVIM_WASM_TIME_H_

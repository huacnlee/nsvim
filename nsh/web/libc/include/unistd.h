#ifndef NSVIM_WASM_UNISTD_H_
#define NSVIM_WASM_UNISTD_H_

// The web has no files: dup always fails.
int dup(int fd);

#endif // NSVIM_WASM_UNISTD_H_

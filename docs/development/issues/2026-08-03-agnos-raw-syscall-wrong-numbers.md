# Four raw `syscall(N, …)` sites use LINUX numbers and call something else entirely on agnos

**Opened** 2026-08-03 · **Status** OPEN, confirmed against kernel source · **Severity** ships today in `build/jalwa-agnos`

## The class

agnos redefines the syscall enum. A raw `syscall(N, …)` with a Linux number **compiles clean, passes
arity checking, and calls a completely different kernel arm.** No warning, no error. Every site below
was verified against the arm in `agnos/kernel/core/syscall.cyr`.

| # | site | intended | agnos arm actually hit | effect |
|---|---|---|---|---|
| **J1** | `src/main.cyr:60`, `:63` — `syscall(83, pre, 493)` | `mkdir 0755` | `syscall.cyr:6963` — `gpu_dispatch_f64_sys(arg1,arg2,arg3)` | **mkdir → GPU f64 matmul dispatch.** Return ignored, so the library dir is silently never created. Reached by the **default agnos desktop entry** (`src/main.cyr:191-202`, no-args → `jlw_open_library()` → `jlw_mkdir_p`) |
| **J2** | `src/gui/present.cyr:34-38`, called every frame at `:109` with `timeout_ms = -1` | `poll` | `syscall.cyr:6445` — `open(name,namelen,flags)` | **poll → open, per frame** on the agnos GUI path. agnos has no poll. `arg3 = -1` sets every flag bit including `AO_CREAT`/`AO_TRUNC`/`AO_DIRECTORY` |
| **J3** | `src/mcp_serve.cyr:277` — `syscall(0, 0, buf + i, 1)` | `read(0,…)` | `syscall.cyr:6305` — **`exit`** | **`jalwa mcp` terminates before emitting a byte.** Zero `#ifdef` in the file; included unconditionally at `src/main.cyr:31` |
| **J4** | `src/ui/tui.cyr:238-242`, callers `:266`, `:273`, `:386` | `poll` | `syscall.cyr:6445` — `open` | same as J2, on the `jalwa tui` path |

## Fixes

- **J1** → `sys_mkdir(path, pathlen)` = **#9** (`syscall.cyr:6494`). ⚠ agnos's mkdir is
  **length-carrying and takes no mode** — changing `83`→`9` alone is NOT the fix; the call shape must
  change too.
- **J3** → `sys_read` = **#5**.
- **J2 / J4** → agnos has no poll syscall. Use the non-blocking + `sched_yield #44` pattern the rest of
  the stack uses, guarded `#ifdef CYRIUS_TARGET_AGNOS`, keeping the Linux `poll` arm for the Linux build.

**General rule:** never use a raw `syscall(` with a literal number on an agnos-reachable path. Use the
`sys_*` wrapper (cyrius 6.5.1 makes wrapper arity a hard error, which is the only automatic protection
that exists here) or guard the raw call with `#ifdef CYRIUS_TARGET_AGNOS`.

## Related, different repo — the same class, one call from live

`agnostik`'s `_fill_random` (seen as `aethersafha/lib/agnostik.cyr:135-159`, so the fix belongs in the
**agnostik** repo and propagates by tag bump — never edit the materialized `lib/` copy):

- `syscall(318, …)` — no agnos arm → -1
- `syscall(2, "/dev/urandom", 0, 0)` → `syscall.cyr:6350` is **getpid**, returns a *positive* value, so
  the `if (fd >= 0)` guard at `:139` **passes**
- `syscall(0, fd, buf+off, n-off)` → `syscall.cyr:6305` is **`exit`** — the process dies here
- the "fail loudly" `syscall(60, 70)` at `:159` → `syscall.cyr:8054` is **`winsize()`**, a no-op

Every agnostik identifier (`agent_id_new` / `trace_id_new` / `session_id_new` / `task_id_new`) funnels
into it. Unreachable from the compositor today; any agnos consumer that generates an identifier dies
silently.

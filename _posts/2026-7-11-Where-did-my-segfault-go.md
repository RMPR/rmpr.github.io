---
layout: post
title: Where did my segfault go?
---

The other day I was iterating on a small C program with [entr](https://eradman.com/entrproject/):

```sh
ls hello.c | entr -s "gcc -o hello hello.c && ./hello"
```

`hello` happened to segfault, but entr showed me... nothing. No `Segmentation
fault`, no non-zero exit visible, just complete silence after the compile
output. Hmm, very weird. Let's prefix with `bash -c`:

```sh
ls hello.c | entr -s "bash -c 'gcc -o hello hello.c && ./hello'"
```

Still nothing. What about moving the command into a script:

```sh
#!/bin/bash
gcc -o hello hello.c && ./hello
```

And point entr to it:

```sh
ls hello.c | entr -s ./run.sh

./run.sh: line 2: 104465 Segmentation fault         (core dumped) ./hello
```

There's my segmentation fault!

# The reason(s)

> Thanks to [superuser](https://superuser.com/a/415954) (remember that site we
> used to use pre LLMs?) for pointing me in the right direction. The question is
> not exactly the same, but it still helped me. At first, I thought the problem
> was with `entr` because I could reproduce both with bash and fish.

**Who actually prints "Segmentation fault"?** Obviously not the crashing
program. It can't since it's dead. The message is printed by the *shell* when it
reaps a child that died from `SIGSEGV`. No parent shell, no message, simple
enough.

But wait, why does even the explicit `bash -c 'gcc && ./hello'` version stay silent?

Turns out bash likes to `exec` the command if it can. When you run `bash -c
"some_command"` and `some_command` is effectively the only thing to do, [bash
doesn't bother forking a new process for
it](https://cgit.git.savannah.gnu.org/cgit/bash.git/tree/execute_cmd.c?id=ecb2456d6357bfd3b2965aafe2f2021d1ced5b72#n5827).
It just `execve`s into it, replacing itself. It's a nice optimization, invisible
99% of the time.

In the script version, entr runs `./run.sh` as a child, and the script's
shebang starts a fresh `bash` to interpret the file. That bash runs `gcc`,
forks for `./hello`, waits, sees it died from `SIGSEGV`, and dutifully prints
`Segmentation fault` before exiting.

# The follow-up

But then it occurred to me, maybe you don't need a wrapper script after all, what
happens if I have the crashing command running in a subshell?

```sh
$ ls hello.c | entr -s "gcc -o hello hello.c && (./hello)"
hello.c: line 1: 106595 Segmentation fault         (core dumped) ( ./hello )
```

Yep, I don't think I have ever been that satisfied to see a program segfaulting.
Here the parens force `./hello` into a forked subshell, so bash can't `exec` its
way out of being the parent. When the subshell dies from `SIGSEGV`, bash reaps
it and prints the message like normal.

I could have stopped there but I wanted a way to test my second hypothesis. What
if you just make sure the shell has something to do after the crashing command?

```sh
$ ls hello.c | entr -s "bash -c 'gcc -o hello hello.c && ./hello; true'"
bash: line 1: 109516 Segmentation fault         (core dumped) ./hello
```

Victory!

Now I should probably get back to writing some C...

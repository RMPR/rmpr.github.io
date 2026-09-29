---
layout: post
title: 30 days writing C++ in Zed
---

# Pre start

I adopted Vscode default shortcuts which binds file search to Ctrl-P. I think I
like this flow better. Of course Vim mode. But I needed to make this change to
keep my sanity:

```shell
{
  "context": "Editor && vim_mode == insert",
  "bindings": {
    "alt-j": ["action::Sequence", ["vim::NormalBefore", "vim::Down"]],
    "alt-h": ["action::Sequence", ["vim::NormalBefore", "vim::Left"]],
    "alt-k": ["action::Sequence", ["vim::NormalBefore", "vim::Up"]],
    "alt-l": ["action::Sequence", ["vim::NormalBefore", "vim::Right"]],
  },
}
```

# Cmake integration

NeoCmake is good as a LSP but I had to configure some convenience CMake tasks
and tell clangd where to find `compile_commands.json`, it ended up looking like:


```shell
{
    "label": "CMake: set preset",
    "command": "...",
    "cwd": "$ZED_WORKTREE_ROOT",
    "reveal": "always",
},
{
    "label": "CMake: configure",
    "command": "...",
    "cwd": "$ZED_WORKTREE_ROOT",
    "reveal": "always"
},
{
    "label": "CMake: build",
    "command": "...",
    "reveal": "always",
    "allow_concurrent_runs": true
},
{
    "label": "CMake: build target",
    "command": "...",
    "cwd": "$ZED_WORKTREE_ROOT",
    "reveal": "always"
},
{
    "label": "CMake: clean",
    "command": "...",
    "cwd": "$ZED_WORKTREE_ROOT",
    "reveal": "always"
},
{
    "label": "CMake: wipe build dir",
    "command": "...",
    "cwd": "$ZED_WORKTREE_ROOT",
    "reveal": "always",
}
```

I recommend having a convenience wrapper around CMake that will at the very least persist the selected preset.

# Git diffs view

I like it! I love it! I noticed it was not as performant as the other things,
but split diffs for the win. I still think that Clion is the better product when
it comes to this but since I started using jujutsu it doesn't really matter.

# In the end

I think I will keep using it more. 1 month is not enough to be very comfortable
with a tool to the point of making a definitive judgment.

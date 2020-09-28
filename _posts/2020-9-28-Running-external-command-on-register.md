---
layout: post
title: Running an external command on the content of a register in Vim
---

While working on a [side project](https://github.com/rmpr/atbswp) I faced a really annoying problem, run a
snippet of code to figure out what it does, or just to tinker a bit, without using it whatsoever, and while I don't think there is a
straightforward way to do this, except [using System](https://vi.stackexchange.com/questions/21284/running-external-command-on-contents-of-register)
I share here a simple trick. 

Rely on your temporary folder, after all that's what the Unix philosophy is about: 
composing tools to achieve what you want... So you start by creating a file in /tmp, I call it /tmp/temp.py
(add the extension because I defined some stuff that load when a Python file is detected see my
[dotfiles](https://github.com/rmpr/dotfiles))
```
:e /tmp/temp.py
```
Paste the content of the register
```
"[register]p
```
Save and execute the command
```
:w | !python %
```
And that's it. the good thing about this is, if you disable swap files, you can always have an history of
what snippet you tested and in the particular case of Python, you can even create a virtual environment, but 
I'll save this one for a later post.



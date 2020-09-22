---
layout: post
title: Yet another use of a Raspberry with an old screen
---
I have an old ultra HD non-smart TV, here's just a tip to make it to good use with
a Raspberry.

TL;DR mpv is dope

What I wanted to achieve is an on-demand TV, where I can play whatever I want, 
without having to mess with VNC, RDP and stuff (which can increase the attack 
surface of the rapsberry used as a server even further). I start with Raspbian Lite, (yeah I find the default 
GUI bloated) then proceed to install `xserver-xorg` and
`i3`, and, it's pretty much it. 
If you don't want to start i3 manually from the Terminal each 
time with `startx` , you can use a session manager. 
I use lightdm, but for hardcore minimalists, 
there's [SLiM](https://wiki.archlinux.org/index.php/SLiM)

Now all you have to do is `mpv <video_address>`.
To prevent the session to be terminated if there something wrong with the 
network, one solution is to use `tmux`, and by adding:

```bash
if command -v tmux &> /dev/null && [ -z "$TMUX" ]; then
    tmux attach -t default || tmux new -s default
fi
```

To your .bashrc, or the config file of whatever shell you use compatible with 
this syntax, you're all set.

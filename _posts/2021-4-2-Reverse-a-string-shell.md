---
layout: post
title: One-liner to reverse a String on a word basis
description: One-liner to reverse a String on a word basis
---

Browsing Lobsters, I stumbled upon [a thread](https://lobste.rs/s/fnpidl/hiring_juniors#c_sx5nfi) someone
talking about reversing a string on a word basis using C while in retrospect they could just have used `awk`. 
I decided to give it a shot but make it more challenging by using a one-liner. Once again I am amazed by the 
power of the coreutils, it almost feels like cheating:

```
tr " " "\n" | awk '/^\S+$/' |  rev | tr "\n" " "
```



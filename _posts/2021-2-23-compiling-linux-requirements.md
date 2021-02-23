---
layout: post
title: One-liner to test the requirements for building the Linux Kernel
---

I applied to a program at the Linux Foundation, and even though it didn't go exactly well (I was too
slow to complete a challenge) I learned a lot of things. But I found it tiresome to check manually if 
each program in Documentation/Changes is installed, so I came up with this one-liner.

```head -n 58 Changes | tail -n 27 | awk  '{print " "$(NF-1)" " " "$NF""}' | paste -sd\; | sh```

# head -n 58 Documentation/Changes | tail -n 27

Get the lines 32 to 58 in the Documentation/Changes file.

# awk  '{print " "$(NF-1)" " " "$NF""}'

Get the last column (either -V or --version) and the column before that (the program name).

# paste -sd\;

Format a bit separating them with semi colons.

# sh

Send everything to the shell, and just watch the output to know what you need to install.

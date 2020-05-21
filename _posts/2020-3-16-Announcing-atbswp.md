---
layout: post
title: Announcing atbswp, a minimalist macro recorder
---

[atbswp](https://github.com/rmpr/atbswp): Automate The Boring Stuff With Python (yes like the book) is a multi platform, open source clone of tinytask, and for those who don't know it, 
the core feature of the software is to allow the users to record their mouse and keyboard actions and reproduce them identically as many times as they want.

[![atbswp quick demo](https://img.youtube.com/vi/L0jjSgX5FYk/0.jpg)](https://www.youtube.com/watch?v=L0jjSgX5FYk)

# atbswp
The rest of this post discusses what atbswp is, why it was built, and miscellaneous things related to the project.

# About atbswp
atbswp is the result of frustration with tinytask which is not available natively on Unix like systems. With tinytask being discontinued now, the position shifted from multiplatform, 
FOSS clone, to only alternative. The most important thing about atbswp is avoiding feature creep, the software is pretty simple as it and we want to keep it that way.

# Initial Features
Many features of tinytask are already present in the current pre-alpha version of atbswp, the one that have not been yet replicated are compile to an executable for the host platform 
and set a replay speed, which are on the roadmap and will be the main focus for the next development. Behind the scenes, atbswp uses a Python file so the power users can always modify
it to their taste.

# Libraries used
- [pyautogui](https://github.com/asweigart/pyautogui) : Used for the replay feature
- [pynput](https://github.com/moses-palmer/pynput): to record user input
PyInstaller: to bundles the application into a single package
poetry: to publish on PyPI
wxPython: for the GUI


---
layout: post
title: Synchronous Async
description: How it became possible to stop infinite playback in atbswp
---
This piece might need to dive a bit in the source code of [atbswp](https://git.sr.ht/~rmpr/atbswp).
I was working on atbswp trying to solve a problem I spotted since the beginning of the project: How to
stop the infinite playback. It doesn't seem like it, but it was very challenging for me.
I even asked it [on stackoverflow](https://stackoverflow.com/questions/60296543/how-to-update-the-state-of-a-toggle-button-after-process-completion)

## The problem
Usually threading (and multiprocessing for that matter) are used for concurrent 
execution. That is, you leave the right to the interpreter/OS to execute 
the tasks in whatever order it find suitable, as long as everything executes 
concurrently. For my use case, I had to keep track of the execution of a thread in the background to launch another 
one (or not) just after it finishes executing.  My problem was thus to try to bend the library 
to my will for a thing it wasn't really designed to do in the first place.


## How did it work all this time?
Since I couldn't really keep track of what was happening, I decided to join the 
threads upon completion. That is, wait for the infinite playback to complete 
before giving back the control to the main event loop. On Linux, for some reason 
everything worked fine, but on Windows... the program will just become unresponsive 
until the user finally kills it or the infinite playback finishes. Which in 
retrospective was the right thing to do from the OS. As any guy working for free 
on a project alone, the solution was simple: ditch Windows support like I did with 
macOS just before that. That's when I realized most of the users (reporting issues) 
are on Windows. That wasn't really feasible then.

## The solution

### Realisation: Why are you trying to solve the same problem 3 different ways?
I came to the conclusion (in my sleep mind you) that the infinite playback, 
the finite playback and the single playback were just 3 instances of the same 
problem (before everything was implemented kinda independently). And just like the 
linked list implementation Torvalds always presents as good taste in code, 
I was able to produce something I found very clever. Yeah I know about the Kernighan's 
law, but I know I will never have to debug this code.

### First thing first split the exec
This is probably what unlocked everything. How to interact with the script 
in exec? Turns out since the macro is (hopefully) always linear, I can just 
execute the thing line by line instead of as a big string and it works!

### Second notify the main event loop that the work is over
For this part, I simply used the built-in mechanism of wx: events. When a thread 
finishes executing, it manually fires an event which will then be caught by the 
event loop which is in charge of everything, keeping track of a counter holding 
the number of executions left to satisfy the current settings it can then decide 
to launch another thread (if the counter is non-zero) or not.

### Finally extend the base Thread class to add a completion flag
The only kinda reliable way to interrupt a thread is to return from inside, I just 
added a flag that the thread will check periodically (before executing the 
current line of the capture) and return if it's set.

## Final thoughts
In retrospective, it seems that I rolled out my own mini-scheduler (kind of) and 
experimenting with async code was challenging because the debugger wasn't as 
useful as usual. That made me rethink about the whole fearless concurrency 
thing of Rust. That might be a thing.
 

---
layout: post
title: Use atbswp's infinite playback effectively - case of a Youtube video
---
If you use atbswp (or any macro recorder) for a punctual task, this guide is not for you. My target are 
repetitive tasks that you can make atbswp perform infinitely (watching repeatedly a Youtube video for example).
This is just a set of good practices I was able to identify while using atbswp for various tasks.

TL;DR Find an entry point and a exit point. The latter must reset the task after each iteration. 

# Show me the code
You can view the end result directly:

<iframe width="560" height="315" src="https://www.youtube-nocookie.com/embed/Gj83D2t-W9g" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>

I'll just expand a bit on the TL;DR with illustrations from what we are trying to achieve: watch repeatedly a 
Youtube video.

## First the entry point
I chose a browser I don't really use/rely on and start it without any other tab open, that's it. In a word, as
much as possible start from a clean environment.

## Actually performing the task
Now you can whatever you want. In this case pasting the url (that I previously put in the clipboard) and going 
to the videos directly.

## The exit point
This is for me the most important part. What happens if there's a failure during the process? For example the 
Internet connection hanging, the browser launching a popup window? ...? You cannot really predict what can go 
wrong, but by exiting gracefully you can solve a great deal of them. And here it's actually closing all the tabs. 
In a word: restore the clean environment you found in the first place.

# Additional advice

- Use keyboard shortcuts as much as possible (I used the mouse in the demo just for the sake of clarity)
- Use virtual machines
- It's useful to insert some pauses during your recording
- Post your issues [here](https://github.com/rmpr/issues) instead of as a comment [here](https://www.youtube.com/watch?v=L0jjSgX5FYk) 

If you think I missed something feel free to comment below, additionally you can join the 
[Telegram community](https://t.me/atbswp).

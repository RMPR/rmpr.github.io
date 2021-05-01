---
layout: post
title: Display Driver on RedHat-based Linux distributions (tested on Asus MB169B+)
---
I got myself a Asus MB169B+ to use with my Thinkpad X1 Yoga, and what leads me to write this is because when
you try to install the displaylink driver you are instantly redirected to Ubuntu's, even very active members
in the community [suggest you to do so](https://www.winglemeyer.org/ramblings/2018/11/19/displaylink.html).
Well, let me tell you something: you don't need to, [display-rpm](https://github.com/displaylink-rpm/displaylink-rpm) is what you have been
looking for. And the thing is actually up to date: I am currently running with the 5.11.16 kernel on
Fedora 33. Granted [Fedora 34 is already out](https://fedoramagazine.org/announcing-fedora-34/) but 
it still got you covered, you can build it yourself. with everything that bleeding-edginess implies. 
The only downside is that it only works on Xorg for now.
One more reason to [stay away from Sway for now](../Frankentile/).

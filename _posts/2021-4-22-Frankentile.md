---
layout: post
title: Frankentile (KDE + i3)
---
I used i3 for 2 years (2017-2019), then Sway to get a taste of Wayland 
for almost the same amount of time. Although I was quite satisfied with my 
setup, I discovered something even better for me: `KDE + i3`. Or more exactly 
`KDE - KWin + i3`.

For some reasons, I had to go back to Xorg in order to get screen sharing working 
with Teams (thanks M$).
I must admit, after 4 years away from DEs, Gnome became even less usable
for me on Xorg (on Wayland it seems fine though). I don't know why the
Gnome Shell kept crashing. That lead me to use KDE (which I was kind of already 
using in a virtual machine to test atbswp) and I must say, it's been a great experience 
(although it feels sluggish compared to a WM).
I especially like the start menu. It's like Windows' but without all the 
bullshit. By the way, somebody tells Microsoft that nobody wants neither candy crush 
auto-installing, nor royal Revolt - what is that even -, nor ads all over the place, 
nor Cortana, etc. I don't know why, but I missed the tiling, and it's not like 
I knew tiling all along. I discovered it 4 years ago and fell in love, 
or maybe I am just becoming old and I don't want things to change anymore. 

Enters i3. I discovered that it's not only possible to integrate i3 with KDE, but 
it's also done with a minimal effort: just write a two lines script and create a 
symlink in the startup folder of Plasma. The good news is, I don't even have to 
give up on Sway, I launch it on a different VT (Ctrl+Alt+F3 usually) and I can 
switch environments with minimal or no hassle at all. This time I got to read 
the whole i3 documentation, there are some hidden gems in there 
like making a window sticky (how did I even manage to live without that feature?).
My config file is [here](https://github.com/rmpr/dotfiles). And if you are 
interested in the guides I used, [here](https://userbase.kde.org/Tutorials/Using_Other_Window_Managers_with_Plasma#Introduction) 
[you](https://github.com/heckelson/i3-and-kde-plasma) [go](https://laptrinhx.com/i3-plasma-using-the-i3-window-manager-on-the-top-of-kde-plasma-and-other-dotfiles-configurations-scripts-workarounds-and-practises-from-my-debian-machines-236106475/#desktop).
I feel like I should mention this [non-hacky awesome guide](https://cravencode.com/post/essentials/enable-tap-to-click-in-i3wm/) 
to make the mouse gestures work. While going through all this, I stumbled upon 
[krohnkite](https://github.com/esjeon/krohnkite). It seemed interesting but I never 
got to actually test it. I guess it might be useful to DWM users.


---
layout: post
title: Internationalization in Org-mode with Doom Emacs
---
Not so long ago, I discovered [digraphs](https://vim.fandom.com/wiki/Entering_special_characters) 
and since I use [aerc](https://aerc-mail.org/) as my email client, I immediately 
found it to be a blessing, because I write in many languages on a daily basis 
(mainly French, English and Spanish), and changing keyboard layouts is very painful 
for the brain, and everybody knows that the QWERTY layout is superior for Vim, even 
[Theprimeagen](https://youtu.be/tCktGgPQ3D0?t=1435) (yes this was an actual talk 
at the Vimconf 2020) says so, but I absolutedly hate the international US Layout.

That said, I obviously directly wanted to include 
this in my org-mode workflow with Doom Emacs, don't ask me why I use both (Neo)vim 
and (Doom)emacs, it turns out that choosing an editor is a more complex task than 
what it seems in the first place... but I digress. What a surprise when I tried  
`<C-K>` and it jumped to the parent tree instead of showing the "right" behaviour, a quick look at the 
[evil's symbols definition](https://github.com/emacs-evil/evil/blob/3f3b2fea87172f155e5f91d75f0fb69d8648acf2/evil-maps.el#L369) 
shows that it's obviously not the culprit, moreover the feature seems to work 
everywhere except in org-mode... Hmmmh doom-emacs what are you up to? Seems like 
the "problem" is [here](https://github.com/hlissner/doom-emacs/blob/848cc117c4555c424fcd7f16d363b5da8118f36d/modules/lang/org/config.el#L981). 
I'm not remotely familiar with Elisp, but it looks like a remaping of 
the keyboard shortcut of interest in org-mode *sigh*.

Anyway, Emacs has a built-in way to insert digraphs, while it's way less efficient 
than what I am accustomed to, you have to type `C-x 8` then the combination of symbols 
to insert your accented character ('e = é for example). If you want to learn more 
have a look at this article about 
[Diacritics in Emacs](https://masteringemacs.org/article/diacritics-in-emacs).

